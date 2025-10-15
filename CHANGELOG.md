# Changelog

## v0.2.0 (2025-10-15)
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
- Included actual repository URL (https://github.com/adolfousier/neura-hustle-tracker) in all examples
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
