# Neura Hustle Tracker

A cross-platform Rust-based time-tracking tool for monitoring productivity through app usage during work sessions. Built with Ratatui for the UI, and Postgres for history. Supports Windows, MacOS, and Linux.

![Demo](src/screenshots/demo.png)

## Features
- **Interactive Dashboard**: Comprehensive data visualization with bar charts, timelines, and statistics
- **App Categorization**: Automatic categorization of apps (Development, Browsing, Communication, Media, Files, Other) with color coding
- **Fully Responsive Design**: Adaptive layout that adjusts to terminal size for optimal viewing on any device
- **Cross-Platform Support**: Works on Linux (X11/Wayland), macOS, and Windows
- **Commands Menu**: Popup menu (Shift+C) showing all available shortcuts and commands
- **Multiple Views**: Daily, Weekly, Monthly, and History views with Tab navigation
- **App Renaming**: Interactive renaming of tracked applications
- **Session Management**: Manual start/end sessions with automatic saving
- **Real-time Tracking**: Live monitoring of active applications and usage time
- **PostgreSQL Storage**: Persistent data storage with automatic migrations

## Prerequisites
- Rust 1.90+
- Docker and Docker Compose (for easy Postgres setup)
- **Platform-specific requirements**:
  - **Linux**: Requires a GUI desktop environment (GNOME, KDE, etc.) to detect active applications. Works with X11 and Wayland.
  - **macOS**: Screen Recording permission may be required for window titles
  - **Windows**: No additional permissions needed

## Setup

### Using Docker for Database (Recommended)
1. Clone or copy the code.
2. Create a `.env` file in the project root (copy from `.env.example` and fill in your values):
    ```
    POSTGRES_USERNAME=your_actual_username
    POSTGRES_PASSWORD=your_actual_password
    DATABASE_URL=postgres://your_actual_username:your_actual_password@localhost:5432/time_tracker
    ```
3. Start PostgreSQL: `docker-compose up -d postgres`
4. Build the app: `cargo build --release`
5. Run: `./target/release/time_tracker`
6. To stop: `docker-compose down`

### Using Local PostgreSQL
1. Install and start PostgreSQL locally.
2. Create database: `CREATE DATABASE time_tracker;`
3. Create a `.env` file with your local credentials.
4. Build: `cargo build --release`
5. Run: `./target/release/time_tracker`

### Windows Setup
1. Install Rust from https://rustup.rs/
2. Install Docker Desktop for Windows
3. Follow the Docker database setup above
4. For window/app detection, the app uses Windows API and typically requires no special permissions. If detection fails, try running the terminal as administrator or check Windows Defender/Firewall settings.
5. Build: `cargo build --release`
6. Run: `.\target\release\time_tracker.exe`

### macOS Setup
1. Install Rust from https://rustup.rs/
2. Install Docker Desktop for Mac
3. Follow the Docker database setup above
4. For window title detection, grant Screen Recording permission to Terminal in System Preferences > Security & Privacy > Privacy > Screen Recording
5. Build: `cargo build --release`
6. Run: `./target/release/time_tracker`

## Usage
The app provides a terminal-based interface for time tracking with an interactive dashboard.

### Commands
- **Tab**: Switch between dashboard views (Daily/Weekly/Monthly/History)
- **Shift+C**: Open commands popup menu with all available shortcuts
- **r**: Rename apps/tabs (arrow keys to navigate, Enter to select)
- **e**: End the current session (saves to database)
- **m**: Manually set app name (if auto-detection fails)
- **u**: Update current app detection
- **l**: View application logs
- **q**: Quit the application

**Note**: The app starts tracking automatically when launched and displays visual analytics with bar charts and detailed statistics.

Sessions automatically track the active application and duration. Data is saved to Postgres every 10 minutes automatically, or manually when ending a session.

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
Modify individual services in their respective directories. Ensure changes maintain the modular structure and add appropriate tests.
