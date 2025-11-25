# Release Process for v0.4.15+

This document outlines how to build and release executables for all platforms.

## Architecture

### Linux
- **Single unified binary**: `neura_hustle_tracker` (includes both TUI and backend)
- No daemon needed - tracking and UI run together

### macOS & Windows
- **daemon**: Background tracking service
- **tui**: Frontend dashboard viewer
- Two separate binaries for flexibility

## Building Executables

### Option 1: Automated GitHub Actions (Recommended)

1. Create a new release tag on GitHub:
   ```bash
   git tag v0.4.15
   git push origin v0.4.15
   ```

2. GitHub Actions will automatically:
   - Build binaries for Linux, macOS (x86_64 & ARM64), and Windows
   - Attach them to the GitHub release
   - Users can download directly from the release page

### Option 2: Manual Local Build

**On Linux:**
```bash
just build-release-linux
# Binaries go to ./dist/bin/
```

**On macOS:**
```bash
just build-release-macos
# Binaries go to ./dist/bin/ for both x86_64 and ARM64
# Note: You may need to build on both architectures separately
```

**On Windows:**
```powershell
just build-release-windows
# Binaries go to ./dist/bin/
```

Or use the scripts directly:
```bash
./src/scripts/build-binaries.sh ./dist linux
```

```powershell
PowerShell -ExecutionPolicy Bypass -File src/scripts/build-binaries.ps1 -OutputDir ./dist -ReleaseTag windows
```

## Release Workflow for v0.4.15

### Step 1: Update Version
```bash
# Update Cargo.toml
# Change version from 0.4.14 to 0.4.15
```

### Step 2: Add Changelog Entry
```bash
# Update CHANGELOG.md with new features and fixes
```

### Step 3: Commit Changes
```bash
git add Cargo.toml CHANGELOG.md
git commit -m "Release v0.4.15: Command improvements and binary releases"
```

### Step 4: Create Release Tag
```bash
git tag v0.4.15
git push origin main
git push origin v0.4.15
```

### Step 5: Wait for GitHub Actions
- GitHub Actions will automatically build all binaries
- Check the "Actions" tab to see the build status
- Once complete, binaries will be attached to the release

### Step 6: Create Release Notes (if needed)
If you need to add custom release notes:
```bash
gh release edit v0.4.15 --notes "Release notes here..."
```

## Binary Naming Convention

```
neura_hustle_tracker-{os}-{arch}[.exe]
neura_hustle_daemon-{os}-{arch}[.exe]
```

Examples:
- `neura_hustle_tracker-linux-x86_64`
- `neura_hustle_tracker-macos-x86_64`
- `neura_hustle_tracker-macos-aarch64`
- `neura_hustle_tracker-windows-x86_64.exe`

## Distribution

Users can:
1. Download binaries from GitHub releases
2. Run directly (only need Docker and PostgreSQL)
3. No Rust or Cargo installation required

### Usage for End Users

**Linux:**
```bash
# Download neura_hustle_tracker-linux-x86_64
chmod +x neura_hustle_tracker-linux-x86_64
docker compose up -d  # Start PostgreSQL
./neura_hustle_tracker-linux-x86_64
```

**macOS:**
```bash
# Download appropriate binary (x86_64 or aarch64)
chmod +x neura_hustle_*-macos-*
docker compose up -d  # Start PostgreSQL
./neura_hustle_daemon-macos-* > daemon.log 2>&1 &
./neura_hustle_tui-macos-*
```

**Windows:**
```powershell
# Download .exe files
docker compose up -d  # Start PostgreSQL
.\neura_hustle_daemon-windows-x86_64.exe
.\neura_hustle_tui-windows-x86_64.exe
```

## Troubleshooting

**Build fails with Rust errors:**
- Make sure Rust is up to date: `rustup update`
- Clean previous builds: `cargo clean`

**macOS universal binaries:**
- Current setup builds for specific architectures
- To create universal binaries, use `cargo-lipo` or `xcrun lipo`

**Windows executable signing:**
- Consider code signing for production releases
- Users may get warnings without proper certificates

## Deprecating Make in Favor of Just

For developers, the new `justfile` provides simpler commands:
- `just run` instead of `make run`
- `just build-release-linux` instead of custom scripts
- Users can still use `make` for backwards compatibility

See `justfile` for all available commands.
