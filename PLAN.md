# Time Tracker App Development Plan

## Overview
Build a production-ready Rust application for tracking time spent on work sessions, monitoring active applications. Uses Ratatui for terminal UI and PostgreSQL 18 for persistent storage.

## Architecture
- **Main Entry**: `src/main.rs` - Application entry point, event loop, and service orchestration
- **Services** (each in own directory with `mod.rs`):
  - `database/` - PostgreSQL connection, migrations, CRUD operations
  - `tracker/` - Application monitoring, session management
  - `ui/` - Ratatui-based terminal interface
  - `config/` - Configuration management (env vars, settings)
  - `models/` - Data structures and types
  - `utils/` - Helper functions and utilities

## Dependencies (added via `cargo add`)
- `ratatui` - Terminal UI framework
- `crossterm` - Terminal manipulation
- `tokio` - Async runtime
- `sqlx` - PostgreSQL driver with compile-time verification
- `chrono` - Date/time handling
- `serde` - Serialization
- `anyhow` - Error handling
- `clap` - CLI argument parsing (if needed)
- `dotenvy` - Environment variable loading

## Database Schema
```sql
CREATE TABLE sessions (
    id SERIAL PRIMARY KEY,
    app_name TEXT NOT NULL,
    start_time TIMESTAMP WITH TIME ZONE NOT NULL,
    duration BIGINT NOT NULL -- in seconds
);
```

## Core Features
1. **Session Management**
   - Start/end work sessions
   - Track active applications during sessions
   - Automatic hourly saves
   - Manual save on session end

2. **Application Monitoring**
   - Detect currently active/focused applications
   - Cross-platform support (Linux/macOS/Windows)
   - Efficient polling without high CPU usage

3. **Terminal UI**
   - Real-time session display
   - History viewer
   - Keyboard shortcuts (s/e/v/q)
   - Status indicators

4. **Data Persistence**
   - PostgreSQL storage
   - Connection pooling
   - Migration handling
   - Query optimization

## Implementation Steps
1. Initialize project structure and dependencies
2. Implement database service (connection, schema)
3. Create data models and types
4. Build application tracker service
5. Develop UI components with Ratatui
6. Integrate services in main.rs
7. Add configuration management
8. Implement testing
9. Performance optimization
10. Documentation and README updates

## Testing Strategy
- Unit tests for each service
- Integration tests for database operations
- UI interaction tests
- Cross-platform compatibility tests

## Deployment Considerations
- Single binary executable
- Environment-based configuration
- Graceful error handling
- Logging for debugging

## Maintenance
- Modular design for easy feature additions
- Clear separation of concerns
- Comprehensive error handling
- Performance monitoring