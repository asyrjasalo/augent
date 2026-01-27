#!/usr/bin/env node

const fs = require("fs");
const path = require("path");
const https = require("https");
const crypto = require("crypto");
const { execSync } = require("child_process");

const VERSION = require("../package.json").version;
const BIN_DIR = path.join(__dirname, "..", "bin");
const BIN_PATH = path.join(
  BIN_DIR,
  process.platform === "win32" ? "augent.exe" : "augent",
);

// Map Node.js platform/arch to Rust target triple
function getTarget() {
  const platform = process.platform;
  const arch = process.arch;

  // Normalize architecture names
  // Node.js reports 'arm64' on both Linux and macOS for ARM64
  // Some systems might report 'aarch64' or other variants
  const normalizedArch =
    arch === "arm64" || arch === "aarch64" ? "arm64" : arch;

  if (platform === "linux") {
    if (normalizedArch === "arm64") {
      return "aarch64-unknown-linux-gnu";
    } else if (normalizedArch === "x64" || normalizedArch === "x86_64") {
      return "x86_64-unknown-linux-gnu";
    }
    throw new Error(`Unsupported Linux architecture: ${arch}`);
  } else if (platform === "darwin") {
    if (normalizedArch === "arm64") {
      return "aarch64-apple-darwin";
    } else if (normalizedArch === "x64" || normalizedArch === "x86_64") {
      return "x86_64-apple-darwin";
    }
    throw new Error(`Unsupported macOS architecture: ${arch}`);
  } else if (platform === "win32") {
    if (normalizedArch === "arm64") {
      return "aarch64-pc-windows-msvc";
    } else if (normalizedArch === "x64" || normalizedArch === "x86_64") {
      return "x86_64-pc-windows-msvc";
    }
    throw new Error(`Unsupported Windows architecture: ${arch}`);
  }

  throw new Error(`Unsupported platform: ${platform} ${arch}`);
}

function getArchiveExtension() {
  return process.platform === "win32" ? "zip" : "tar.gz";
}

function downloadFile(url, dest) {
  return new Promise((resolve, reject) => {
    const file = fs.createWriteStream(dest);
    https
      .get(url, (response) => {
        if (response.statusCode === 302 || response.statusCode === 301) {
          // Follow redirect
          return downloadFile(response.headers.location, dest)
            .then(resolve)
            .catch(reject);
        }
        if (response.statusCode !== 200) {
          reject(
            new Error(
              `Failed to download: ${response.statusCode} ${response.statusMessage}`,
            ),
          );
          return;
        }
        response.pipe(file);
        file.on("finish", () => {
          file.close();
          resolve();
        });
      })
      .on("error", (err) => {
        fs.unlink(dest, () => {});
        reject(err);
      });
  });
}

function downloadText(url) {
  return new Promise((resolve, reject) => {
    https
      .get(url, (response) => {
        if (response.statusCode === 302 || response.statusCode === 301) {
          return downloadText(response.headers.location)
            .then(resolve)
            .catch(reject);
        }
        if (response.statusCode !== 200) {
          reject(
            new Error(
              `Failed to download: ${response.statusCode} ${response.statusMessage}`,
            ),
          );
          return;
        }
        let data = "";
        response.on("data", (chunk) => {
          data += chunk;
        });
        response.on("end", () => resolve(data));
      })
      .on("error", reject);
  });
}

function calculateSHA256(filePath) {
  const fileBuffer = fs.readFileSync(filePath);
  const hashSum = crypto.createHash("sha256");
  hashSum.update(fileBuffer);
  return hashSum.digest("hex");
}

function verifyChecksum(archivePath, expectedChecksum) {
  console.log("Verifying checksum...");
  const actualChecksum = calculateSHA256(archivePath);
  if (actualChecksum !== expectedChecksum) {
    throw new Error(
      `Checksum verification failed!\n` +
        `Expected: ${expectedChecksum}\n` +
        `Actual:   ${actualChecksum}`,
    );
  }
  console.log("✓ Checksum verified");
}

function extractArchive(archivePath, target) {
  const ext = getArchiveExtension();
  const binName = process.platform === "win32" ? "augent.exe" : "augent";

  try {
    if (ext === "tar.gz") {
      // Extract tar.gz
      execSync(`tar -xzf "${archivePath}" -C "${BIN_DIR}"`, {
        stdio: "inherit",
      });
    } else {
      // Extract zip (Windows) using PowerShell
      const archivePathEscaped = archivePath.replace(/'/g, "''"); // Escape single quotes for PowerShell
      const binDirEscaped = BIN_DIR.replace(/'/g, "''");
      execSync(
        `powershell -Command "Expand-Archive -Path '${archivePathEscaped}' -DestinationPath '${binDirEscaped}' -Force"`,
        { stdio: "inherit" },
      );
    }

    // Move binary from extracted directory to bin directory
    const extractedDir = path.join(BIN_DIR, `augent-v${VERSION}-${target}`);
    const extractedBin = path.join(extractedDir, binName);

    if (fs.existsSync(extractedBin)) {
      fs.renameSync(extractedBin, BIN_PATH);
      fs.rmSync(extractedDir, { recursive: true, force: true });
    } else {
      throw new Error(`Binary not found in extracted archive: ${extractedBin}`);
    }
  } catch (error) {
    throw new Error(`Failed to extract archive: ${error.message}`);
  }
}

async function main() {
  // Skip if binary already exists
  if (fs.existsSync(BIN_PATH)) {
    console.log("augent binary already exists, skipping download");
    return;
  }

  const target = getTarget();
  const ext = getArchiveExtension();
  const archiveName = `augent-v${VERSION}-${target}.${ext}`;
  const archiveUrl = `https://github.com/asyrjasalo/augent/releases/download/v${VERSION}/${archiveName}`;
  const checksumsUrl = `https://github.com/asyrjasalo/augent/releases/download/v${VERSION}/checksums.txt`;
  const archivePath = path.join(BIN_DIR, archiveName);
  const checksumsPath = path.join(BIN_DIR, "checksums.txt");

  console.log(`Downloading augent ${VERSION} for ${target}...`);
  console.log(`Detected: platform=${process.platform}, arch=${process.arch}`);
  console.log(`Target triple: ${target}`);

  try {
    // Ensure bin directory exists
    if (!fs.existsSync(BIN_DIR)) {
      fs.mkdirSync(BIN_DIR, { recursive: true });
    }

    // Download checksums file
    console.log("Downloading checksums...");
    const checksumsContent = await downloadText(checksumsUrl);
    fs.writeFileSync(checksumsPath, checksumsContent);

    // Parse checksums file to find expected checksum for our archive
    const checksumLine = checksumsContent
      .split("\n")
      .find((line) => line.includes(archiveName));

    if (!checksumLine) {
      throw new Error(`Checksum not found for ${archiveName} in checksums.txt`);
    }

    // Extract checksum (format: "checksum  filename" or "checksum *filename")
    const expectedChecksum = checksumLine.trim().split(/\s+/)[0];
    if (!expectedChecksum || expectedChecksum.length !== 64) {
      throw new Error(
        `Invalid checksum format in checksums.txt: ${checksumLine}`,
      );
    }

    // Download archive
    await downloadFile(archiveUrl, archivePath);

    // Verify checksum before extracting
    verifyChecksum(archivePath, expectedChecksum);

    // Extract archive
    await extractArchive(archivePath, target);

    // Clean up archive and checksums
    fs.unlinkSync(archivePath);
    fs.unlinkSync(checksumsPath);

    // Make binary executable (Unix)
    if (process.platform !== "win32") {
      fs.chmodSync(BIN_PATH, 0o755);
    }

    console.log("augent installed successfully");
  } catch (error) {
    console.error(`Failed to install augent: ${error.message}`);
    console.error(`\nYou can manually download from: ${archiveUrl}`);
    // Clean up on error
    if (fs.existsSync(archivePath)) fs.unlinkSync(archivePath);
    if (fs.existsSync(checksumsPath)) fs.unlinkSync(checksumsPath);
    process.exit(1);
  }
}

main().catch((error) => {
  console.error(error);
  process.exit(1);
});
