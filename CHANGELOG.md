# Changelog

## v0.3.0 (2025-10-18)

- **Manual Category Management**: New 'c' command allows users to manually assign and create custom categories for apps
  - Press 'c' to enter category selection mode
  - Select app to categorize from usage list (shows current category)
  - Choose from predefined categories or create custom ones
  - Categories persist across all sessions for that app
  - Real-time UI updates throughout dashboard, history, and breakdowns
  - Custom category creation with emoji support (e.g., "🎮 Gaming", "🎨 Design")
- **Database Enhancements**:
  - Added `update_app_category()` method for bulk category updates
  - Category changes apply to all past and future sessions of an app
- **UI Improvements**:
  - New `SelectingCategory` state for app selection in category mode
  - New `CategoryMenu` state for category selection with visual indicators
  - Updated commands menu to include '[c] Change app category'
  - Category display shows with color coding in selection view
  - Status bar messages for category workflow
- **Code Organization**:
  - Moved daemon files to `src/utils/` directory structure
  - Updated module paths for better organization
  - Prepared groundwork for future command refactoring to `src/ui/commands/`

## v0.2.8 (2025-10-17)

- **Background Daemon Architecture**: Implemented separate daemon mode for macOS/Windows to solve TUI tracking interference issue
  - Created `neura_hustle_daemon` binary that runs silently in background tracking all apps
  - Created `neura_hustle_tracker` TUI binary for viewing stats (can open/close without affecting tracking)
  - Solves fundamental issue: TUI can't track other apps when IT is the focused window
  - Daemon polls active window every 100ms (same as unified mode for real-time accuracy)
  - Daemon auto-saves sessions hourly + on app switch + graceful shutdown (SIGTERM/SIGINT)
- **Platform-Specific Modes**:
  - **Linux X11**: Unified mode works perfectly - TUI + tracking in one process (no changes, no additional requirements)
  - **Linux Wayland**: Unified mode works perfectly - **REQUIRES** [Window Calls GNOME extension](https://extensions.gnome.org/extension/4724/window-calls/) for window tracking
  - **macOS/Windows**: Daemon mode recommended - background tracking + separate TUI viewer
  - Both modes available on all platforms - user chooses at runtime which binary to run
- **New Makefile Commands**:
  - `make daemon-start` - Start background tracking daemon (starts DB, builds daemon, runs in background)
  - `make daemon-stop` - Stop background tracking daemon
  - `make daemon-status` - Check if daemon is running (shows PID and log location)
  - `make view` - Open TUI to view stats (warns if daemon not running)
  - `make build-daemon` - Build only the daemon binary
  - Updated `make build` to build only TUI binary (for faster incremental builds)
  - Updated `make help` with clear Linux vs macOS/Windows command guidance
- **One-Liner Installation Updates**:
  - **Linux**: Creates `hustle` alias (unified mode - works perfectly)
  - **macOS**: Creates `hustle-start`, `hustle-stop`, `hustle-view`, `hustle-status` aliases (daemon mode)
  - **Windows**: Creates `hustle-start`, `hustle-stop`, `hustle-view`, `hustle-status` functions (daemon mode)
  - Removed comments from .env generation in one-liners (comments broke execution)
- **README Enhancements**:
  - Added "Which Mode Should I Use?" decision guide section
  - Clear explanation of why Linux uses unified mode (works perfectly) vs macOS/Windows daemon mode (TUI interference)
  - Updated Platform-Specific Notes with daemon mode instructions for macOS/Windows
  - Added Wayland extension requirement documentation for Linux users
  - Clarified that Linux tracking is 100% accurate in unified mode
  - Updated daemon mode documentation with clear start/stop/view workflow
- **Code Organization**:
  - Added `src/active_window/daemon.rs` - Background tracking daemon (268 lines)
  - Added `src/utils/daemon_main.rs` - Daemon entry point with logging setup
  - Added `src/active_window/mod.rs` - Module declaration
  - Updated `Cargo.toml` to build two separate binaries
  - Added `daemon.log` and `daemon.pid` to .gitignore
  - Daemon writes to `daemon.log` when DEBUG_LOGS_ENABLED=true (separate from TUI's app.log)

## v0.2.7 (2025-10-16)

- **Fixed Cross-Platform Detection**: Properly detect and log macOS, Windows, and Linux operating systems
  - Platform detection now logged at startup (macOS/Windows/Linux)
  - Platform-specific API labels in logs (Cocoa/AppKit for macOS, Win32 for Windows, X11/Wayland for Linux)
  - Fixed misleading "X11" labels that appeared on macOS and Windows
  - `is_wayland()` now only runs on Linux systems
- **Fixed App Name Normalization**: Linux-specific name fixes no longer applied to macOS/Windows
  - macOS Terminal.app no longer incorrectly renamed to "gnome-terminal"
  - Wayland dot/underscore splitting (e.g., "org.gnome.Nautilus") only applied on Linux
  - Linux-specific apps (nautilus, gedit, rhythmbox, etc.) only detected on Linux
  - Cross-platform apps (Chrome, Firefox, VS Code, Slack, etc.) still work on all platforms
- **Enhanced Debug Logging**: Comprehensive platform-specific logging for troubleshooting
  - Shows raw window data: app_name, title, process_path, position
  - Logs original vs normalized app names to see transformation
  - Platform-specific debug output with OS-specific details
  - Error messages now show relevant environment variables per platform
- **Optional Debug Logging**: Add `DEBUG_LOGS_ENABLED=true` to .env to enable detailed logging
  - Regular users don't generate app.log files (cleaner experience)
  - Debug mode writes comprehensive logs to app.log
  - Debug logs include platform detection, window detection, and app name normalization
  - Disabled by default to avoid unnecessary file generation
- **Platform-Specific Error Messages**: Accurate error guidance for each OS
  - macOS errors show DISPLAY, PATH, HOME environment variables
  - Windows errors show USERPROFILE, COMPUTERNAME
  - Linux errors properly handle Wayland vs X11 detection
  - All error messages point users to enable DEBUG_LOGS_ENABLED for troubleshooting

## v0.2.6 (2025-10-16)

- **Migration Note for v0.2.5 Database Rename**: Database name was changed from 'time_tracker' to 'hustle-tracker' in v0.2.5
  - **For existing users who want to keep old database name**: Update your `compose.yml` POSTGRES_DB and `.env` DATABASE_URL to use your current database name (e.g., `time_tracker`)
  - **For existing users who want to rename to hustle-tracker**:
    1. Stop the database: `docker compose down`
    2. Start database: `docker compose up -d`
    3. Connect and rename: `docker exec -it neura-hustle-tracker-postgres-1 psql -U <your_username> -c "ALTER DATABASE time_tracker RENAME TO \"hustle-tracker\";"`
    4. Update `.env` DATABASE_URL to use `/hustle-tracker` instead of `/time_tracker`
    5. Restart app with `cargo run` or `make run`
- **Global Command Alias**: One-liner installers now create a global `hustle` command that works from any directory
  - Linux: Adds alias to `~/.bashrc` and sources it automatically
  - macOS: Adds alias to `~/.zshrc` and sources it automatically
  - Windows: Adds PowerShell function to `$PROFILE` and loads it automatically
  - After installation, simply type `hustle` from anywhere to start the app
- **Fixed TUI Corruption Bug**: Logs now write to `app.log` file instead of stderr, preventing log messages from corrupting the terminal UI
- **Logging Improvements**: Changed default log level from debug to info for cleaner logs, reduced zbus verbosity
- **Better UX**: `cargo run` now works cleanly without requiring stderr redirection (`2> debug.log`)

## v0.2.5 (2025-10-16)

- **DB Connection**: Fixed query and db generation for universal builds
- **Database Rename**: Renamed database from 'time_tracker' to 'hustle-tracker' to match app name
- **Configuration Updates**: Updated compose.yml, src/config/settings.rs, and .env to use new database name
- **Docker Volume**: Ensured postgres service uses correct volume for data persistence

## v0.2.4 (2025-10-16)

- **Enhanced Session Tracking**: Added comprehensive metadata fields for browser (URL, title, notifications), terminal (user, host, directory, project), editor (file, path, language), multiplexer (tmux), and IDE tracking
- **Advanced Analytics Dashboard**: Implemented breakdown views for browser usage, project tracking, file editing, terminal sessions, and categories with dedicated popup accessible via 'b' key
  - Beautiful grid layout (2x3) showing all breakdown categories at once
  - View-mode aware: breakdown data automatically matches Daily/Weekly/Monthly view selection
  - In-memory aggregation from filtered session history for optimal performance
- **Full History Views**: History popup ('h' key) now shows complete session history for Daily/Weekly/Monthly view modes, not just the current day
- **Database Schema Expansion**: Added 20+ new columns to sessions table with JSONB metadata storage and parsing success tracking
- **New Parser Module**: Introduced intelligent window title parsing for extracting detailed context from applications
  - Detects 20+ web services (Gmail, GitHub, Slack, YouTube, etc.) from browser titles
  - Extracts terminal session info (user@host:/path, project detection)
  - Parses editor context (filename, language, project path)
  - Identifies tmux/screen multiplexers
- **UI Refinements**:
  - Replaced History view mode with scrollable HistoryPopup ('h' key)
  - Removed obsolete manual app commands ('m', 'u') for cleaner interface
  - Fixed CommandsPopup preserving view mode (was hardcoded to Daily)
  - Cleaned app names by removing "gnome-" prefix for better readability
- **Performance Improvements**:
  - Removed synchronous wrappers, fully async API
  - Optimized logging configuration to reduce zbus verbosity
  - In-memory breakdown aggregation instead of multiple database queries
- **Code Cleanup**:
  - Removed unused database breakdown methods (replaced by in-memory aggregation)
  - Enhanced error handling and improved test coverage
  - Updated all tests to use async/await patterns

## v0.2.3 (2025-10-16)

- **Enhanced History Viewing**: Replaced History view mode with a dedicated scrollable HistoryPopup accessible via 'h' key, supporting ↑/↓/PgUp/PgDn navigation
- **Simplified Interface**: Removed manual app name setting ('m') and update current app detection ('u') commands for cleaner UX
- **Improved Session Persistence**: Removed session combining logic - now saves all sessions regardless of duration for complete activity tracking
- **Async Code Cleanup**: Removed synchronous wrapper methods in AppMonitor, updated tests to use async/await pattern
- **Better Logging**: Improved logging configuration to reduce zbus verbosity and removed debug DATABASE_URL prints
- **Code Cleanup**: Removed unused update_session_duration database method and added server.log to .gitignore
- **Updated Documentation**: Refreshed README with new command shortcuts and interface changes

## v0.2.2 (2025-10-16)

- **Fixed Midnight Boundary Tracking**: Corrected daily activity tracking to properly reset at 00:00 and include all activity since midnight
- Updated all time-based queries to use `date_trunc('day', CURRENT_TIMESTAMP)` for timezone-aware midnight boundary detection
- Fixed daily, weekly, and monthly usage queries to accurately track sessions from 00:00 onwards
- Enhanced session retrieval methods to ensure activities logged after midnight (e.g., 02:14 AM) are correctly included in today's data
- Improved data consistency with proper PostgreSQL timestamp handling across all dashboard views

## v0.2.1 (2025-10-15)

- **Fixed startup scrip .env Loading**: Corrected credential loading to use project root directory, preventing generation from startup scripts or different working directories
- **Data Recovery**: Restored lost database data by switching to correct Docker volume (neura-hustle_tracker_postgres_data)
- **Cross-Platform Startup**: Ensured startup scripts work consistently across Windows, macOS, and Linux by loading .env from cwd
- **No Overwrite Policy**: Implemented protection against overwriting existing .env files; only generates if missing
- **README Updates**: Added CONTRIBUTING.md, updated contributing section, added license section
- **Build Fixes**: Resolved path issues for reliable .env detection and database connections

## v0.2.0 (2025-10-15)

- **Wayland AFK Detection**: Implemented D-Bus based idle time monitoring for Wayland systems
- Added dual-mode AFK detection: GNOME Session Manager D-Bus interface for Wayland, rdev library for X11
- Enhanced Wayland compatibility with automatic session type detection and appropriate input monitoring
- Improved error handling for D-Bus connection failures with graceful fallback mechanisms
- Added `uses_wayland()` method to AppMonitor for session type detection
- **Fixed .env Loading**: Corrected credential loading to use project root directory, preventing generation from startup scripts or different working directories
- **Data Recovery**: Restored lost database data by switching to correct Docker volume (neura-hustle-tracker_postgres_data)
- **Cross-Platform Startup**: Ensured startup scripts work consistently across Windows, macOS, and Linux by loading .env from cwd
- **No Overwrite Policy**: Implemented protection against overwriting existing .env files; only generates if missing
- **README Updates**: Added CONTRIBUTING.md, updated contributing section, added license section
- **Build Fixes**: Resolved path issues for reliable .env detection and database connections

- **Wayland AFK Detection**: Implemented D-Bus based idle time monitoring for Wayland systems
- Added dual-mode AFK detection: GNOME Session Manager D-Bus interface for Wayland, rdev library for X11
- Enhanced Wayland compatibility with automatic session type detection and appropriate input monitoring
- Improved error handling for D-Bus connection failures with graceful fallback mechanisms
- Added `uses_wayland()` method to AppMonitor for session type detection

## v0.1.9 (2025-10-15)

- **Real-Time Activity Progress Bars**: Replaced timeline chart with dynamic progress bars showing percentage of day for each app
- Implemented clean app name display by removing "gnome-" prefixes for better readability
- Added automatic database migration to fix historical categorization data corrupted from previous versions
- Fixed category preservation when renaming apps - renamed apps maintain their original categories instead of defaulting to "Other"
- Enhanced visual design with proper top margins in all dashboard cards for consistent spacing
- Improved progress bar layout with percentage display and clean visual design

## v0.1.8 (2025-10-15)

- **Native Wayland Support**: Full support for GNOME Wayland sessions via D-Bus integration
- Added automatic Wayland/X11 detection - app intelligently switches between backends
- Integrated with [Window Calls GNOME Extension](https://extensions.gnome.org/extension/4724/window-calls/) for native Wayland window tracking
- Implemented dual-mode window detection: D-Bus for Wayland, active-win-pos-rs for X11/Windows/macOS
- Added `zbus` and `serde_json` dependencies for D-Bus communication
- Enhanced error messages to guide users through Wayland setup requirements
- **Automated Wayland Setup Check**: Makefile now detects Wayland sessions and verifies extension installation
- Added `check-wayland` target that validates Window Calls extension on Wayland systems
- Improved user experience with helpful setup instructions when extension is missing
- Updated README with comprehensive Wayland setup instructions
- Separated Linux setup into X11 and Wayland sections with specific requirements
- Cross-platform compatibility maintained: X11, Wayland (GNOME), Windows, macOS
- Enhanced app name normalization for Wayland's different wm_class format (e.g., org.gnome.Nautilus, firefox_firefox)
- Added support for Alacritty and Nautilus in app name detection
- Improved logging with platform-specific detection messages (Wayland vs X11)

## v0.1.7 (2025-10-15)

- **Zero-Configuration Onboarding**: Database credentials now auto-generate on first run
- Implemented automatic credential generation system with secure random passwords
- Added auto-detection for missing .env file - creates one automatically with secure credentials
- Enhanced settings module to check and generate credentials when DATABASE_URL is not set
- Updated .env.example with comprehensive documentation about auto-generation feature
- **Cross-Platform Makefile**: Complete rewrite with platform detection for Windows, macOS, and Linux
- Added `make run` command - single command to start DB, build release binary, and run app
- Added `make dev` command for quick development with debug builds
- Added `make help` command showing all available commands with descriptions
- Makefile now automatically detects OS and uses correct binary paths
- **Simplified README**: Completely overhauled setup documentation
- Featured "One Command Setup" approach for new users (`make run` or manual one-liner)
- Added platform-specific setup sections for Windows, macOS, and Linux
- Clear warnings about needing to `cd` into project directory before running commands
- Included actual repository URL (<https://github.com/adolfousier/neura-hustle-tracker>) in all examples
- Emphasized use of `cargo build --release` for optimized production builds
- Moved advanced configuration options to separate clearly-marked sections
- Added `rand` dependency for cryptographically secure credential generation
- Non-technical users can now run the app without manual .env file creation
- Credentials format: username `timetracker_<8-random-chars>`, password 32-char alphanumeric
- Auto-generated .env file includes helpful comments explaining the credentials

## v0.1.6 (2025-10-15)

- Fixed UI to update timeline, session history, and categories when switching between Daily/Weekly/Monthly views
- Added database methods for retrieving sessions by time period (daily, weekly, monthly)
- Improved view-specific data loading for better performance and accuracy
- Added hourly auto-save feature (saves sessions every 1 hour automatically)
- Implemented signal handlers for graceful crash recovery (SIGTERM, SIGINT)
- Sessions now automatically save on quit and on crash/signal interruption
- Removed 'e' end session command - simplified workflow where users only quit when done
- Enhanced data persistence with automatic save intervals and crash protection

## v0.1.5 (2025-10-14)

- Added AFK (Away From Keyboard) detection feature with real-time status display
- Implemented global keyboard and mouse activity monitoring using rdev library
- Added "AFK Status" card to the UI showing current status, idle time, and activity detection
- Positioned AFK card adjacent to Timeline in horizontal layout (50/50 split) and below in vertical layout
- AFK threshold set to 5 minutes of inactivity
- Cross-platform input event listening for Windows, macOS, and Linux
- Enhanced UI responsiveness with adaptive AFK card sizing
- Implemented cross-platform support for Linux (X11/Wayland), Windows, and macOS using active-win-pos-rs library
- Replaced Linux-specific GNOME extension with universal window detection for better cross-platform compatibility
- Updated README with detailed setup instructions for Windows and macOS, including permission requirements
- Added platform-specific prerequisites and supported platforms section
- Restored and enhanced Features section in README highlighting cross-platform support and full responsiveness
- Added app categorization system with automatic category assignment (Development, Browsing, Communication, Media, Files, Other)
- Implemented commands popup menu accessible via Shift+C for better discoverability
- Added adaptive responsive layout that adjusts to terminal size (vertical layout for small terminals, horizontal for large ones)
- Enhanced UI with centered input dialogs and improved visual design
- Added category column to database schema with migration support
- Improved bar chart with category-based colors and better scaling
- Enhanced timeline visualization with category colors and adaptive sizing
- Added comprehensive commands menu with all available shortcuts
- Improved app selection interface with better navigation and styling
- Added support for storing and retrieving app categories in session data

## v0.1.4 (2025-10-14)

- Complete dashboard redesign with comprehensive data visualization
- Added pie chart showing app usage categories (Development, Browsing, Communication, Media, Files, Other)
- Added color-coded timeline showing recent activity patterns
- Improved layout: 50/50 split with bar chart, timeline, stats on left; history and categories on right
- Fixed "Other" category color to gray for better visibility
- Optimized layout proportions for perfect visual alignment between sections
- Fixed history loading to display 30 recent sessions instead of 10
- Enhanced rename operation to refresh all usage data (daily, weekly, monthly, history)
- Improved session history to always show updated renamed app names
- Added sorted category display using BTreeMap for consistent ordering
- Better integration of all dashboard components for comprehensive view

## v0.1.3 (2025-10-14)

- Added interactive dashboard with multiple views: Daily, Weekly, Monthly, and History
- Implemented Tab key navigation to switch between dashboard views
- Added visual bar charts for usage statistics with color-coded displays
- Created app/tab renaming feature with arrow key navigation (press 'r')
- Removed session renaming (sessions auto-track, only apps/tabs can be renamed)
- Added database method to rename all sessions for a specific app at once
- Enhanced UI with detailed stats showing total hours and minutes
- Improved session persistence on exit and crash scenarios
- Updated commands display to show [Tab] for view switching

## v0.1.2 (2025-10-13)

- Added logging functionality: press 'l' to view application logs
- Changed auto-save interval from 1 hour to 10 minutes for more frequent data persistence
- Improved error handling throughout the application
- Added database method to update session names
- Modified session saving to skip sessions shorter than 10 seconds
- Added manual app name setting feature
- Enhanced UI with better display of session information and usage statistics

## v0.1.0 (Initial Release) (2025-10-13)

- Initial implementation: TUI for sessions, Postgres storage, app monitoring.
- Dependencies added via cargo add.
- Modular structure with services.
- Auto-save sessions every hour.
- Manual start/end/view commands.
- Cross-platform app monitoring (placeholder implementation).
