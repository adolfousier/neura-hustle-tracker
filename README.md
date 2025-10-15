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
sudo apt update && sudo apt install -y make docker.io curl git && curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y && source ~/.cargo/env && git clone https://github.com/adolfousier/neura-hustle-tracker.git && cd neura-hustle-tracker && make run
```

### macOS
```bash
brew install make docker git rustup-init && rustup-init -y && source ~/.cargo/env && git clone https://github.com/adolfousier/neura-hustle-tracker.git && cd neura-hustle-tracker && make run
```

### Windows (PowerShell)
```powershell
winget install --id=Rustlang.Rustup -e; winget install --id=GnuWin32.Make -e; winget install --id=Docker.DockerDesktop -e; winget install --id=Git.Git -e; git clone https://github.com/adolfousier/neura-hustle-tracker.git; cd neura-hustle-tracker; $env:PATH += ";$env:USERPROFILE\.cargo\bin"; make run;
```

**Note**: These commands install all required dependencies, clone the repository, and start the application. Admin/sudo privileges may be required for installations.

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
docker-compose up -d && cargo build --release && ./target/release/neura_hustle_tracker

# For Windows:
docker-compose up -d && cargo build --release && .\target\release\neura_hustle_tracker.exe
```

**⚠️ Important**: You **must** be inside the project directory (`cd neura-hustle-tracker`) to run these commands.

**Note**: Using `cargo build --release` creates an optimized binary that runs faster. For development/testing, you can use `cargo run` (debug mode) instead.

---

### Platform-Specific Notes

#### Windows
- Install [Rust](https://rustup.rs/) and [Docker Desktop](https://www.docker.com/products/docker-desktop/)
- Use PowerShell or CMD
- Clone: `git clone https://github.com/adolfousier/neura-hustle-tracker && cd neura-hustle-tracker`
- Run: `make run` (if you have Make) OR `docker-compose up -d && cargo build --release && .\target\release\neura_hustle_tracker.exe`
- **Note**: Windows API is used for app detection - usually no special permissions needed

#### macOS
- Install [Rust](https://rustup.rs/) and [Docker Desktop](https://www.docker.com/products/docker-desktop/)
- Clone: `git clone https://github.com/adolfousier/neura-hustle-tracker && cd neura-hustle-tracker`
- Run: `make run` OR `docker-compose up -d && cargo build --release && ./target/release/neura_hustle_tracker`
- **Note**: Grant Screen Recording permission to Terminal in System Preferences > Security & Privacy > Privacy > Screen Recording

#### Linux
- Install Rust (`curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`)
- Install Docker and Docker Compose from your package manager
- Clone: `git clone https://github.com/adolfousier/neura-hustle-tracker && cd neura-hustle-tracker`
- Run: `make run` OR `docker-compose up -d && cargo build --release && ./target/release/neura_hustle_tracker`
- **Note**: Requires GUI desktop environment (GNOME, KDE, etc.) for app detection. Works with X11 and Wayland.

---

### Advanced: Custom Database Credentials

By default, credentials are auto-generated. To use custom credentials:

1. Copy `.env.example` to `.env`
2. Edit `.env` with your values:
   ```
   POSTGRES_USERNAME=your_username
   POSTGRES_PASSWORD=your_password
   DATABASE_URL=postgres://your_username:your_password@localhost:5432/time_tracker
   ```
3. Run: `make run` or `docker-compose up -d && cargo run`

### Advanced: Local PostgreSQL (No Docker)

1. Install and start PostgreSQL locally
2. Create database: `CREATE DATABASE time_tracker;`
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
- **Tab**: Switch between dashboard views (Daily/Weekly/Monthly/History)
- **Shift+C**: Open commands popup menu with all available shortcuts
- **r**: Rename apps/tabs (arrow keys to navigate, Enter to select)
- **m**: Manually set app name (if auto-detection fails)
- **u**: Update current app detection
- **l**: View application logs with timestamps
- **q**: Quit the application

**Note**: The app starts tracking automatically when launched and displays visual analytics with bar charts and detailed statistics.

Sessions automatically track the active application and duration with real-time updates every 5 seconds. The current active session shows live duration with a [LIVE] indicator. Data is saved to Postgres every hour automatically, or when switching applications. Sessions shorter than 10 seconds are combined with consecutive sessions of the same app.

## Architecture
The application is organized into modular services:
- `database/`: PostgreSQL connection and queries
- `tracker/`: Cross-platform application monitoring using active-win-pos-rs
- `ui/`: Ratatui-based terminal interface (works on Windows, macOS, Linux)
- `config/`: Configuration management
- `models/`: Data structures
- `utils/`: Helper utilities

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
