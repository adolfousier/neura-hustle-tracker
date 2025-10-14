# Neura Husle Tracker Linux x11 App

A Rust-based time-tracking tool for monitoring app usage during work sessions. Built with Ratatui for the UI and Postgres 18 for history.

![Demo](src/screenshots/demo.png)

## Prerequisites
- **Desktop Environment**: This app requires a GUI desktop environment (GNOME, KDE, etc.) to detect active applications. It does not work in terminal-only or headless environments.
- Rust 1.90+
- Docker and Docker Compose (for easy Postgres setup)
- For X11: xdotool (`sudo apt-get install xdotool`)
- For Wayland/GNOME: GNOME Shell extension (installed automatically by install-extension.sh)

## Setup

### Option 1: Using Docker for DB + Host App (Recommended)
1. Clone or copy the code.
2. Install dependencies:
   - For X11: `sudo apt-get install xdotool`
   - For Wayland/GNOME: Run `./install-extension.sh`, then **restart GNOME Shell** by pressing Alt+F2, typing `r`, and pressing Enter
3. Create a `.env` file in the project root (copy from `.env.example` and fill in your values):
   ```
   POSTGRES_USERNAME=your_actual_username
   POSTGRES_PASSWORD=your_actual_password
   DATABASE_URL=postgres://your_actual_username:your_actual_password@localhost:5432/time_tracker
   ```
4. Start PostgreSQL: `docker-compose up -d postgres`
5. Build the app: `cargo build --release`
6. Run on host: `./target/release/time_tracker`
7. To stop: `docker-compose down`

### Option 2: Local PostgreSQL
1. Install and start PostgreSQL 18 locally.
2. Create database: `CREATE DATABASE time_tracker;`
3. Create a `.env` file with your local credentials.
4. Build: `cargo build --release`
5. Run: `./target/release/time_tracker`

## Usage
The app provides a terminal-based interface for time tracking with an interactive dashboard.

### Commands
- **Tab**: Switch between dashboard views (Daily/Weekly/Monthly/History)
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
- `tracker/`: Application monitoring
- `ui/`: Ratatui-based terminal interface
- `config/`: Configuration management
- `models/`: Data structures
- `utils/`: Helper utilities

## Testing
Run `cargo test` to execute unit tests for database operations and core functionality.

## Contributing
Modify individual services in their respective directories. Ensure changes maintain the modular structure and add appropriate tests.
