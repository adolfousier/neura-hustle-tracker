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
/// Pattern: "username@hostname: /directory/path"
fn parse_terminal(window_name: &str, parsed: &mut ParsedSessionData) {
    // Pattern: username@hostname: /path
    if let Some(at_pos) = window_name.find('@') {
        let username = window_name[..at_pos].trim();
        parsed.terminal_username = Some(username.to_string());

        if let Some(colon_pos) = window_name.find(':') {
            let hostname = window_name[at_pos + 1..colon_pos].trim();
            parsed.terminal_hostname = Some(hostname.to_string());

            let directory = window_name[colon_pos + 1..].trim();
            parsed.terminal_directory = Some(directory.to_string());

            // Extract project name from directory
            parsed.terminal_project_name = extract_project_name(directory);
        }
    }
}

/// Extract project name from directory path
fn extract_project_name(path: &str) -> Option<String> {
    // Common patterns: /srv/rs/PROJECT, /home/user/projects/PROJECT, etc.
    let parts: Vec<&str> = path.split('/').collect();

    // Get last non-empty component
    for part in parts.iter().rev() {
        if !part.is_empty() && *part != "." && *part != ".." {
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
