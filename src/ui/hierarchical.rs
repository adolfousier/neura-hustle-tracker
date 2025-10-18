use std::collections::BTreeMap;
use crate::models::session::Session;

/// Extract project name from directory path with improved heuristics
fn extract_project_name(path: &str) -> Option<String> {
    // Handle home directory specially
    if let Ok(home) = std::env::var("HOME") {
        if path == home || path == "~" {
            return Some("Home".to_string());
        }
        if path.starts_with(&format!("{}/", home)) {
            // Extract the first directory after home (e.g., ~/Documents -> Documents)
            let after_home = &path[home.len() + 1..];
            if let Some(slash_pos) = after_home.find('/') {
                let first_dir = &after_home[..slash_pos];
                if !first_dir.is_empty() {
                    return Some(first_dir.to_string());
                }
            } else if !after_home.is_empty() {
                return Some(after_home.to_string());
            }
        }
    }

    // Standard project extraction from path
    let parts: Vec<&str> = path.split('/').collect();

    // Get last non-empty component, skipping common non-project directories
    let skip_dirs = ["bin", "usr", "etc", "var", "tmp", "dev", "proc", "sys", "home", "root"];

    for part in parts.iter().rev() {
        let part_lower = part.to_lowercase();
        if !part.is_empty() && *part != "." && *part != ".." && !skip_dirs.contains(&part_lower.as_str()) {
            // Additional heuristics: prefer directories that look like projects
            if part.chars().next().map_or(false, |c| c.is_alphabetic()) && part.len() >= 2 {
                return Some(part.to_string());
            }
        }
    }

    // Fallback: if we have any valid directory component
    for part in parts.iter().rev() {
        if !part.is_empty() && *part != "." && *part != ".." {
            return Some(part.to_string());
        }
    }

    None
}
/// Creates hierarchical usage data from sessions for display in stats
/// Format: App entries with sub-entries indented with "  └─ "
pub fn create_hierarchical_usage(sessions: &[Session]) -> Vec<(String, i64)> {
    // Group sessions by app, then by window details
    let mut app_map: BTreeMap<String, BTreeMap<String, i64>> = BTreeMap::new();

    for session in sessions {
        // Skip AFK sessions
        if session.is_afk.unwrap_or(false) {
            continue;
        }

        let app_name = session.app_name.clone();

        // Create meaningful sub-entry based on session data
        let sub_entry = if let Some(page_title) = &session.browser_page_title {
            // Browser: show page title
            page_title.clone()
        } else if let Some(dir) = &session.terminal_directory {
            // Terminal: show directory
            dir.clone()
        } else if let Some(filename) = &session.editor_filename {
            // Editor: show file
            if let Some(lang) = &session.editor_language {
                format!("{} ({})", filename, lang)
            } else {
                filename.clone()
            }
        } else if let Some(tmux_window) = &session.tmux_window_name {
            // Tmux: show window name
            format!("tmux: {}", tmux_window)
        } else if let Some(window) = &session.window_name {
            // Fallback: show window name
            window.clone()
        } else {
            // No sub-entry
            String::new()
        };

        let app_sessions = app_map.entry(app_name).or_insert_with(BTreeMap::new);
        if !sub_entry.is_empty() {
            *app_sessions.entry(sub_entry).or_insert(0) += session.duration;
        } else {
            *app_sessions.entry("(general)".to_string()).or_insert(0) += session.duration;
        }
    }

    // Flatten into hierarchical display format
    let mut result: Vec<(String, i64)> = Vec::new();

    // Sort apps by total duration
    let mut app_totals: Vec<(String, i64)> = app_map.iter()
        .map(|(app, sessions)| (app.clone(), sessions.values().sum()))
        .collect();
    app_totals.sort_by(|a, b| b.1.cmp(&a.1));

    for (app_name, app_total) in app_totals {
        // Add app header
        result.push((app_name.clone(), app_total));

        // Add sub-entries (top 3 by duration)
        if let Some(sessions) = app_map.get(&app_name) {
            let mut session_list: Vec<(String, i64)> = sessions.iter()
                .map(|(sub, dur)| (sub.clone(), *dur))
                .collect();
            session_list.sort_by(|a, b| b.1.cmp(&a.1));

            for (sub_entry, duration) in session_list.iter().take(2) {
                if sub_entry != "(general)" {
                    result.push((format!("  └─ {}", sub_entry), *duration));
                }
            }
        }
    }

    result
}

/// Creates hierarchical breakdown data for browser sessions
/// Groups by service, then shows page titles
pub fn create_browser_breakdown(sessions: &[Session]) -> Vec<(String, i64)> {
    let mut browser_map: BTreeMap<String, BTreeMap<String, i64>> = BTreeMap::new();

    for session in sessions {
        // Skip AFK sessions
        if session.is_afk.unwrap_or(false) {
            continue;
        }

        if let Some(page_title) = &session.browser_page_title {
            // Use browser_url as service if available (YouTube, WhatsApp, etc.)
            // Otherwise, don't create a hierarchy - just skip it
            if let Some(url) = &session.browser_url {
                // We have a recognized service (YouTube, WhatsApp, LinkedIn, etc.)
                let service_map = browser_map.entry(url.clone()).or_insert_with(BTreeMap::new);
                *service_map.entry(page_title.clone()).or_insert(0) += session.duration;
            }
            // If no browser_url, we don't include it in breakdown (it's already in regular app stats)
        }
    }

    flatten_hierarchical_map(browser_map, 5)
}

/// Creates hierarchical breakdown data for projects/terminal sessions
/// Groups by project, then shows directories
pub fn create_project_breakdown(sessions: &[Session]) -> Vec<(String, i64)> {
    let mut project_dir_map: BTreeMap<String, BTreeMap<String, i64>> = BTreeMap::new();

    for session in sessions {
        // Skip AFK sessions
        if session.is_afk.unwrap_or(false) {
            continue;
        }

        if let (Some(project), Some(dir)) = (&session.terminal_project_name, &session.terminal_directory) {
            let dir_map = project_dir_map.entry(project.clone()).or_insert_with(BTreeMap::new);
            *dir_map.entry(dir.clone()).or_insert(0) += session.duration;
        } else if let Some(project) = &session.ide_project_name {
            let dir_map = project_dir_map.entry(project.clone()).or_insert_with(BTreeMap::new);
            *dir_map.entry("(IDE)".to_string()).or_insert(0) += session.duration;
        }
    }

    flatten_hierarchical_map(project_dir_map, 3)
}

/// Creates hierarchical breakdown data for terminal sessions
/// Groups by project, then shows directories
pub fn create_terminal_breakdown(sessions: &[Session]) -> Vec<(String, i64)> {
    let mut terminal_project_map: BTreeMap<String, BTreeMap<String, i64>> = BTreeMap::new();

    for session in sessions {
        // Skip AFK sessions
        if session.is_afk.unwrap_or(false) {
            continue;
        }

        // Determine project name: prefer tmux window name, then terminal project, then directory-based
        let project_name = if let Some(tmux_window) = &session.tmux_window_name {
            // When tmux is detected, use the window name as the project
            tmux_window.clone()
        } else if let Some(terminal_project) = &session.terminal_project_name {
            terminal_project.clone()
        } else if let Some(dir) = &session.terminal_directory {
            // Fallback to directory-based project extraction
            extract_project_name(dir).unwrap_or_else(|| "Other".to_string())
        } else {
            "Other".to_string()
        };

        // Add the session to the project map
        let dir_map = terminal_project_map.entry(project_name).or_insert_with(BTreeMap::new);

        // Use directory as sub-entry, or tmux info if available
        let sub_entry = if let Some(tmux_window) = &session.tmux_window_name {
            if let Some(dir) = &session.terminal_directory {
                format!("{} ({})", extract_project_name(dir).unwrap_or_else(|| dir.clone()), tmux_window)
            } else {
                format!("tmux: {}", tmux_window)
            }
        } else if let Some(dir) = &session.terminal_directory {
            dir.clone()
        } else {
            "terminal".to_string()
        };

        *dir_map.entry(sub_entry).or_insert(0) += session.duration;
    }

    flatten_hierarchical_map(terminal_project_map, 3)
}

/// Creates hierarchical breakdown data for file editing
/// Groups by project, then shows files
pub fn create_file_breakdown(sessions: &[Session]) -> Vec<(String, String, i64)> {
    let mut file_project_map: BTreeMap<String, BTreeMap<(String, String), i64>> = BTreeMap::new();

    for session in sessions {
        // Skip AFK sessions
        if session.is_afk.unwrap_or(false) {
            continue;
        }

        if let (Some(filename), Some(language)) = (&session.editor_filename, &session.editor_language) {
            let project = session.editor_project_path.clone().unwrap_or_else(|| "Other".to_string());
            let file_map = file_project_map.entry(project).or_insert_with(BTreeMap::new);
            *file_map.entry((filename.clone(), language.clone())).or_insert(0) += session.duration;
        }
    }

    // Flatten file breakdown
    let mut file_flattened: Vec<(String, String, i64)> = Vec::new();
    let mut file_project_totals: Vec<(String, i64)> = Vec::new();

    for (project, files) in &file_project_map {
        let total: i64 = files.values().sum();
        file_project_totals.push((project.clone(), total));
    }
    file_project_totals.sort_by(|a, b| b.1.cmp(&a.1));

    for (project, _) in file_project_totals {
        if let Some(files) = file_project_map.get(&project) {
            let mut file_list: Vec<((String, String), i64)> = files.iter()
                .map(|((f, l), d)| ((f.clone(), l.clone()), *d))
                .collect();
            file_list.sort_by(|a, b| b.1.cmp(&a.1));

            // Show top 10 files
            for ((filename, language), duration) in file_list.iter().take(10) {
                file_flattened.push((filename.clone(), language.clone(), *duration));
            }
        }
    }

    file_flattened
}

/// Helper function to flatten a hierarchical map into display format
fn flatten_hierarchical_map(
    map: BTreeMap<String, BTreeMap<String, i64>>,
    max_children: usize,
) -> Vec<(String, i64)> {
    let mut flattened: Vec<(String, i64)> = Vec::new();
    let mut parent_totals: Vec<(String, i64)> = Vec::new();

    // Calculate totals per parent
    for (parent, children) in &map {
        let total: i64 = children.values().sum();
        parent_totals.push((parent.clone(), total));
    }
    parent_totals.sort_by(|a, b| b.1.cmp(&a.1));

    // Build hierarchical display
    for (parent, parent_total) in parent_totals {
        // Add parent header
        flattened.push((parent.clone(), parent_total));

        // Add children under this parent (sorted by duration)
        if let Some(children) = map.get(&parent) {
            let mut child_list: Vec<(String, i64)> = children.iter()
                .map(|(child, duration)| (child.clone(), *duration))
                .collect();
            child_list.sort_by(|a, b| b.1.cmp(&a.1));

            // Only show top N children per parent to avoid clutter
            for (child, duration) in child_list.iter().take(max_children) {
                flattened.push((format!("  └─ {}", child), *duration));
            }
        }
    }

    flattened
}
