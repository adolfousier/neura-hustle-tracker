# Windows PowerShell Installation Script for Neura Hustle Tracker
# Run this script in PowerShell as Administrator

Write-Host "Installing Neura Hustle Tracker dependencies..." -ForegroundColor Green

# Function to check if a package is installed and handle upgrade prompt
function Install-OrUpgradePackage {
    param (
        [string]$PackageId,
        [string]$FriendlyName
    )
    
    Write-Host "`nChecking $FriendlyName..." -ForegroundColor Cyan
    
    # Check if package is already installed
    $installed = winget list --id $PackageId --exact 2>$null
    
    if ($LASTEXITCODE -eq 0 -and $installed -match $PackageId) {
        Write-Host "$FriendlyName is already installed." -ForegroundColor Yellow
        $response = Read-Host "Would you like to upgrade it? (y/n)"
        
        if ($response -eq 'y' -or $response -eq 'Y') {
            Write-Host "Upgrading $FriendlyName..." -ForegroundColor Green
            winget upgrade --id=$PackageId -e --silent
        } else {
            Write-Host "Continuing with installed version of $FriendlyName." -ForegroundColor Green
        }
    } else {
        Write-Host "Installing $FriendlyName..." -ForegroundColor Green
        winget install --id=$PackageId -e --silent
    }
}

# Install/upgrade required packages
Install-OrUpgradePackage -PackageId "Rustlang.Rustup" -FriendlyName "Rust"
Install-OrUpgradePackage -PackageId "GnuWin32.Make" -FriendlyName "Make"
Install-OrUpgradePackage -PackageId "Docker.DockerDesktop" -FriendlyName "Docker Desktop"
Install-OrUpgradePackage -PackageId "Git.Git" -FriendlyName "Git"

# Refresh PATH after installations
$env:PATH = [System.Environment]::GetEnvironmentVariable("Path","Machine") + ";" + [System.Environment]::GetEnvironmentVariable("Path","User")

# Add Make to PATH (GnuWin32)
$env:PATH += ";C:\Program Files (x86)\GnuWin32\bin;C:\Program Files\GnuWin32\bin"

# Clone repository with verification
$expectedHash = "abc123..."  # Get from trusted source
git clone https://github.com/adolfousier/neura-hustle-tracker.git 2>$null
cd neura-hustle-tracker

# Verify repository integrity
$actualHash = (Get-FileHash -Path "Cargo.toml" -Algorithm SHA256).Hash
if ($actualHash -ne $expectedHash) {
    Write-Host "SECURITY WARNING: Repository hash mismatch!" -ForegroundColor Red
    exit 1
}

# Prompt user to review code before continuing
Write-Host "Please review the source code before continuing." -ForegroundColor Yellow
$confirm = Read-Host "Have you reviewed the code and wish to continue? (yes/no)"
if ($confirm -ne "yes") {
    exit 0
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
Write-Host "`nChecking Docker..." -ForegroundColor Green
try {
    & docker version 2>$null | Out-Null
    Write-Host "Docker is running." -ForegroundColor Green
} catch {
    Write-Host "Docker Desktop is not running. Please start Docker Desktop manually and re-run the script." -ForegroundColor Red
    exit 1
}

Write-Host "`nStarting daemon..." -ForegroundColor Green
& "C:\Program Files (x86)\GnuWin32\bin\make.exe" daemon-start

Write-Host "`nInstallation complete! Use hustle-start, hustle-stop, hustle-view, and hustle-status commands." -ForegroundColor Cyan
