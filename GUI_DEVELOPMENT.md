# GUI Development Guide

## Overview

A modern desktop GUI is in development using the **GPUI framework** (Zed editor's UI toolkit). The GUI provides real-time activity tracking with a visual dashboard showing daily, weekly, and monthly statistics.

## Building the GUI

### Prerequisites

**Linux (X11/Wayland):**
```bash
sudo apt install libxkbcommon-dev libxcb1-dev libfreetype6-dev
```

**macOS:**
- Xcode Command Line Tools (handles dependencies automatically)

**Windows:**
- Visual Studio Build Tools or similar

### Build Instructions

```bash
# Build release binary
cargo build --release --bin neura_hustle_gui

# Run the GUI
./target/release/neura_hustle_gui
```

The binary will be located at `target/release/neura_hustle_gui`.

## Running the GUI

### Requirements

1. **Database**: PostgreSQL must be running
   ```bash
   make db-up
   ```

2. **Display Server** (Linux only):
   - X11 or Wayland session
   - For Wayland: GNOME Shell "Window Calls" extension must be installed:
     https://extensions.gnome.org/extension/4724/window-calls/

3. **Permissions** (macOS only):
   - Grant Terminal app Accessibility permissions in System Preferences
   - Grant Input Monitoring permissions

### Start the GUI

```bash
./target/release/neura_hustle_gui
```

Or run directly from cargo:
```bash
cargo run --release --bin neura_hustle_gui
```

## Features

### Dashboard Views
- **Daily**: Today's activity breakdown by app
- **Weekly**: Last 7 days of usage
- **Monthly**: Last 30 days of usage
- **Sessions**: Detailed session history with timestamps, apps, window titles, and duration
- **Breakdown**: Detailed view of browser tabs, terminals, editors, and projects

### Tracking
- **Automatic**: Runs in background, tracks active window/app changes
- **Real-time**: Green indicator shows when tracking is active
- **Window/Tab tracking**: Displays not just apps (Firefox) but specific tabs/windows
- **AFK detection**: Marks sessions as AFK after 5 minutes of inactivity
- **Session data**: Each session records:
  - App name and window title
  - Start time and duration
  - Category (auto-detected or custom)
  - AFK status

### Statistics
- Total sessions and time spent per app
- Category breakdown (Development, Browsing, Communication, etc.)
- Browser tab/project breakdown
- Terminal directory tracking
- Editor file tracking

## Architecture

### Components

**Main entry**: `src/gui_main.rs`
- Initializes database connection
- Spawns background tracking task
- Spawns periodic stats reload task
- Initializes GPUI application

**App state**: `src/gui/app.rs`
- `GuiApp`: Main application controller
- `GuiAppState`: Shared state (RwLock-protected)
- Manages stats, session history, and current tracking info

**Tracking loop**: Background tokio task in `gui_main.rs`
- Polls active window every 500ms via `AppMonitor`
- Detects app/window changes
- Saves sessions to database
- Updates GuiApp state for UI rendering

**UI rendering**: `src/gui_main.rs` (DashboardView)
- GPUI-based components
- Reads from GuiApp state
- Displays stats, sessions, and tracking status

### Key Data Flow

```
Active Window (Monitor)
         ↓
   AppMonitor.get_active_app_async()
         ↓
   Background tracking task
         ↓
   ├→ Database.insert_session() [DB write]
   └→ GuiApp.state.current_session [UI update]
         ↓
   DashboardView reads state
         ↓
   UI renders with latest info
```

### Platform-Specific Details

**Linux/Wayland**:
- Uses D-Bus to query GNOME Shell Extension "Window Calls"
- Extension provides window information (app, title, focus status)
- Requires JSON deserialization from D-Bus response

**Linux/X11**:
- Uses X11 APIs via `active-win-pos-rs` crate
- Direct window property queries

**macOS**:
- Uses Cocoa/AppKit APIs via `active-win-pos-rs`
- AppleScript fallback available

**Windows**:
- Uses Win32 APIs via `active-win-pos-rs`
- PowerShell for process tree inspection

### Window Title Handling

The GUI tracks **window names** (which represent browser tabs, file paths, etc.):
- Session display shows: "firefox - GitHub Issues" (not just "firefox")
- Allows tracking specific projects, websites, files
- Stored in `Session.window_name` column

## Development Notes

### Known Issues & TODOs

1. **Wayland null values**: Some windows return null `wm_class` values from Extension
   - Handled by using Option<String> and defaulting to "Unknown"
   - Consider filtering Unknown apps in UI

2. **GUI responsiveness**: Large stat calculations might block render
   - Stats reload every 5 seconds (could be optimized)

3. **Dead code warnings**: Many database methods from TUI not used in GUI
   - These are for future features (categorization, renaming, etc.)

4. **Logger output**: Debug logs go to gui.log but may not capture all async task output
   - Use DEBUG_LOGS_ENABLED=true in .env to enable file logging

### Code Style

- Follow existing Rust conventions
- Use `log::info!()` for tracking events (not eprintln!)
- Minimize allocations in render loop
- Use Arc<RwLock> for shared state, try_read() for non-blocking UI reads

### Testing

```bash
# Build debug binary (faster iteration)
cargo build --bin neura_hustle_gui

# Run with debug logging
DEBUG_LOGS_ENABLED=true ./target/debug/neura_hustle_gui

# View logs
tail -f gui.log
```

## Troubleshooting

### GUI starts but shows "Not tracking"
- Check AppMonitor logs for window detection errors
- Verify GNOME Extension is installed (Wayland)
- Check permissions (macOS Accessibility)

### Wayland "invalid type: null" errors
- Normal - some windows don't have wm_class
- Fixed by using Option<String> with defaults

### Sessions show "Unknown" app
- Wayland Extension returned null wm_class
- Usually resolves on next window focus change

### Stats not updating
- Check database connection (make db-up)
- Verify ~/.env DATABASE_URL is correct
- Check gui.log for insert_session errors

### High CPU usage
- Background tracking task polls every 500ms (design choice for responsiveness)
- Could optimize with event-based detection instead of polling

## Contributing

### Adding Features

1. **New stat type**: Add to `TimePeriodStats` struct, update `fetch_period_stats_with_breakdown()`
2. **New dashboard view**: Add to `ViewMode` enum, implement in `DashboardView::render()`
3. **Categorization UI**: Add inputs to session rendering, call database categorize methods
4. **Settings**: Store in config, pass to tracking task via Arc<Mutex<>> or channels

### Testing Changes

Always test with:
- Both daily and weekly views
- Multiple app switches
- Window title changes (tabs)
- Long idle periods (AFK detection)
- System suspend/resume

## Resources

- GPUI docs: https://github.com/zed-industries/zed/tree/main/crates/gpui
- Active window detection: https://github.com/KitsuneNGE/active-win-pos-rs
- GNOME Shell Extensions: https://extensions.gnome.org/
