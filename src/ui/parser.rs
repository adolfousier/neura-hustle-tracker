use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ParsedSessionData {
    // Browser tracking
    pub browser_url: Option<String>,
    pub browser_page_title: Option<String>,
    pub browser_notification_count: Option<i32>,

    // Terminal tracking
    pub terminal_username: Option<String>,
    pub terminal_hostname: Option<String>,
    pub terminal_directory: Option<String>,
    pub terminal_project_name: Option<String>,

    // Editor tracking
    pub editor_filename: Option<String>,
    pub editor_filepath: Option<String>,
    pub editor_project_path: Option<String>,
    pub editor_language: Option<String>,

    // Multiplexer tracking
    pub tmux_window_name: Option<String>,
    pub tmux_pane_count: Option<i32>,
    pub terminal_multiplexer: Option<String>,

    // IDE tracking
    pub ide_project_name: Option<String>,
    pub ide_file_open: Option<String>,
    pub ide_workspace: Option<String>,

    // Metadata
    pub parsing_success: bool,
}

impl ParsedSessionData {
    pub fn new() -> Self {
        Self {
            parsing_success: true,
            ..Default::default()
        }
    }
}

/// Main parser function that routes to specific parsers based on app type
pub fn parse_window_name(app_name: &str, window_name: &str) -> ParsedSessionData {
    let mut parsed = ParsedSessionData::new();

    let app_lower = app_name.to_lowercase();

    // Route to appropriate parser based on app type
    if is_browser(&app_lower) {
        parse_browser(window_name, &mut parsed);
    } else if is_terminal(&app_lower) {
        parse_terminal(window_name, &mut parsed);
    } else if is_editor(&app_lower) {
        parse_editor(window_name, &mut parsed);
    } else if is_file_manager(&app_lower) {
        parse_file_manager(window_name, &mut parsed);
    }

    parsed
}

/// Check if app is a browser
fn is_browser(app_name: &str) -> bool {
    app_name.contains("firefox")
    || app_name.contains("chrome")
    || app_name.contains("chromium")
    || app_name.contains("brave")
    || app_name.contains("safari")
    || app_name.contains("edge")
}

/// Check if app is a terminal
fn is_terminal(app_name: &str) -> bool {
    app_name.contains("terminal")
    || app_name.contains("gnome-terminal")
    || app_name.contains("alacritty")
    || app_name.contains("kitty")
    || app_name.contains("wezterm")
    || app_name.contains("konsole")
}

/// Check if app is an editor
fn is_editor(app_name: &str) -> bool {
    app_name.contains("editor")
    || app_name.contains("texteditor")
    || app_name.contains("vim")
    || app_name.contains("nvim")
    || app_name.contains("emacs")
    || app_name.contains("vscode")
    || app_name.contains("code")
    || app_name.contains("gedit")
    || app_name.contains("kate")
}

/// Check if app is a file manager
fn is_file_manager(app_name: &str) -> bool {
    app_name.contains("nautilus")
    || app_name.contains("file-manager")
    || app_name.contains("files")
    || app_name.contains("dolphin")
    || app_name.contains("thunar")
    || app_name.contains("nemo")
}

/// Parse browser window title
/// Pattern: "(notification_count) Page Title — Browser Name" or "Page Title — Browser Name"
fn parse_browser(window_name: &str, parsed: &mut ParsedSessionData) {
    // Extract notification count if present
    if let Some(start) = window_name.find('(') {
        if let Some(end) = window_name.find(')') {
            if end > start {
                let count_str = &window_name[start + 1..end];
                if let Ok(count) = count_str.parse::<i32>() {
                    parsed.browser_notification_count = Some(count);
                }
            }
        }
    }

    // Extract page title (remove browser name and notification count)
    let title = window_name
        .split(" — ")
        .next()
        .unwrap_or(window_name)
        .trim();

    // Remove notification count from title
    let clean_title = if title.starts_with('(') {
        if let Some(pos) = title.find(')') {
            title[pos + 1..].trim()
        } else {
            title
        }
    } else {
        title
    };

    parsed.browser_page_title = Some(clean_title.to_string());

    // Detect service from page title
    parsed.browser_url = detect_service(clean_title);
}

/// Detect web service from page title
fn detect_service(title: &str) -> Option<String> {
    let title_lower = title.to_lowercase();

    // Social media
    if title_lower.contains("whatsapp") {
        return Some("WhatsApp".to_string());
    }
    if title_lower.contains("facebook") {
        return Some("Facebook".to_string());
    }
    if title_lower.contains("twitter") || title_lower.contains("x.com") {
        return Some("Twitter/X".to_string());
    }
    if title_lower.contains("linkedin") {
        return Some("LinkedIn".to_string());
    }
    if title_lower.contains("instagram") {
        return Some("Instagram".to_string());
    }
    if title_lower.contains("reddit") {
        return Some("Reddit".to_string());
    }

    // Email
    if title_lower.contains("gmail") {
        return Some("Gmail".to_string());
    }
    if title_lower.contains("outlook") {
        return Some("Outlook".to_string());
    }
    if title_lower.contains("protonmail") {
        return Some("ProtonMail".to_string());
    }

    // Development
    if title_lower.contains("github") {
        return Some("GitHub".to_string());
    }
    if title_lower.contains("gitlab") {
        return Some("GitLab".to_string());
    }
    if title_lower.contains("stack overflow") {
        return Some("Stack Overflow".to_string());
    }
    if title_lower.contains("localhost") {
        return Some("Localhost".to_string());
    }

    // Work/Productivity
    if title_lower.contains("slack") {
        return Some("Slack".to_string());
    }
    if title_lower.contains("teams") {
        return Some("Microsoft Teams".to_string());
    }
    if title_lower.contains("notion") {
        return Some("Notion".to_string());
    }
    if title_lower.contains("jira") {
        return Some("Jira".to_string());
    }
    if title_lower.contains("trello") {
        return Some("Trello".to_string());
    }

    // Video
    if title_lower.contains("youtube") {
        return Some("YouTube".to_string());
    }
    if title_lower.contains("netflix") {
        return Some("Netflix".to_string());
    }

    None
}

/// Parse terminal window title
/// Handles multiple patterns: "username@hostname: /directory/path", tmux variants, and platform differences
fn parse_terminal(window_name: &str, parsed: &mut ParsedSessionData) {
    // First, check for tmux patterns and extract tmux information
    let (cleaned_title, tmux_info) = extract_tmux_info(window_name);

    // Set tmux information if found
    if let Some((window_name, pane_count)) = tmux_info {
        parsed.tmux_window_name = Some(window_name);
        parsed.tmux_pane_count = pane_count;
        parsed.terminal_multiplexer = Some("tmux".to_string());
    }

    // Parse the cleaned title (with tmux info removed) for user/host/directory
    if let Some(at_pos) = cleaned_title.find('@') {
        let username = cleaned_title[..at_pos].trim();
        parsed.terminal_username = Some(username.to_string());

        if let Some(colon_pos) = cleaned_title.find(':') {
            let hostname = cleaned_title[at_pos + 1..colon_pos].trim();
            parsed.terminal_hostname = Some(hostname.to_string());

            let directory_raw = cleaned_title[colon_pos + 1..].trim();

            // Expand tilde to home directory
            let directory = expand_tilde(directory_raw);
            parsed.terminal_directory = Some(directory.clone());

            // Extract project name from expanded directory
            parsed.terminal_project_name = extract_project_name(&directory);
        }
    } else {
        // Fallback: try to extract directory info even without user@host format
        // Look for common patterns like "/path/to/dir" or "~/path"
        if let Some(directory_raw) = extract_directory_fallback(&cleaned_title) {
            let directory = expand_tilde(&directory_raw);
            parsed.terminal_directory = Some(directory.clone());
            parsed.terminal_project_name = extract_project_name(&directory);
        }
    }
}

/// Extract tmux information from terminal title
/// Returns (cleaned_title, tmux_info) where tmux_info is Some((window_name, pane_count))
fn extract_tmux_info(title: &str) -> (String, Option<(String, Option<i32>)>) {
    // Common tmux patterns:
    // "tmux: window_name - user@host: ~/dir"
    // "[tmux] window_name | user@host: ~/dir"
    // "user@host: ~/dir - tmux (window_name)"
    // "window_name - tmux"

    let title_lower = title.to_lowercase();

    // Pattern 1: "tmux: window_name"
    if let Some(tmux_pos) = title_lower.find("tmux:") {
        let after_tmux = &title[tmux_pos + 5..].trim();
        if let Some(dash_pos) = after_tmux.find(" - ") {
            let window_name = after_tmux[..dash_pos].trim().to_string();
            let remaining = &title[tmux_pos + 5 + dash_pos + 3..].trim();
            let prefix = title[..tmux_pos].trim();
            let cleaned = if prefix.is_empty() {
                remaining.to_string()
            } else {
                format!("{} {}", prefix, remaining)
            };
            return (cleaned.trim().to_string(), Some((window_name, None)));
        }
    }

    // Pattern 2: "[tmux] window_name"
    if let Some(start) = title_lower.find("[tmux]") {
        let after_bracket = &title[start + 6..].trim();
        if let Some(end) = after_bracket.find(" | ") {
            let window_name = after_bracket[..end].trim().to_string();
            let cleaned = title[..start].trim().to_string() + &title[start + 6 + end + 3..];
            return (cleaned, Some((window_name, None)));
        }
    }

    // Pattern 3: " - tmux (window_name)"
    if let Some(tmux_start) = title_lower.find(" - tmux (") {
        if let Some(close_paren) = title[tmux_start..].find(')') {
            let window_name_start = tmux_start + 9; // Skip " - tmux ("
            let window_name = title[window_name_start..tmux_start + close_paren].trim().to_string();
            let cleaned = title[..tmux_start].trim().to_string();
            return (cleaned, Some((window_name, None)));
        }
    }

    // Pattern 4: Simple "window_name - tmux"
    if let Some(tmux_pos) = title_lower.find(" - tmux") {
        let window_name = title[..tmux_pos].trim().to_string();
        return ("".to_string(), Some((window_name, None)));
    }

    // Pattern 5: Alacritty/tmux common format: "tmux [window_name] - ..."
    if let Some(bracket_start) = title_lower.find("tmux [") {
        if let Some(bracket_end) = title[bracket_start..].find(']') {
            let window_name = title[bracket_start + 6..bracket_start + bracket_end].trim().to_string();
            let cleaned = title[..bracket_start].trim().to_string() + &title[bracket_start + bracket_end + 1..];
            return (cleaned.trim().to_string(), Some((window_name, None)));
        }
    }

    // Pattern 6: Look for tmux in title and extract window name from context
    if title_lower.contains("tmux") {
        // Try to extract window name from common patterns
        if let Some(dash_pos) = title.find(" - ") {
            let before_dash = title[..dash_pos].trim();
            if before_dash.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-') && before_dash.len() >= 2 {
                // Looks like a window name before dash
                let after_dash = title[dash_pos + 3..].trim();
                if after_dash.to_lowercase().contains("tmux") || after_dash.to_lowercase().contains("alacritty") {
                    return (after_dash.to_string(), Some((before_dash.to_string(), None)));
                }
            }
        }
    }

    // No tmux info found
    (title.to_string(), None)
}

/// Expand tilde (~) to home directory
fn expand_tilde(path: &str) -> String {
    if path.starts_with('~') {
        if let Ok(home) = std::env::var("HOME") {
            if path == "~" {
                return home;
            } else if path.starts_with("~/") {
                return home + &path[1..];
            }
        }
    }
    path.to_string()
}

/// Extract directory information as fallback when user@host: format is not found
fn extract_directory_fallback(title: &str) -> Option<String> {
    // Look for paths starting with / or ~
    if let Some(slash_pos) = title.find('/') {
        let potential_path = &title[slash_pos..];
        // Make sure it looks like a path (has multiple segments or common directories)
        if potential_path.contains('/') || potential_path.starts_with("~/") {
            return Some(potential_path.trim().to_string());
        }
    }

    // Look for ~ followed by path
    if let Some(tilde_pos) = title.find('~') {
        let after_tilde = &title[tilde_pos..];
        if after_tilde.len() > 1 && (after_tilde.starts_with("~/") || after_tilde.chars().nth(1).map_or(false, |c| c.is_alphabetic())) {
            return Some(after_tilde.trim().to_string());
        }
    }

    None
}

/// Extract project name from directory path with improved heuristics
fn extract_project_name(path: &str) -> Option<String> {
    // Handle tilde specially (even if HOME is not set)
    if path == "~" {
        return Some("Home".to_string());
    }

    // Handle home directory specially
    if let Ok(home) = std::env::var("HOME") {
        if path == home {
            return Some("Home".to_string());
        }
        if path.starts_with(&format!("{}/", home)) {
            // For deeper paths like ~/projects/myapp, extract the project name (myapp)
            // But for simple paths like ~/Documents, use the directory name (Documents)
            let after_home = &path[home.len() + 1..];
            let parts: Vec<&str> = after_home.split('/').collect();

            // If we have multiple parts (e.g., projects/myapp), prefer the last meaningful part
            for part in parts.iter().rev() {
                if !part.is_empty() && *part != "." && *part != ".." && part.len() >= 2 {
                    return Some(part.to_string());
                }
            }

            // Fallback to first directory after home
            if let Some(first_dir) = parts.first() {
                if !first_dir.is_empty() {
                    return Some(first_dir.to_string());
                }
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

    // If all parts are in skip_dirs, return None
    // Fallback: if we have any valid directory component (not in skip_dirs)
    for part in parts.iter().rev() {
        let part_lower = part.to_lowercase();
        if !part.is_empty() && *part != "." && *part != ".." && !skip_dirs.contains(&part_lower.as_str()) {
            return Some(part.to_string());
        }
    }

    None
}

/// Parse editor window title
/// Pattern: "filename (path) - Editor Name" or "path/filename - Editor Name"
fn parse_editor(window_name: &str, parsed: &mut ParsedSessionData) {
    // Try pattern: "filename (path) - Editor"
    if let Some(paren_start) = window_name.find('(') {
        let filename = window_name[..paren_start].trim();
        parsed.editor_filename = Some(filename.to_string());

        if let Some(paren_end) = window_name.find(')') {
            let filepath = window_name[paren_start + 1..paren_end].trim();
            parsed.editor_filepath = Some(filepath.to_string());

            // Extract project path
            parsed.editor_project_path = extract_project_name(filepath);

            // Detect language from file extension
            parsed.editor_language = detect_language(filename);
        }
    } else {
        // Try pattern: "path/filename - Editor"
        if let Some(dash_pos) = window_name.find(" - ") {
            let full_path = window_name[..dash_pos].trim();

            // Extract filename
            if let Some(last_slash) = full_path.rfind('/') {
                let filename = &full_path[last_slash + 1..];
                parsed.editor_filename = Some(filename.to_string());

                // Extract directory
                let directory = &full_path[..last_slash];
                parsed.editor_filepath = Some(directory.to_string());
                parsed.editor_project_path = extract_project_name(directory);

                // Detect language
                parsed.editor_language = detect_language(filename);
            }
        }
    }
}

/// Detect programming language from file extension
fn detect_language(filename: &str) -> Option<String> {
    if let Some(ext_pos) = filename.rfind('.') {
        let ext = &filename[ext_pos + 1..].to_lowercase();

        let language = match ext.as_str() {
            "rs" => "Rust",
            "py" => "Python",
            "js" => "JavaScript",
            "ts" => "TypeScript",
            "jsx" => "React",
            "tsx" => "React TypeScript",
            "go" => "Go",
            "java" => "Java",
            "cpp" | "cc" | "cxx" => "C++",
            "c" => "C",
            "h" | "hpp" => "Header",
            "sh" | "bash" => "Shell",
            "md" => "Markdown",
            "toml" => "TOML",
            "yaml" | "yml" => "YAML",
            "json" => "JSON",
            "xml" => "XML",
            "html" => "HTML",
            "css" => "CSS",
            "scss" | "sass" => "SCSS",
            "sql" => "SQL",
            "php" => "PHP",
            "rb" => "Ruby",
            "swift" => "Swift",
            "kt" | "kts" => "Kotlin",
            "vim" => "VimScript",
            "lua" => "Lua",
            _ => return None,
        };

        Some(language.to_string())
    } else {
        None
    }
}

/// Parse file manager window title
fn parse_file_manager(window_name: &str, parsed: &mut ParsedSessionData) {
    // File manager titles often contain paths
    // This is a simple implementation, can be enhanced
    if window_name.starts_with('/') || window_name.contains("file://") {
        parsed.terminal_directory = Some(window_name.to_string());
        parsed.terminal_project_name = extract_project_name(window_name);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_browser() {
        let parsed = parse_window_name(
            "firefox",
            "(11) WhatsApp Business — Mozilla Firefox"
        );

        assert_eq!(parsed.browser_notification_count, Some(11));
        assert_eq!(parsed.browser_page_title, Some("WhatsApp Business".to_string()));
        assert_eq!(parsed.browser_url, Some("WhatsApp".to_string()));
    }

    #[test]
    fn test_parse_terminal() {
        let parsed = parse_window_name(
            "gnome-terminal",
            "adolfo@adolfo-ubuntu-pro25: /srv/rs/neura-hustle-tracker"
        );

        assert_eq!(parsed.terminal_username, Some("adolfo".to_string()));
        assert_eq!(parsed.terminal_hostname, Some("adolfo-ubuntu-pro25".to_string()));
        assert_eq!(parsed.terminal_directory, Some("/srv/rs/neura-hustle-tracker".to_string()));
        assert_eq!(parsed.terminal_project_name, Some("neura-hustle-tracker".to_string()));
    }

    #[test]
    fn test_parse_editor() {
        let parsed = parse_window_name(
            "texteditor",
            "commands.md (/srv/rs/neura-hustle-tracker) - Text Editor"
        );

        assert_eq!(parsed.editor_filename, Some("commands.md".to_string()));
        assert_eq!(parsed.editor_filepath, Some("/srv/rs/neura-hustle-tracker".to_string()));
        assert_eq!(parsed.editor_language, Some("Markdown".to_string()));
    }

    #[test]
    fn test_detect_language() {
        assert_eq!(detect_language("main.rs"), Some("Rust".to_string()));
        assert_eq!(detect_language("app.py"), Some("Python".to_string()));
        assert_eq!(detect_language("index.js"), Some("JavaScript".to_string()));
        assert_eq!(detect_language("component.tsx"), Some("React TypeScript".to_string()));
    }

    #[test]
    fn test_detect_service() {
        assert_eq!(detect_service("WhatsApp Business"), Some("WhatsApp".to_string()));
        assert_eq!(detect_service("GitHub - Repository"), Some("GitHub".to_string()));
        assert_eq!(detect_service("Gmail - Inbox"), Some("Gmail".to_string()));
    }
}
