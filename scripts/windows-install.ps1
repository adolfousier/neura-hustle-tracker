# Windows PowerShell Installation Script for Neura Hustle Tracker
# Run this script in PowerShell as Administrator

Write-Host "Installing Neura Hustle Tracker dependencies..." -ForegroundColor Green

# Install required packages
winget install --id=Rustlang.Rustup -e
winget install --id=GnuWin32.Make -e
winget install --id=Docker.DockerDesktop -e
winget install --id=Git.Git -e

# Refresh PATH after installations
$env:PATH = [System.Environment]::GetEnvironmentVariable("Path","Machine") + ";" + [System.Environment]::GetEnvironmentVariable("Path","User")

# Add Make to PATH (GnuWin32)
$env:PATH += ";C:\Program Files (x86)\GnuWin32\bin;C:\Program Files\GnuWin32\bin"

# Clone repository
git clone https://github.com/adolfousier/neura-hustle-tracker.git 2>$null
cd neura-hustle-tracker

# Verify we are in the repo
if (!(Test-Path "Cargo.toml")) {
    Write-Host "Failed to enter the repository directory." -ForegroundColor Red
    exit 1
}

# Add Rust to PATH
$env:PATH += ";$env:USERPROFILE\.cargo\bin"

# Create .env file if it doesn't exist
$repoPath = Get-Location
if (!(Test-Path "$repoPath\.env")) {
    $rand = New-Object System.Random
    $bytes = New-Object byte[] 16
    $rand.NextBytes($bytes)
    $USERNAME = "timetracker_$($rand.Next(65535).ToString('X4'))"
    $PASSWORD = [Convert]::ToBase64String($bytes)
    $envContent = "POSTGRES_USERNAME=$USERNAME`nPOSTGRES_PASSWORD=$PASSWORD`nDATABASE_URL=postgres://$USERNAME`:$PASSWORD@localhost:5432/hustle-tracker"
    [System.IO.File]::WriteAllText("$repoPath\.env", $envContent)
    Write-Host "Created .env file with database credentials" -ForegroundColor Yellow
}

# Set execution policy to allow scripts
Set-ExecutionPolicy -ExecutionPolicy RemoteSigned -Scope CurrentUser -Force

# Create PowerShell profile functions
if (!(Test-Path $PROFILE)) {
    New-Item -Path $PROFILE -ItemType File -Force
    Write-Host "Created PowerShell profile" -ForegroundColor Yellow
}

$currentPath = Get-Location
Add-Content $PROFILE "function hustle-start { Set-Location '$currentPath'; make daemon-start }"
Add-Content $PROFILE "function hustle-stop { Set-Location '$currentPath'; make daemon-stop }"
Add-Content $PROFILE "function hustle-view { Set-Location '$currentPath'; make view }"
Add-Content $PROFILE "function hustle-status { Set-Location '$currentPath'; make daemon-status }"

# Reload profile
. $PROFILE

# Check if Docker is running
Write-Host "Checking Docker..." -ForegroundColor Green
try {
    & docker version 2>$null | Out-Null
    Write-Host "Docker is running." -ForegroundColor Green
} catch {
    Write-Host "Docker Desktop is not running. Please start Docker Desktop manually and re-run the script." -ForegroundColor Red
    exit 1
}

Write-Host "Starting daemon..." -ForegroundColor Green
& "C:\Program Files (x86)\GnuWin32\bin\make.exe" daemon-start

Write-Host "Installation complete! Use hustle-start, hustle-stop, hustle-view, and hustle-status commands." -ForegroundColor Cyan