# Changelog

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