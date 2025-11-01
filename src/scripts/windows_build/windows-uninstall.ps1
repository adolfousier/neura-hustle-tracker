# Neura Hustle Tracker - Windows Uninstall Script
# This script removes the application, database volume, and local directory

Write-Host "=====================================" -ForegroundColor Cyan
Write-Host "Neura Hustle Tracker - Uninstall" -ForegroundColor Cyan
Write-Host "=====================================" -ForegroundColor Cyan
Write-Host ""

Write-Host "This will:" -ForegroundColor Yellow
Write-Host "  1. Stop the PostgreSQL database" -ForegroundColor Yellow
Write-Host "  2. Remove the database volume (all tracked data)" -ForegroundColor Yellow
Write-Host "  3. Delete the local installation directory" -ForegroundColor Yellow
Write-Host ""
Write-Host "⚠️  WARNING: This cannot be undone!" -ForegroundColor Red
Write-Host ""

# First confirmation
$response = Read-Host "Do you want to proceed? (yes/no)"
if ($response -ne "yes") {
    Write-Host "✗ Uninstall cancelled." -ForegroundColor Yellow
    exit 0
}

Write-Host ""
Write-Host "Stopping Docker Compose and removing database..." -ForegroundColor Cyan

try {
    docker compose down -v
    Write-Host "✓ Docker Compose stopped and volume removed" -ForegroundColor Green
} catch {
    Write-Host "Docker Compose already stopped or not running" -ForegroundColor Yellow
}

Write-Host ""
Write-Host "✗ FINAL WARNING: This will delete ALL files in:" -ForegroundColor Red
$currentPath = Get-Location
Write-Host "  $currentPath" -ForegroundColor Red
Write-Host ""

$confirm = Read-Host "Type 'yes' to confirm complete deletion"

if ($confirm -ne "yes") {
    Write-Host "✗ Cancelled. Directory kept." -ForegroundColor Yellow
    exit 0
}

Write-Host ""
Write-Host "Removing installation directory..." -ForegroundColor Cyan

try {
    # Go up one directory level
    Set-Location ..

    # Get the directory name to remove
    $dirToRemove = Split-Path -Leaf $currentPath

    # Remove the directory
    Remove-Item -Path $dirToRemove -Recurse -Force

    Write-Host ""
    Write-Host "✓ Uninstall complete!" -ForegroundColor Green
    Write-Host ""
    Write-Host "Neura Hustle Tracker has been completely removed." -ForegroundColor Green
    Write-Host "All tracked data has been deleted." -ForegroundColor Green
} catch {
    Write-Host ""
    Write-Host "✗ Error during uninstall: $_" -ForegroundColor Red
    Write-Host "You may need to manually delete the directory." -ForegroundColor Yellow
    exit 1
}
