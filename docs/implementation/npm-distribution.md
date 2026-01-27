# npm Distribution for bunx Compatibility

## Overview

Augent is distributed via npm for use with both npm/pnpm/yarn and Bun's `bunx`. This document explains how the npm package is structured and published.

## Problem

Bun blocks `postinstall` scripts by default for security reasons. The previous approach used a `postinstall` script to download binaries from GitHub releases, which didn't work with `bunx`.

## Solution

The npm package now includes pre-built binaries for all supported platforms directly in the package, eliminating the need for postinstall scripts.

## Package Structure

```text
augent/
├── package.json          # Package metadata
├── scripts/
│   └── augent-wrapper.js # Platform-aware wrapper script (Node.js)
├── bin/
│   ├── darwin-x64/       # macOS Intel binaries
│   │   └── augent
│   ├── darwin-arm64/     # macOS Apple Silicon binaries
│   │   └── augent
│   ├── linux-x64/        # Linux x86_64 binaries
│   │   └── augent
│   ├── linux-arm64/      # Linux ARM64 binaries
│   │   └── augent
│   ├── win32-x64/        # Windows x64 binaries
│   │   └── augent.exe
│   └── win32-arm64/      # Windows ARM64 binaries
│       └── augent.exe
├── README.md
└── LICENSE
```

## Wrapper Script

The `scripts/augent-wrapper.js` wrapper script:

1. Detects the current platform and architecture
2. Selects the appropriate binary from platform-specific directories
3. Executes the binary with all command-line arguments
4. Provides clear error messages if a binary is missing

This approach works with:

- `npm install -g augent` → `augent`
- `npx augent` → `augent`
- `bunx augent` → `augent`
- `pnpm add -g augent` → `augent`
- `yarn global add augent` → `augent`

## Release Workflow

The GitHub Actions release workflow (`release.yml`) includes a `publish-npm` job that:

1. Downloads all platform binaries from build artifacts (created by the `build-binaries` job)
2. Extracts them into platform-specific directories under `bin/`
3. Makes binaries executable (Unix)
4. Makes the wrapper script executable
5. Publishes the complete package to npm

The binaries are downloaded directly from build artifacts, ensuring they match the exact version being published and eliminating the need to wait for GitHub releases.

## Package Configuration

### package.json

- **`bin`**: Points to `./scripts/augent-wrapper.js` (the wrapper script)
- **`files`**: Includes `bin/` and `scripts/` directories
- **No `postinstall` script**: Binaries are pre-packaged
- **No `os`/`cpu` restrictions**: All platforms supported in one package

### .gitignore

- Platform-specific binary directories (`bin/*/`) are ignored
- The wrapper script (`scripts/augent-wrapper.js`) is committed to git
- Binary executables (`bin/*.exe`) are ignored

## Benefits

1. **bunx compatibility**: Works without requiring `trustedDependencies`
2. **Faster installation**: No download step during install
3. **Offline support**: Binaries are included in the package
4. **Consistent behavior**: Same package works for all package managers
5. **Smaller package size**: Only includes necessary binaries (though npm will download the full package)

## Platform Support

The npm package supports:

- **Linux**: x64, ARM64
- **macOS**: x64 (Intel), ARM64 (Apple Silicon)
- **Windows**: x64, ARM64

All platforms are included in a single npm package, with the wrapper script selecting the correct binary at runtime.

## Testing

### Quick Test (Current Platform Only)

Test the wrapper script with your current platform's binary:

```bash
cargo build --release

mkdir -p bin/darwin-arm64
cp target/release/augent bin/darwin-arm64/augent
chmod +x bin/darwin-arm64/augent

node scripts/augent-wrapper.js --version

npm link
augent --version
npm unlink -g augent
```

### Full Test (Create npm Package)

Test the complete npm package as it would be published:

```bash
cargo build --release

mkdir -p bin
cp target/release/augent bin/darwin-arm64/augent
chmod +x bin/darwin-arm64/augent

npm pack

npm install -g augent-*.tgz

augent --version
augent list

bunx augent --version

npm uninstall -g augent
rm augent-*.tgz
rm -rf bin/
```

### Testing Multiple Platforms

To test multiple platforms locally (requires cross-compilation):

```bash
cargo install cross --git https://github.com/cross-rs/cross

cargo build --release --target x86_64-apple-darwin
cargo build --release --target aarch64-apple-darwin
cross build --release --target x86_64-unknown-linux-gnu
cross build --release --target aarch64-unknown-linux-gnu

mkdir -p bin/{darwin-x64,darwin-arm64,linux-x64,linux-arm64}
cp target/x86_64-apple-darwin/release/augent bin/darwin-x64/
cp target/aarch64-apple-darwin/release/augent bin/darwin-arm64/
cp target/x86_64-unknown-linux-gnu/release/augent bin/linux-x64/
cp target/aarch64-unknown-linux-gnu/release/augent bin/linux-arm64/
chmod +x bin/*/augent

npm pack
npm install -g augent-*.tgz
augent --version
```

### Testing the Wrapper Script Directly

Test the wrapper script without npm:

```bash
mkdir -p bin/darwin-arm64
cp target/release/augent bin/darwin-arm64/augent
chmod +x bin/darwin-arm64/augent

node scripts/augent-wrapper.js --version

node scripts/augent-wrapper.js list
node scripts/augent-wrapper.js help
```

### Verifying Package Contents

Check what files are included in the npm package:

```bash
npm pack

tar -xzf augent-*.tgz
ls -la package/
ls -la package/bin/
ls -la package/scripts/

rm -rf package/ augent-*.tgz
```

## Future Improvements

- Consider using `optionalDependencies` with platform-specific packages for smaller downloads
- Add binary size optimization
- Consider using `@vercel/ncc` or similar to bundle the wrapper script if needed
