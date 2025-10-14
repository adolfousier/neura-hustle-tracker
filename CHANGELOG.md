# Changelog

## v0.1.3
- Added interactive dashboard with multiple views: Daily, Weekly, Monthly, and History
- Implemented Tab key navigation to switch between dashboard views
- Added visual bar charts for usage statistics with color-coded displays
- Created app/tab renaming feature with arrow key navigation (press 'r')
- Removed session renaming (sessions auto-track, only apps/tabs can be renamed)
- Added database method to rename all sessions for a specific app at once
- Enhanced UI with detailed stats showing total hours and minutes
- Improved session persistence on exit and crash scenarios
- Updated commands display to show [Tab] for view switching

## v0.1.2
- Added logging functionality: press 'l' to view application logs
- Changed auto-save interval from 1 hour to 10 minutes for more frequent data persistence
- Improved error handling throughout the application
- Added database method to update session names
- Modified session saving to skip sessions shorter than 10 seconds
- Added manual app name setting feature
- Enhanced UI with better display of session information and usage statistics

## v0.1.0 (Initial Release)
- Initial implementation: TUI for sessions, Postgres storage, app monitoring.
- Dependencies added via cargo add.
- Modular structure with services.
- Auto-save sessions every hour.
- Manual start/end/view commands.
- Cross-platform app monitoring (placeholder implementation).