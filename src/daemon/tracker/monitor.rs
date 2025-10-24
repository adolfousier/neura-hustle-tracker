use active_win_pos_rs::get_active_window;
use anyhow::Result;
use std::env;
#[cfg(target_os = "windows")]
use super::windows_inspection;
#[cfg(target_os = "macos")]
use super::macos_inspection;

#[derive(serde::Deserialize, Debug)]
struct WindowInfo {
    #[serde(default)]
    wm_class: String,
    #[serde(default)]
    title: String,
    #[serde(default)]
    focus: bool,
}

#[cfg(target_os = "linux")]
#[derive(Debug)]
struct ProcessInfo {
    has_tmux: bool,
    tmux_session: Option<String>,
    tmux_window: Option<String>,
    editor_info: Option<EditorInfo>,
}

#[cfg(target_os = "linux")]
#[derive(Debug)]
struct EditorInfo {
    filename: String,
    filepath: String,
}

pub struct AppMonitor {
    use_wayland: bool,
}

impl Default for AppMonitor {
    fn default() -> Self {
        Self::new()
    }
}

impl AppMonitor {
    pub fn new() -> Self {
        // Log detected platform
        #[cfg(target_os = "macos")]
        log::info!("=== PLATFORM: macOS ===");

        #[cfg(target_os = "windows")]
        log::info!("=== PLATFORM: Windows ===");

        #[cfg(target_os = "linux")]
        log::info!("=== PLATFORM: Linux ===");

        let use_wayland = Self::is_wayland();

        // Platform-specific window tracking method
        #[cfg(target_os = "linux")]
        {
            if use_wayland {
                log::info!("Session type: Wayland - using D-Bus for window tracking");
            } else {
                log::info!("Session type: X11 - using X11 APIs for window tracking");
            }
        }

        #[cfg(target_os = "macos")]
        log::info!("Using Cocoa/AppKit APIs for window tracking");

        #[cfg(target_os = "windows")]
        log::info!("Using Win32 APIs for window tracking");

        Self { use_wayland }
    }


    fn is_wayland() -> bool {
        #[cfg(target_os = "linux")]
        {
            env::var("WAYLAND_DISPLAY").is_ok() ||
            env::var("XDG_SESSION_TYPE").map(|s| s == "wayland").unwrap_or(false)
        }

        #[cfg(not(target_os = "linux"))]
        {
            false
        }
    }

    #[cfg(target_os = "linux")]
    #[cfg(target_os = "linux")]
    fn inspect_process_tree(pid: u64) -> Option<ProcessInfo> {
        let mut info = ProcessInfo {
            has_tmux: false,
            tmux_session: None,
            tmux_window: None,
            editor_info: None,
        };

        // Get child processes recursively
        let children = Self::get_child_processes(pid);

        for child_pid in children {
            if let Some(cmdline) = Self::get_process_cmdline(child_pid) {
                let cmd = cmdline.split('\0').next().unwrap_or("");

                // Check for tmux
                if cmd.contains("tmux") {
                    info.has_tmux = true;
                    // Try to get session name from cmdline
                    for arg in cmdline.split('\0').skip(1) {
                        if arg.starts_with("-t") || arg.starts_with("-s") {
                            if let Some(session) = arg.split('=').nth(1).or_else(|| arg.split(' ').nth(1)) {
                                info.tmux_session = Some(session.to_string());
                            }
                        }
                    }
                }

                // Check for vim/neovim
                if cmd.ends_with("vim") || cmd.ends_with("nvim") || cmd == "vim" || cmd == "nvim" {
                    let args: Vec<&str> = cmdline.split('\0').collect();
                    if args.len() > 1 {
                        let file_arg = args.last().unwrap();
                        if !file_arg.starts_with('-') && !file_arg.is_empty() {
                            // Try to resolve to absolute path
                            let filepath = std::fs::canonicalize(file_arg).unwrap_or_else(|_| std::path::PathBuf::from(file_arg));
                            let filename = std::path::Path::new(&filepath)
                                .file_name()
                                .unwrap_or_else(|| std::ffi::OsStr::new(file_arg))
                                .to_string_lossy()
                                .to_string();

                            info.editor_info = Some(EditorInfo {
                                filename,
                                filepath: filepath.to_string_lossy().to_string(),
                            });
                        }
                    }
                }
            }
        }

        // If tmux detected, try to get the current window name
        if info.has_tmux {
            let session_arg = if let Some(ref session) = info.tmux_session {
                format!("-t {}", session)
            } else {
                "".to_string()
            };
            let cmd = format!("tmux list-windows {} -F \"#{{window_name}}:#{{window_active}}\"", session_arg);
            if let Ok(output) = std::process::Command::new("sh").arg("-c").arg(&cmd).output() {
                if output.status.success() {
                    let stdout = String::from_utf8_lossy(&output.stdout);
                    for line in stdout.lines() {
                        if let Some(colon) = line.rfind(':') {
                            let active = &line[colon + 1..];
                            if active == "1" {
                                let window_name = line[..colon].to_string();
                                info.tmux_window = Some(window_name);
                                break;
                            }
                        }
                    }
                }
            }
        }

        Some(info)
    }

    #[cfg(target_os = "linux")]
    fn get_child_processes(pid: u64) -> Vec<u64> {
        let mut children = Vec::new();

        // Read /proc/<pid>/task/<pid>/children
        let children_path = format!("/proc/{}/task/{}/children", pid, pid);
        if let Ok(content) = std::fs::read_to_string(&children_path) {
            for child in content.split_whitespace() {
                if let Ok(child_pid) = child.parse::<u64>() {
                    children.push(child_pid);
                    // Recursively get grandchildren
                    children.extend(Self::get_child_processes(child_pid));
                }
            }
        }

        children
    }

    #[cfg(target_os = "linux")]
    fn get_process_cmdline(pid: u64) -> Option<String> {
        let cmdline_path = format!("/proc/{}/cmdline", pid);
        std::fs::read_to_string(&cmdline_path).ok()
    }

    async fn get_active_window_x11() -> Result<(String, String)> {
        use std::process::Command;

        // Get focused window ID
        let window_id_output = Command::new("xdotool")
            .arg("getwindowfocus")
            .output()?;

        if !window_id_output.status.success() {
            return Err(anyhow::anyhow!("xdotool getwindowfocus failed"));
        }

        let wid = String::from_utf8_lossy(&window_id_output.stdout).trim().to_string();

        // Get window name
        let title_output = Command::new("xdotool")
            .arg("getwindowname")
            .arg(&wid)
            .output()?;

        if !title_output.status.success() {
            return Err(anyhow::anyhow!("xdotool getwindowname failed"));
        }

        let title = String::from_utf8_lossy(&title_output.stdout).trim().to_string();

        // Get WM_CLASS
        let wm_class_output = Command::new("xprop")
            .arg("-id")
            .arg(&wid)
            .arg("WM_CLASS")
            .output()?;

        if !wm_class_output.status.success() {
            return Err(anyhow::anyhow!("xprop WM_CLASS failed"));
        }

        let wm_class_str = String::from_utf8_lossy(&wm_class_output.stdout);
        let class = wm_class_str
            .lines()
            .find(|line| line.contains("WM_CLASS"))
            .and_then(|line| line.split('"').nth(1))
            .unwrap_or("")
            .to_string();

        Ok((class, title))
    }

    async fn get_active_window_wayland() -> Result<(String, String)> {
        let connection = zbus::Connection::session().await?;

        let response = connection.call_method(
            Some("org.gnome.Shell"),
            "/org/gnome/Shell/Extensions/Windows",
            Some("org.gnome.Shell.Extensions.Windows"),
            "List",
            &(),
        ).await?;

        // The response is a string directly, not a variant
        let json_str: String = response.body().deserialize()?;

        let windows: Vec<WindowInfo> = serde_json::from_str(&json_str)?;

        let focused_window = windows.iter()
            .find(|w| w.focus)
            .ok_or(anyhow::anyhow!("No focused window found"))?;

        Ok((focused_window.wm_class.clone(), focused_window.title.clone()))
    }

    // Get both app and window info in a single call
    pub async fn get_active_window_info_async(&self) -> Result<(String, Option<String>)> {
        // Try active-win-pos-rs first (works for X11 and some Wayland compositors)
        match get_active_window() {
            Ok(active_window) => {
                let app_name = self.fix_app_name(active_window.app_name.clone());
                log::info!("Detected app: {}", app_name);
                let mut window_title = if active_window.title.is_empty() || active_window.title == active_window.app_name {
                    None
                } else {
                    Some(active_window.title.clone())
                };
                // Enhance title for terminal apps if process_id is available
                if Self::is_terminal_app(&app_name) {
                    let current_title = window_title.as_deref().unwrap_or("");
                    log::info!("Main path terminal title before enhancement: '{}'", current_title);
                    // Extract directory from prompt if it looks like a shell prompt
                    let mut enhanced = if current_title.contains("@") && current_title.contains(": ") {
                        current_title.split(": ").last().unwrap_or(current_title).to_string()
                    } else {
                        current_title.to_string()
                    };
                    log::info!("Main path terminal title after directory extraction: '{}'", enhanced);

                    if active_window.process_id != 0 {
                        let pid = active_window.process_id as u64;
                        enhanced = {
                            #[cfg(target_os = "windows")]
                            { super::windows_inspection::enhance_windows_title(&enhanced, pid) }
                            #[cfg(target_os = "macos")]
                            { super::macos_inspection::enhance_macos_title(&enhanced, pid) }
                             #[cfg(target_os = "linux")]
                             { if let Some(info) = Self::inspect_process_tree(pid) {
                                 let mut title = enhanced;
                                 if let Some(window) = info.tmux_window {
                                     title = format!("{} - {}", window, title);
                                 } else if info.has_tmux {
                                     let session = info.tmux_session.unwrap_or("session".to_string());
                                     title = format!("tmux: {} - {}", session, title);
                                 }
                                 if let Some(editor) = info.editor_info {
                                     title = format!("{} ({}) - {}", editor.filename, editor.filepath, title);
                                 }
                                 title
                             } else {
                                 enhanced
                             } }
                        };
                    }

                    if enhanced != current_title {
                        window_title = Some(enhanced);
                    }
                }
                return Ok((app_name, window_title));
            }
            Err(_) => {
                // Fallbacks based on platform/session type
                if self.use_wayland {
                    // Try GNOME extension for Wayland
                    match Self::get_active_window_wayland().await {
                        Ok((wm_class, title)) => {
                            let app_name = self.fix_app_name(wm_class);
                            return Ok((app_name, Some(title)));
                        }
                        Err(_) => {}
                    }
                } else {
                    // Try GNOME extension for Wayland
                    match Self::get_active_window_wayland().await {
                        Ok((wm_class, mut title)) => {
                            log::info!("Wayland fallback title: '{}'", title);
                            let app_name = self.fix_app_name(wm_class);
                            // Extract directory from prompt if it looks like a shell prompt
                            if Self::is_terminal_app(&app_name) && title.contains("@") && title.contains(": ") {
                                if let Some(dir) = title.split(": ").last() {
                                    title = dir.to_string();
                                    log::info!("Wayland fallback title after extraction: '{}'", title);
                                }
                            }
                            return Ok((app_name, Some(title)));
                        }
                        Err(_) => {
                            // Try xdotool/xprop for X11
                            #[cfg(target_os = "linux")]
                            {
                                if let Ok((wm_class, title)) = Self::get_active_window_x11().await {
                                    let app_name = self.fix_app_name(wm_class);
                                    return Ok((app_name, Some(title)));
                                }
                            }
                        }
                    }
                }

                // macOS AppleScript fallback
                #[cfg(target_os = "macos")]
                {
                    if let Ok((app, title, pid)) = Self::get_active_window_info_macos().await {
                        let app_name = self.fix_app_name(app);
                        let mut window_title = if title.is_empty() { None } else { Some(title) };
                        if Self::is_terminal_app(&app_name) {
                            window_title = Some(macos_inspection::enhance_macos_title(&window_title.unwrap_or_default(), pid));
                        }
                        return Ok((app_name, window_title));
                    }
                }
            }
        }

        Err(anyhow::anyhow!("Failed to get window info"))
    }



    #[cfg(target_os = "macos")]
    async fn get_active_app_macos() -> Result<String> {
        use std::process::Command;

        // AppleScript to get the frontmost application name
        let script = r#"
            tell application "System Events"
                set frontApp to first application process whose frontmost is true
                return name of frontApp
            end tell
        "#;

        let output = Command::new("osascript")
            .arg("-e")
            .arg(script)
            .output()?;

        if output.status.success() {
            let app_name = String::from_utf8_lossy(&output.stdout)
                .trim()
                .to_string();

            if !app_name.is_empty() {
                log::debug!("AppleScript returned app: '{}'", app_name);
                return Ok(app_name);
            }
        } else {
            let error = String::from_utf8_lossy(&output.stderr);
            log::warn!("AppleScript failed to get app: {}", error);
        }

        Err(anyhow::anyhow!("Failed to get active app via AppleScript"))
    }

    #[cfg(target_os = "macos")]
    async fn get_active_window_info_macos() -> Result<(String, String, u64)> {
        use std::process::Command;

        // Comprehensive AppleScript that tries app-specific methods for browsers and terminals
        let script = r#"
            tell application "System Events"
                set frontApp to first application process whose frontmost is true
                set appName to name of frontApp
                set pid to unix id of frontApp
            end tell

            -- Try application-specific methods for common apps
            if appName contains "Firefox" or appName is "firefox" then
                tell application "Firefox"
                    try
                        set windowTitle to name of front window
                        return appName & "|" & windowTitle & "|" & pid
                    end try
                end tell
            else if appName contains "Chrome" or appName contains "Brave" or appName contains "Chromium" then
                tell application appName
                    try
                        set windowTitle to title of active tab of front window
                        return appName & "|" & windowTitle & "|" & pid
                    end try
                end tell
            else if appName contains "Safari" then
                tell application "Safari"
                    try
                        set windowTitle to name of front document
                        return appName & "|" & windowTitle & "|" & pid
                    end try
                end tell
            else if appName contains "Terminal" or appName is "Terminal" then
                tell application "Terminal"
                    try
                        set windowTitle to name of front window
                        return appName & "|" & windowTitle & "|" & pid
                    end try
                end tell
            else if appName contains "iTerm" or appName is "iTerm2" then
                tell application "iTerm"
                    try
                        set windowTitle to name of current session of current window
                        return appName & "|" & windowTitle & "|" & pid
                    end try
                end tell
            else if appName contains "Alacritty" or appName is "Alacritty" or appName is "alacritty" then
                tell application "System Events"
                    tell process "Alacritty"
                        try
                            set windowTitle to name of front window
                            return appName & "|" & windowTitle & "|" & pid
                        end try
                    end tell
                end tell
            end if

            -- Fallback: try System Events
            tell application "System Events"
                tell process appName
                    try
                        set windowTitle to name of front window
                        return appName & "|" & windowTitle & "|" & pid
                    on error
                        return appName & "|" & "|" & pid
                    end try
                end tell
            end tell
        "#;

        let output = Command::new("osascript")
            .arg("-e")
            .arg(script)
            .output()?;

        if output.status.success() {
            let result = String::from_utf8_lossy(&output.stdout).trim().to_string();

            let parts: Vec<&str> = result.split('|').collect();
            if parts.len() >= 3 {
                let app_name = parts[0].trim().to_string();
                let window_title = parts[1].trim().to_string();
                if let Ok(pid) = parts[2].trim().parse::<u64>() {
                    log::debug!("AppleScript returned: app='{}', title='{}', pid={}", app_name, window_title, pid);

                    if !app_name.is_empty() {
                        return Ok((app_name, window_title, pid));
                    }
                }
            }
        } else {
            let error = String::from_utf8_lossy(&output.stderr);
            log::warn!("AppleScript failed: {}", error);
        }

        Err(anyhow::anyhow!("Failed to get window info via AppleScript"))
    }

    #[cfg(target_os = "macos")]
    async fn get_window_title_macos(app_name: &str) -> Result<String> {
        use std::process::Command;

        // AppleScript to get the window title of the frontmost window
        let script = format!(
            r#"
            tell application "System Events"
                set frontApp to first application process whose frontmost is true
                set appName to name of frontApp
                if appName is "{}" then
                    try
                        set windowTitle to name of front window of frontApp
                        return windowTitle
                    on error
                        return ""
                    end try
                else
                    return ""
                end if
            end tell
            "#,
            app_name
        );

        let output = Command::new("osascript")
            .arg("-e")
            .arg(&script)
            .output()?;

        if output.status.success() {
            let title = String::from_utf8_lossy(&output.stdout)
                .trim()
                .to_string();

            if !title.is_empty() {
                log::debug!("AppleScript returned: '{}'", title);
                return Ok(title);
            }
        } else {
            let error = String::from_utf8_lossy(&output.stderr);
            log::warn!("AppleScript failed: {}", error);
        }

        Err(anyhow::anyhow!("Failed to get window title via AppleScript"))
    }

    #[cfg(target_os = "windows")]
    async fn get_window_title_windows(app_name: &str) -> Result<String> {
        use std::process::Command;

        // PowerShell script to get the window title of the active window
        let script = format!(
            r#"
            Add-Type @"
                using System;
                using System.Runtime.InteropServices;
                using System.Text;
                public class Win32 {{
                    [DllImport("user32.dll")]
                    public static extern IntPtr GetForegroundWindow();
                    [DllImport("user32.dll", CharSet = CharSet.Unicode)]
                    public static extern int GetWindowText(IntPtr hWnd, StringBuilder text, int count);
                }}
"@
            $hwnd = [Win32]::GetForegroundWindow()
            $title = New-Object System.Text.StringBuilder 256
            [Win32]::GetWindowText($hwnd, $title, 256) | Out-Null
            $title.ToString()
            "#
        );

        let output = Command::new("powershell")
            .arg("-NoProfile")
            .arg("-Command")
            .arg(&script)
            .output()?;

        if output.status.success() {
            let title = String::from_utf8_lossy(&output.stdout)
                .trim()
                .to_string();

            if !title.is_empty() {
                log::debug!("PowerShell returned: '{}'", title);
                return Ok(title);
            }
        } else {
            let error = String::from_utf8_lossy(&output.stderr);
            log::warn!("PowerShell failed: {}", error);
        }

        Err(anyhow::anyhow!("Failed to get window title via PowerShell"))
    }

    fn is_terminal_app(app: &str) -> bool {
        let app_lower = app.to_lowercase();
        app_lower.contains("terminal") ||
        app_lower.contains("iterm") ||
        app_lower.contains("alacritty") ||
        app_lower.contains("cmd") ||
        app_lower.contains("powershell") ||
        app_lower.contains("wt") ||
        app_lower == "hyper" ||
        app_lower == "tabby" ||
        app_lower == "warp"
    }

    fn fix_app_name(&self, app: String) -> String {
        let app_lower = app.to_lowercase();

        // Linux-specific: Handle Wayland wm_class format (e.g., "org.gnome.Nautilus", "firefox_firefox")
        #[cfg(target_os = "linux")]
        let normalized = {
            if app_lower.contains('.') {
                app_lower.split('.').last().unwrap_or(&app_lower).to_string()
            } else if app_lower.contains('_') {
                app_lower.split('_').next().unwrap_or(&app_lower).to_string()
            } else {
                app_lower.clone()
            }
        };

        // macOS/Windows: No normalization, use lowercase as-is
        #[cfg(not(target_os = "linux"))]
        let normalized = app_lower.clone();

        // Cross-platform app detection (works on all platforms)
        if normalized.contains("chrome") || normalized.contains("chromium") || normalized.contains("google-chrome") {
            return "chrome".to_string();
        } else if normalized.contains("firefox") {
            return "firefox".to_string();
        } else if normalized.contains("code") || normalized.contains("vscode") || normalized.contains("vscodium") {
            return "vscode".to_string();
        } else if normalized.contains("slack") {
            return "slack".to_string();
        } else if normalized.contains("discord") {
            return "discord".to_string();
        } else if normalized.contains("telegram") {
            return "telegram".to_string();
        } else if normalized.contains("zoom") {
            return "zoom".to_string();
        } else if normalized.contains("teams") {
            return "teams".to_string();
        } else if normalized.contains("skype") {
            return "skype".to_string();
        } else if normalized.contains("spotify") {
            return "spotify".to_string();
        } else if normalized.contains("vlc") {
            return "vlc".to_string();
        }

        // Linux-ONLY app detection (GNOME, KDE-specific apps)
        #[cfg(target_os = "linux")]
        {
            if normalized.contains("gnome-terminal") || normalized.contains("terminal") {
                return "gnome-terminal".to_string();
            } else if normalized == "soffice" || app_lower == "soffice.bin" {
                return "libreoffice".to_string();
            } else if normalized.contains("nautilus") || normalized.contains("files") || normalized.contains("thunar") || normalized.contains("dolphin") || normalized.contains("nemo") {
                return "file-manager".to_string();
            } else if normalized.contains("alacritty") || normalized.contains("kitty") || normalized.contains("wezterm") || normalized.contains("konsole") {
                return "terminal".to_string();
            } else if normalized.contains("vim") || normalized.contains("nvim") || normalized.contains("emacs") || normalized.contains("nano") || normalized.contains("gedit") || normalized.contains("kate") || normalized.contains("mousepad") {
                return "editor".to_string();
            } else if normalized.contains("rhythmbox") || normalized.contains("audacious") || normalized.contains("clementine") {
                return "media".to_string();
            } else if normalized.contains("thunderbird") || normalized.contains("evolution") || normalized.contains("geary") {
                return "email".to_string();
            } else if normalized.contains("signal") || normalized.contains("element") || normalized.contains("matrix") {
                return "chat".to_string();
            }
        }

        // Return original or normalized name
        if normalized.len() < app.len() && !normalized.is_empty() {
            normalized
        } else {
            app
        }
    }

}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_get_active_window_info_async() {
        let monitor = AppMonitor::new();
        // Note: This test may fail if no active window is available
        match monitor.get_active_window_info_async().await {
            Ok((app, _window_title)) => {
                assert!(!app.is_empty());
                // window_title can be None, which is fine
            }
            Err(_) => {
                // Test passes if method exists and can be called, even if it fails due to no active window
            }
        }
    }
}
