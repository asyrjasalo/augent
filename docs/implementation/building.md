# Building Augent

This document describes how to build Augent for distribution.

## Building Rust Binary

### Development Build

```bash
cargo build
```

### Release Build

```bash
cargo build --release
```

The binary will be located at `target/release/augent`.

### Cross-Platform Builds

For cross-compilation, use [cross](https://github.com/cross-rs/cross):

```bash
# Install cross
cargo install cross --git https://github.com/cross-rs/cross

# Build for Linux ARM64
cross build --release --target aarch64-unknown-linux-gnu

# Build for Windows
cross build --release --target x86_64-pc-windows-msvc
```

## Building Python Wheels

Augent uses [Maturin](https://github.com/PyO3/maturin) to build Python wheels that include the compiled Rust binary.

### Install Maturin

```bash
# Via pip
pip install maturin

# Via cargo
cargo install maturin
```

### Build Wheel

```bash
# Build wheel for current platform
maturin build --release

# Build wheel with specific target
maturin build --release --target x86_64-unknown-linux-gnu
```

Wheels will be created in `target/wheels/`.

### Build for Multiple Platforms

```bash
# Linux x86_64 (manylinux)
maturin build --release --target x86_64-unknown-linux-gnu --manylinux auto

# Linux ARM64 (manylinux)
maturin build --release --target aarch64-unknown-linux-gnu --manylinux auto

# macOS x86_64
maturin build --release --target x86_64-apple-darwin

# macOS ARM64 (Apple Silicon)
maturin build --release --target aarch64-apple-darwin

# Windows x86_64
maturin build --release --target x86_64-pc-windows-msvc

# Windows ARM64
maturin build --release --target aarch64-pc-windows-msvc
```

### Test Wheel Locally

```bash
# Build and install in development mode
maturin develop

# Test the installed package
augent --version

# Or install from built wheel
pip install target/wheels/augent-*.whl
```

## Publishing

### Publish to PyPI

```bash
# Build wheels for all platforms (requires cross-compilation setup)
maturin build --release

# Publish to PyPI
maturin publish

# Or publish specific wheel
maturin upload target/wheels/augent-*.whl
```

### Publish to crates.io

```bash
# Login to crates.io
cargo login

# Publish
cargo publish
```

## CI/CD

### Automated Builds

The project uses GitHub Actions for automated builds:

- **ci.yml**: Runs tests and builds binaries for all platforms on every push
- **release.yml**: Builds Python wheels and publishes to PyPI and crates.io on tags

### Creating a Release

1. Update version in `Cargo.toml` and `pyproject.toml`
2. Update `CHANGELOG.md` with release notes
3. Commit changes
4. Create and push a tag:

```bash
git tag -a v0.1.0 -m "Release v0.1.0"
git push origin v0.1.0
```

5. GitHub Actions will automatically:
   - Build wheels for all platforms
   - Publish to PyPI
   - Publish to crates.io
   - Create a GitHub release with binaries

## Platform Support

Augent is built for these platforms:

| Platform | Target | Notes |
|----------|--------|-------|
| Linux x86_64 | `x86_64-unknown-linux-gnu` | manylinux wheels |
| Linux ARM64 | `aarch64-unknown-linux-gnu` | manylinux wheels |
| macOS x86_64 | `x86_64-apple-darwin` | Intel Macs |
| macOS ARM64 | `aarch64-apple-darwin` | Apple Silicon |
| Windows x86_64 | `x86_64-pc-windows-msvc` | 64-bit Windows |
| Windows ARM64 | `aarch64-pc-windows-msvc` | ARM64 Windows |

## Troubleshooting

### Maturin Build Fails

**Problem:** `error: no such subcommand: 'maturin'`

**Solution:** Install Maturin:

```bash
pip install maturin
```

### Cross-Compilation Fails

**Problem:** `error: linker not found`

**Solution:** Install cross-compilation toolchain:

```bash
# For Linux ARM64 on Linux x86_64
sudo apt-get install gcc-aarch64-linux-gnu

# Or use cross (recommended)
cargo install cross --git https://github.com/cross-rs/cross
cross build --release --target aarch64-unknown-linux-gnu
```

### PyPI Upload Permission Denied

**Problem:** `error: cannot upload to PyPI without authentication`

**Solution:** Configure PyPI credentials:

```bash
# Create ~/.pypirc
[pypi]
username = __token__
password = pypi-your-api-token-here
```

Or set environment variable:

```bash
export MATURIN_PYPI_TOKEN=pypi-your-api-token-here
maturin publish
```

### GitHub Actions Secrets

For automated publishing, configure these secrets in GitHub repository settings:

- `CARGO_REGISTRY_TOKEN`: crates.io API token
- `PYPI_API_TOKEN`: PyPI API token (for manual setup, though PyPI recommends trusted publishing)

For PyPI trusted publishing (recommended):

1. Go to PyPI project settings
2. Add GitHub Actions as trusted publisher
3. No token needed - uses OIDC

## See Also

- [Maturin Documentation](https://www.maturin.rs/)
- [cross Documentation](https://github.com/cross-rs/cross)
- [Cargo Book - Publishing](https://doc.rust-lang.org/cargo/reference/publishing.html)
