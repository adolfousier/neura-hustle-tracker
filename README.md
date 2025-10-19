[![Rust](https://img.shields.io/badge/rust-%23000000.svg?style=for-the-badge&logo=rust&logoColor=white)](https://www.rust-lang.org)
[![Ratatui](https://img.shields.io/badge/ratatui-%23000000.svg?style=for-the-badge&logo=rust&logoColor=white)](https://ratatui.rs)
[![Docker](https://img.shields.io/badge/docker-%230db7ed.svg?style=for-the-badge&logo=docker&logoColor=white)](https://docker.com)
[![PostgreSQL](https://img.shields.io/badge/postgresql-%23316192.svg?style=for-the-badge&logo=postgresql&logoColor=white)](https://www.postgresql.org)

[![Neura Hustle Tracker](https://img.shields.io/badge/Neura%20Hustle%20Tracker-7f56da)](https://meetneura.ai) [![Powered by Neura AI](https://img.shields.io/badge/Powered%20by-Neura%20AI-7f56da)](https://meetneura.ai)

# Neura Hustle Tracker BETA

A cross-platform time-tracking tool for monitoring your productivity through app usage during work sessions. Built with Rust, Ratatui for the UI and Postgres database. Supports Windows, macOS (macOS and Windows not tested yet, please if you try provide feedback), and Linux (X11 and Wayland).

![Demo](src/screenshots/demo.png)

## Features

- **Interactive Dashboard**: Comprehensive data visualization with bar charts, timelines, and statistics
- **App Categorization**: Automatic categorization of apps (Development, Browsing, Communication, Media, Files, Email, Office, Other) with color coding
- **Fully Responsive Design**: Adaptive layout that adjusts to terminal size for optimal viewing on any device
- **Cross-Platform Support**: Works on Linux (X11), macOS, and Windows
- **Commands Menu**: Popup menu (Shift+C) showing all available shortcuts and commands
- **Multiple Views**: Daily, Weekly, Monthly, and History views with Tab navigation
- **App Renaming**: Interactive renaming of tracked applications
- **Session Management**: Manual start/end sessions with automatic saving
- **Real-time Tracking**: Live monitoring with 5-second dashboard updates and live session duration
- **Enhanced App Detection**: Tracks editors (vim, emacs, vscode), file managers, terminals, chat apps, media players, email clients, and office suites
- **Live Session Display**: Current active session shows real-time duration with [LIVE] indicator
- **Timestamped Logs**: All log entries include timestamps for better debugging
- **PostgreSQL Storage**: Persistent data storage with automatic migrations

## Which Mode Should I Use?

Neura Hustle Tracker supports two operating modes depending on your platform:

### Linux (X11/Wayland) - Unified Mode ✅
- **Recommended**: Use unified mode (default)
- **How it works**: TUI and tracking run in one process
- **Command**: `make run`
- **Why**: Linux window detection works perfectly even when TUI is running
- **Note**: Wayland users need [Window Calls extension](https://extensions.gnome.org/extension/4724/window-calls/)

### macOS/Windows - Daemon Mode 🔄
- **Recommended**: Use daemon mode for accurate tracking
- **How it works**:
  - Background daemon tracks all apps silently
  - TUI opens separately to view stats (doesn't interfere with tracking)
- **Commands**:
  - `make daemon-start` - Start background tracking
  - `make view` - Open TUI to view stats
  - `make daemon-stop` - Stop background tracking
- **Why**: On macOS/Windows, when the TUI runs, it becomes the focused window and can't detect other apps you switch to

### Feature Comparison

##### Basics

|                 | User owns data     | GUI                | Sync                       | Open Source        |
| --------------- |:------------------:|:------------------:|:--------------------------:|:------------------:|
| HustleTracker   | :white_check_mark: | :white_check_mark: | Centralized                | :white_check_mark: |
| [ActivityWatch] | :white_check_mark: | :white_check_mark: | WIP, decentralized         | :white_check_mark: |
| [RescueTime]    | :x:                | :white_check_mark: | Centralized                | :x:                |
| [Selfspy]       | :white_check_mark: | :x:                | :x:                        | :white_check_mark: |
| [ulogme]        | :white_check_mark: | :white_check_mark: | :x:                        | :white_check_mark: |
| [WakaTime]      | :x:                | :white_check_mark: | Centralized                | Clients            |

[ActivityWatch]: https://activitywatch.net/
[RescueTime]: https://www.rescuetime.com/
[Selfspy]: https://github.com/selfspy/selfspy
[ulogme]: https://github.com/karpathy/ulogme
[WakaTime]: https://wakatime.com/

##### Platforms

|               | Windows            | macOS              | Linux              | Android            | iOS                 |
| ------------- |:------------------:|:------------------:|:------------------:|:------------------:|:-------------------:|
| HustleTracker | :white_check_mark: | :white_check_mark: | :white_check_mark: | :x:                |:x:                  |
|[ActivityWatch]| :white_check_mark: | :white_check_mark: | :white_check_mark: | :white_check_mark: |:x:                  |
| [RescueTime]  | :white_check_mark: | :white_check_mark: | :white_check_mark: | :white_check_mark: |Limited              |
| [Selfspy]     | :white_check_mark: | :white_check_mark: | :white_check_mark: | :x:                |:x:                  |
| [ulogme]      | :x:                | :white_check_mark: | :white_check_mark: | :x:                |:x:                  |
| [WakaTime]    | :white_check_mark: | :white_check_mark: | :white_check_mark: | :x:                |:x:                  |

##### Tracking

|               | App & Window Title | AFK                | Browser Extensions | Editor Plugins     | Extensible            | Comprehensive App Detection |
| ------------- |:------------------:|:------------------:|:------------------:|:------------------:|:---------------------:|:---------------------------:|
| HustleTracker | :white_check_mark: | :white_check_mark: | :white_check_mark: | :x:                | :white_check_mark:    | :white_check_mark:          |
|[ActivityWatch]| :white_check_mark: | :white_check_mark: | :white_check_mark: | :white_check_mark: | :white_check_mark:    | :white_check_mark:          |
| [RescueTime]  | :white_check_mark: | :white_check_mark: | :white_check_mark: | :x:                | :x:                   | :white_check_mark:          |
| [Selfspy]     | :white_check_mark: | :white_check_mark: | :x:                | :x:                | :x:                   | :x:                         |
| [ulogme]      | :white_check_mark: | :white_check_mark: | :x:                | :x:                | :x:                   | :x:                         |
| [WakaTime]    | :x:                | :white_check_mark: | :white_check_mark: | :white_check_mark: | Only for text editors | :x:                         |

## Prerequisites

- Rust 1.90+
- Docker and Docker Compose (for easy Postgres setup)
- **Platform-specific requirements**:
  - **Linux**: Requires a GUI desktop environment (GNOME, KDE, etc.) to detect active applications. Works with X11 and Wayland.
  - **macOS**: Screen Recording permission may be required for window titles
  - **Windows**: No additional permissions needed

## One-Click Installation (No Prerequisites Required)

For users who don't have Rust, Docker, Git, or Make installed, use these one-liner commands to install everything and run the app automatically:

### Linux (Ubuntu/Debian)

```bash
sudo apt update && sudo apt install -y make docker.io curl git openssl && curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y && source ~/.cargo/env && git clone https://github.com/adolfousier/neura-hustle-tracker.git && cd neura-hustle-tracker && [ ! -f .env ] && USERNAME="timetracker_$(openssl rand -hex 4)" && PASSWORD="$(openssl rand -base64 16)" && echo -e "POSTGRES_USERNAME=$USERNAME\nPOSTGRES_PASSWORD=$PASSWORD\nDATABASE_URL=postgres://$USERNAME:$PASSWORD@localhost:5432/hustle-tracker" > .env && echo "alias hustle='cd $(pwd) && make run'" >> ~/.bashrc && source ~/.bashrc && make run
```

### macOS

```bash
echo "Please download and install Docker Desktop from https://docs.docker.com/desktop/install/mac-install/ before proceeding." && read -p "Press Enter to continue after installing Docker Desktop..." && brew install make git rustup-init && rustup-init -y && source ~/.cargo/env && git clone https://github.com/adolfousier/neura-hustle-tracker.git && cd neura-hustle-tracker && [ ! -f .env ] && USERNAME="timetracker_$(openssl rand -hex 4)" && PASSWORD="$(openssl rand -base64 16)" && echo -e "POSTGRES_USERNAME=$USERNAME\nPOSTGRES_PASSWORD=$PASSWORD\nDATABASE_URL=postgres://$USERNAME:$PASSWORD@localhost:5432/hustle-tracker" > .env && echo "alias hustle-start='cd $(pwd) && make daemon-start'" >> ~/.zshrc && echo "alias hustle-stop='cd $(pwd) && make daemon-stop'" >> ~/.zshrc && echo "alias hustle-view='cd $(pwd) && make view'" >> ~/.zshrc && echo "alias hustle-status='cd $(pwd) && make daemon-status'" >> ~/.zshrc && source ~/.zshrc && if ! docker info > /dev/null 2>&1; then echo "Error: Docker Desktop is not running. Please start Docker Desktop and re-run the command."; exit 1; fi && make daemon-start
```

### Windows (PowerShell)

```powershell
winget install --id=Rustlang.Rustup -e; winget install --id=GnuWin32.Make -e; winget install --id=Docker.DockerDesktop -e; winget install --id=Git.Git -e; git clone https://github.com/adolfousier/neura-hustle-tracker.git; cd neura-hustle-tracker; $env:PATH += ";$env:USERPROFILE\.cargo\bin"; if (!(Test-Path .env)) { $USERNAME = "timetracker_$((Get-Random -Maximum 65535).ToString('X4'))"; $PASSWORD = [Convert]::ToBase64String((Get-Random -Count 16 -Maximum 256)); "POSTGRES_USERNAME=$USERNAME`nPOSTGRES_PASSWORD=$PASSWORD`nDATABASE_URL=postgres://$USERNAME`:$PASSWORD@localhost:5432/hustle-tracker" | Out-File .env -Encoding UTF8 }; if (!(Test-Path $PROFILE)) { New-Item -Path $PROFILE -ItemType File -Force }; Add-Content $PROFILE "function hustle-start { Set-Location '$(Get-Location)'; make daemon-start }"; Add-Content $PROFILE "function hustle-stop { Set-Location '$(Get-Location)'; make daemon-stop }"; Add-Content $PROFILE "function hustle-view { Set-Location '$(Get-Location)'; make view }"; Add-Content $PROFILE "function hustle-status { Set-Location '$(Get-Location)'; make daemon-status }"; . $PROFILE; make daemon-start;
```

**What this does**: Installs all dependencies, clones repo, starts background tracking, creates global commands.

**After installation, use these commands from anywhere**:
- **Linux**: `hustle` - Start tracking with TUI
- **macOS/Windows**: `hustle-start`, `hustle-stop`, `hustle-view`, `hustle-status`

## Setup

**🎉 Zero Configuration Required!** Database credentials are auto-generated on first run if `.env` is missing. Existing `.env` files are never overwritten.

### Easiest Way - Using Make (Recommended)

**One single command does everything:**

```bash
# Clone and navigate to project directory
git clone https://github.com/adolfousier/neura-hustle-tracker
cd neura-hustle-tracker

# Run everything with ONE command:
make run
```

**⚠️ Important**: You **must** be inside the project directory (`cd neura-hustle-tracker`) before running `make run`.

**What `make run` does:**

- ✅ Starts PostgreSQL in Docker
- ✅ Builds optimized release binary
- ✅ Auto-generates secure database credentials
- ✅ Creates `.env` file
- ✅ Sets up database tables
- ✅ Starts tracking!

**Other useful Make commands:**

- `make dev` - Quick start with debug build (faster for development)
- `make help` - See all available commands
- `make clean` - Clean everything (build artifacts + database)

---

### Alternative - Manual Commands

If you prefer manual control or don't have Make:

```bash
# 1. Clone and navigate to directory
git clone https://github.com/adolfousier/neura-hustle-tracker
cd neura-hustle-tracker

# 2. Build and run (optimized release build):
docker compose up -d && cargo build --release && ./target/release/neura_hustle_tracker

# For Windows:
docker compose up -d && cargo build --release && .\target\release\neura_hustle_tracker.exe
```

**⚠️ Important**: You **must** be inside the project directory (`cd neura-hustle-tracker`) to run these commands.

**Note**: Using `cargo build --release` creates an optimized binary that runs faster. For development/testing, you can use `cargo run` (debug mode) instead.

---

### Platform-Specific Notes

#### Linux (X11/Wayland) - Unified Mode ✅

- Install Rust (`curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`)
- Install Docker and Docker Compose from your package manager
- Clone: `git clone https://github.com/adolfousier/neura-hustle-tracker && cd neura-hustle-tracker`
- **Quick Start**: `make run` (ONE command - starts DB, builds, runs!)
- **Manual**: `docker compose up -d && cargo build --release && ./target/release/neura_hustle_tracker`
- **Requirements**: GUI desktop environment (GNOME, KDE, etc.) for app detection

**Wayland Users (GNOME)**: Install the [Window Calls extension](https://extensions.gnome.org/extension/4724/window-calls/) for window tracking on Wayland. This extension provides D-Bus calls to return list of windows, move, resize, and close them.

**Note**: Linux tracking is 100% accurate in unified mode. X11 and Wayland (with extension) work perfectly!

#### macOS - Daemon Mode (Recommended)

**⚠️ Important**: Docker Desktop must be running for `make daemon-start` and `docker compose` commands to work.

- Install [Rust](https://rustup.rs/) and [Docker Desktop](https://www.docker.com/products/docker-desktop/)
- Clone: `git clone https://github.com/adolfousier/neura-hustle-tracker && cd neura-hustle-tracker`
- **Start tracking**: `make daemon-start` (starts DB + background daemon)
- **View stats**: `make view` (opens TUI to view stats)
- **Stop tracking**: `make daemon-stop`
- **Check status**: `make daemon-status`
- **Manual daemon start**: `docker compose up -d && cargo build --release --bin neura_hustle_daemon && ./target/release/neura_hustle_daemon &`
- **Manual TUI**: `cargo build --release --bin neura_hustle_tracker && ./target/release/neura_hustle_tracker`
- **Permissions**: Grant Screen Recording permission to Terminal in System Preferences > Security & Privacy > Privacy > Screen Recording

**Alternative - Unified Mode**: You can still use `make run` for unified mode, but tracking accuracy may be reduced when TUI is focused.

#### Windows - Daemon Mode (Recommended)

- Install [Rust](https://rustup.rs/) and [Docker Desktop](https://www.docker.com/products/docker-desktop/)
- Use PowerShell or CMD
- Clone: `git clone https://github.com/adolfousier/neura-hustle-tracker && cd neura-hustle-tracker`
- **Start tracking**: `make daemon-start` (starts DB + background daemon)
- **View stats**: `make view` (opens TUI to view stats)
- **Stop tracking**: `make daemon-stop`
- **Check status**: `make daemon-status`
- **Manual daemon start**: `docker compose up -d && cargo build --release --bin neura_hustle_daemon && .\target\release\neura_hustle_daemon.exe`
- **Manual TUI**: `cargo build --release --bin neura_hustle_tracker && .\target\release\neura_hustle_tracker.exe`
- **Permissions**: Windows API is used - usually no special permissions needed

**Alternative - Unified Mode**: You can still use `make run` for unified mode, but tracking accuracy may be reduced when TUI is focused.

---

### Advanced: Custom Database Credentials

By default, credentials are auto-generated. To use custom credentials:

1. Copy `.env.example` to `.env`
2. Edit `.env` with your values:

   ```
   POSTGRES_USERNAME=your_username
   POSTGRES_PASSWORD=your_password
   DATABASE_URL=postgres://your_username:your_password@localhost:5432/hustle-tracker
   ```

3. Run: `make run` or `docker compose up -d && cargo run`

### Advanced: Local PostgreSQL (No Docker)

1. Install and start PostgreSQL locally
2. Create database: `CREATE DATABASE hustle-tracker;`
3. Create `.env` file with your credentials (see above)
4. Navigate to project: `cd neura-hustle-tracker`
5. Run: `cargo build --release && ./target/release/neura_hustle_tracker`

## Startup on Boot/Login

To run Neura Hustle Tracker automatically on system startup:

**Note**: The startup scripts include a 30-second delay to allow system services (like Docker) to fully initialize before launching the application.

### Ubuntu/Linux (GNOME)

```bash
mkdir -p ~/.config/autostart/ && cp src/scripts/startup/neura-tracker.desktop ~/.config/autostart/
```

Then edit `~/.config/autostart/neura-tracker.desktop` and replace `/path/to/neura-hustle-tracker` with your actual project directory path (e.g., `/home/user/neura-hustle-tracker`).

Log out and back in to start automatically.

### macOS

```bash
mkdir -p ~/Library/LaunchAgents/ && cp src/scripts/startup/neura-tracker.plist ~/Library/LaunchAgents/ && launchctl load ~/Library/LaunchAgents/neura-tracker.plist
```

Log out and back in to start automatically.

### Windows

```cmd
copy src\scripts\startup\neura-tracker.bat "%APPDATA%\Microsoft\Windows\Start Menu\Programs\Startup\"
```

Or use Task Scheduler to run the batch file at logon.

## Usage

The app provides a terminal-based interface for time tracking with an interactive dashboard.

### Commands

- **Tab**: Switch between dashboard views (Daily/Weekly/Monthly)
- **h**: View full session history (scrollable popup with ↑/↓/PgUp/PgDn)
- **Shift+C**: Open commands popup menu with all available shortcuts
- **r**: Rename apps/tabs (arrow keys to navigate, Enter to select)
- **l**: View application logs with timestamps
- **q**: Quit the application

**Note**: The app starts tracking automatically when launched and displays visual analytics with bar charts and detailed statistics.

Sessions automatically track the active application and duration with real-time updates every 5 seconds. The current active session shows live duration with a [LIVE] indicator. Data is saved to Postgres every hour automatically, or when switching applications. Sessions shorter than 10 seconds are combined with consecutive sessions of the same app.

## Architecture

The application is organized into modular services:

- `active_window/`: Background daemon for window tracking (macOS/Windows architecture)
- `database/`: PostgreSQL connection and queries
- `tracker/`: Cross-platform application monitoring using active-win-pos-rs
- `ui/`: Ratatui-based terminal interface (works on Windows, macOS, Linux)
- `config/`: Configuration management
- `models/`: Data structures
- `utils/`: Helper utilities

### Daemon Mode (macOS/Windows)

For macOS and Windows users who need background tracking without the TUI interfering:

```bash
# Start background daemon
cargo build --release
./target/release/neura_hustle_daemon  # macOS/Linux
# or
.\target\release\neura_hustle_daemon.exe  # Windows

# In another terminal, view stats with TUI
./target/release/neura_hustle_tracker  # macOS/Linux
# or
.\target\release\neura_hustle_tracker.exe  # Windows
```

The daemon runs silently in the background tracking active windows, while the TUI can be opened/closed to view stats without affecting tracking.

## Supported Platforms

- **Linux**: X11 and Wayland support
- **macOS**: Full support with Accessibility API
- **Windows**: Full support with Windows API

## Testing

Run `cargo test` to execute unit tests for database operations and core functionality.

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for detailed contribution guidelines.

## License

This project is licensed under the [LICENSE](LICENSE) file.

[![Star History Chart](https://api.star-history.com/svg?repos=adolfousier/neura-hustle-tracker&type=Date)](https://star-history.com/#adolfousier/neura-hustle-tracker&Date)
