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

function downloadFile(url, dest, redirectCount = 0) {
  return new Promise((resolve, reject) => {
    // Prevent infinite redirect loops
    if (redirectCount > 10) {
      reject(new Error("Too many redirects"));
      return;
    }

    // Handle relative redirect URLs
    const urlObj = new URL(url);
    const options = {
      hostname: urlObj.hostname,
      port: urlObj.port || 443,
      path: urlObj.pathname + urlObj.search,
      method: "GET",
      timeout: 30000, // 30 second timeout
    };

    const file = fs.createWriteStream(dest);
    const request = https
      .get(options, (response) => {
        if (response.statusCode === 302 || response.statusCode === 301) {
          // Follow redirect
          request.destroy();
          file.destroy();
          fs.unlink(dest, () => {});
          const redirectUrl = response.headers.location;
          // Handle relative redirects
          const absoluteUrl = redirectUrl.startsWith("http")
            ? redirectUrl
            : `${urlObj.protocol}//${urlObj.host}${redirectUrl}`;
          return downloadFile(absoluteUrl, dest, redirectCount + 1)
            .then(resolve)
            .catch(reject);
        }
        if (response.statusCode !== 200) {
          request.destroy();
          file.destroy();
          fs.unlink(dest, () => {});
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
          response.destroy();
          request.destroy();
          resolve();
        });
      })
      .on("error", (err) => {
        request.destroy();
        file.destroy();
        fs.unlink(dest, () => {});
        reject(err);
      })
      .on("timeout", () => {
        request.destroy();
        file.destroy();
        fs.unlink(dest, () => {});
        reject(new Error("Request timeout"));
      });
  });
}

function downloadText(url, redirectCount = 0) {
  return new Promise((resolve, reject) => {
    // Prevent infinite redirect loops
    if (redirectCount > 10) {
      reject(new Error("Too many redirects"));
      return;
    }

    // Handle relative redirect URLs
    const urlObj = new URL(url);
    const options = {
      hostname: urlObj.hostname,
      port: urlObj.port || 443,
      path: urlObj.pathname + urlObj.search,
      method: "GET",
      timeout: 30000, // 30 second timeout
    };

    const request = https
      .get(options, (response) => {
        if (response.statusCode === 302 || response.statusCode === 301) {
          request.destroy();
          const redirectUrl = response.headers.location;
          // Handle relative redirects
          const absoluteUrl = redirectUrl.startsWith("http")
            ? redirectUrl
            : `${urlObj.protocol}//${urlObj.host}${redirectUrl}`;
          return downloadText(absoluteUrl, redirectCount + 1)
            .then(resolve)
            .catch(reject);
        }
        if (response.statusCode !== 200) {
          request.destroy();
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
        response.on("end", () => {
          response.destroy();
          request.destroy();
          resolve(data);
        });
      })
      .on("error", (err) => {
        request.destroy();
        reject(err);
      })
      .on("timeout", () => {
        request.destroy();
        reject(new Error("Request timeout"));
      });
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
      // Use double quotes and escape them properly for PowerShell
      const archivePathEscaped = archivePath.replace(/"/g, '""');
      const binDirEscaped = BIN_DIR.replace(/"/g, '""');
      execSync(
        `powershell -Command "Expand-Archive -Path \"${archivePathEscaped}\" -DestinationPath \"${binDirEscaped}\" -Force"`,
        { stdio: "inherit" },
      );
    }

    // Move binary from extracted directory to bin directory
    // Try the expected directory name first
    const extractedDir = path.join(BIN_DIR, `augent-v${VERSION}-${target}`);
    let extractedBin = path.join(extractedDir, binName);

    // If not found, search for the binary in the bin directory
    if (!fs.existsSync(extractedBin)) {
      // Look for any directory that might contain the binary
      const entries = fs.readdirSync(BIN_DIR, { withFileTypes: true });
      for (const entry of entries) {
        if (entry.isDirectory() && entry.name.startsWith("augent-")) {
          const candidateBin = path.join(BIN_DIR, entry.name, binName);
          if (fs.existsSync(candidateBin)) {
            extractedBin = candidateBin;
            break;
          }
        }
      }
    }

    if (fs.existsSync(extractedBin)) {
      fs.renameSync(extractedBin, BIN_PATH);
      // Clean up the extracted directory
      const extractedDirToRemove = path.dirname(extractedBin);
      if (extractedDirToRemove !== BIN_DIR) {
        fs.rmSync(extractedDirToRemove, { recursive: true, force: true });
      }
    } else {
      throw new Error(`Binary not found in extracted archive: ${binName}`);
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
    extractArchive(archivePath, target);

    // Clean up archive and checksums
    fs.unlinkSync(archivePath);
    fs.unlinkSync(checksumsPath);

    // Make binary executable (Unix)
    if (process.platform !== "win32") {
      fs.chmodSync(BIN_PATH, 0o755);
    }

    // Verify binary exists and is accessible
    if (!fs.existsSync(BIN_PATH)) {
      throw new Error("Binary was not created successfully");
    }

    // Try to execute the binary to verify it works (non-Windows)
    if (process.platform !== "win32") {
      try {
        execSync(`"${BIN_PATH}" --version`, { stdio: "pipe", timeout: 5000 });
      } catch (error) {
        // If version check fails, log warning but don't fail installation
        console.warn("Warning: Could not verify binary execution");
      }
    }

    console.log("augent installed successfully");
    process.exit(0);
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
