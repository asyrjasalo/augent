#!/usr/bin/env node

/**
 * Platform-aware wrapper for augent binary
 * This script detects the platform and executes the correct binary
 * Works with both npm and bunx without requiring postinstall scripts
 */

const fs = require("fs");
const path = require("path");
const os = require("os");
const { spawn, execFile } = require("child_process");

// Map Node.js platform/arch to binary directory name
function getBinaryPath() {
  const platform = process.platform;
  const arch = process.arch;

  // Normalize architecture names
  const normalizedArch =
    arch === "arm64" || arch === "aarch64" ? "arm64" : arch;

  let platformDir;
  if (platform === "linux") {
    platformDir = normalizedArch === "arm64" ? "linux-arm64" : "linux-x64";
  } else if (platform === "darwin") {
    platformDir = normalizedArch === "arm64" ? "darwin-arm64" : "darwin-x64";
  } else if (platform === "win32") {
    platformDir = normalizedArch === "arm64" ? "win32-arm64" : "win32-x64";
  } else {
    throw new Error(`Unsupported platform: ${platform} ${arch}`);
  }

  const binName = platform === "win32" ? "augent.exe" : "augent";
  // Script is in scripts/, binaries are in bin/ (sibling directory)
  const binPath = path.join(__dirname, "..", "bin", platformDir, binName);

  if (!fs.existsSync(binPath)) {
    throw new Error(
      `Binary not found for ${platform} ${arch} at ${binPath}\n` +
        `This package may not support your platform. Please check:\n` +
        `https://github.com/asyrjasalo/augent/releases`,
    );
  }

  return binPath;
}

// Execute the binary with all arguments
const binaryPath = getBinaryPath();
const args = process.argv.slice(2);

// Use execFile on Windows for better compatibility, spawn elsewhere
// execFile is specifically designed for executables and handles Windows better
const isWindows = process.platform === "win32";
const child = isWindows
  ? execFile(binaryPath, args, { stdio: "inherit" })
  : spawn(binaryPath, args, { stdio: "inherit", shell: false });

// Forward signals so Ctrl+C etc. reaches the child (Unix)
if (process.platform !== "win32") {
  ["SIGINT", "SIGTERM"].forEach((sig) => {
    process.on(sig, () => {
      child.kill(sig);
    });
  });
}

child.on("error", (err) => {
  console.error(`Failed to execute augent: ${err.message}`);
  process.exit(1);
});

child.on("exit", (code, signal) => {
  if (code !== null) {
    process.exit(code);
  }
  if (signal && os.constants.signals && os.constants.signals[signal] != null) {
    process.exit(128 + os.constants.signals[signal]);
  }
  process.exit(0);
});
