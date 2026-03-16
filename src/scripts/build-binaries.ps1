param(
    [string]$OutputDir = ".",
    [string]$ReleaseTag = "local"
)

Write-Host "Building Hustle Tracker Executables" -ForegroundColor Cyan
Write-Host "==========================================" -ForegroundColor Cyan
Write-Host "Output directory: $OutputDir"
Write-Host "Release tag: $ReleaseTag"
Write-Host ""

$binDir = Join-Path $OutputDir "bin"
New-Item -ItemType Directory -Path $binDir -Force | Out-Null

$arch = if ([System.Environment]::Is64BitOperatingSystem) { "x86_64" } else { "i686" }

Write-Host "Building for Windows ($arch)..." -ForegroundColor Yellow

try {
    Write-Host "Building TUI binary..."
    & cargo build --release --bin hustle_tracker
    if ($LASTEXITCODE -ne 0) { throw "TUI build failed" }

    Write-Host "Building Daemon binary..."
    & cargo build --release --bin hustle_daemon
    if ($LASTEXITCODE -ne 0) { throw "Daemon build failed" }

    Write-Host "Copying binaries..."
    Copy-Item "target/release/hustle_tracker.exe" "$binDir/hustle_tracker-windows-$arch.exe"
    Copy-Item "target/release/hustle_daemon.exe" "$binDir/hustle_daemon-windows-$arch.exe"

    Write-Host "✓ Windows binaries built successfully" -ForegroundColor Green
} catch {
    Write-Host "✗ Build failed: $_" -ForegroundColor Red
    exit 1
}

Write-Host ""
Write-Host "Build Summary" -ForegroundColor Cyan
Write-Host "=============" -ForegroundColor Cyan
Write-Host "Location: $binDir/"
Get-ChildItem $binDir | Format-Table -AutoSize
Write-Host ""
Write-Host "✓ All binaries built successfully!" -ForegroundColor Green
Write-Host ""
Write-Host "Next steps:" -ForegroundColor Yellow
Write-Host "1. Test the binaries with Docker and PostgreSQL running"
Write-Host "2. Create a GitHub release with tag: $ReleaseTag"
Write-Host "3. Upload these binaries to the release"
