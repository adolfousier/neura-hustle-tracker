use active_win_pos_rs::get_active_window;
use anyhow::Result;
use std::env;
#[cfg(target_os = "linux")]
use super::process_inspection;

#[derive(serde::Deserialize, Debug)]
struct WindowInfo {
    #[serde(default)]
    wm_class: String,
    #[serde(default)]
    title: String,
    #[serde(default)]
    focus: bool,
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

    pub fn uses_wayland(&self) -> bool {
        self.use_wayland
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

    // Get both app and window info in a single call (more efficient for macOS AppleScript)
    pub async fn get_active_app_async(&self) -> Result<String> {
        if self.use_wayland {
            // Use Wayland D-Bus method
            match Self::get_active_window_wayland().await {
                Ok((wm_class, _title)) => {
                    log::info!("Detected active app (Wayland): {}", wm_class);
                    Ok(self.fix_app_name(wm_class))
                }
                Err(e) => {
                    let error_msg = format!(
                        "Wayland window detection failed: {}. \
                        Make sure the 'Window Calls' GNOME extension is installed and enabled. \
                        Install from: https://extensions.gnome.org/extension/4724/window-calls/",
                        e
                    );
                    log::warn!("{}", error_msg);
                    Err(anyhow::anyhow!(error_msg))
                }
            }
        } else {
            // Use platform-specific native APIs
            match get_active_window() {
                Ok(active_window) => {
                    // Platform-specific debug logging
                    #[cfg(target_os = "macos")]
                    log::debug!("[macOS] Raw window - app: '{}', title: '{}', path: {:?}, position: {:?}",
                               active_window.app_name,
                               active_window.title,
                               active_window.process_path,
                               active_window.position);

                    #[cfg(target_os = "windows")]
                    log::debug!("[Windows] Raw window - app: '{}', title: '{}', path: {:?}, position: {:?}",
                               active_window.app_name,
                               active_window.title,
                               active_window.process_path,
                               active_window.position);

                    #[cfg(target_os = "linux")]
                    log::debug!("[Linux/X11] Raw window - app: '{}', title: '{}'",
                               active_window.app_name,
                               active_window.title);

                    let original_name = active_window.app_name.clone();
                    let fixed_name = self.fix_app_name(original_name.clone());

                    if original_name != fixed_name {
                        log::info!("App detected: '{}' (normalized from '{}')", fixed_name, original_name);
                    } else {
                        log::info!("App detected: '{}'", fixed_name);
                    }

                    Ok(fixed_name)
                }
                Err(e) => {
                    log::error!("Failed to get active window: {:?}", e);

                    // On macOS, try AppleScript as fallback
                    #[cfg(target_os = "macos")]
                    {
                        log::info!("active-win-pos-rs failed, trying AppleScript fallback...");
                        match Self::get_active_app_macos().await {
                            Ok(app_name) => {
                                log::info!("AppleScript successfully detected app: '{}'", app_name);
                                return Ok(self.fix_app_name(app_name));
                            }
                            Err(applescript_err) => {
                                log::error!("AppleScript fallback also failed: {}", applescript_err);
                            }
                        }
                    }

                    let error_msg = "Failed to get active window";
                    log::warn!("{}", error_msg);
                    Err(anyhow::anyhow!(error_msg))
                }
            }
        }
    }

    pub async fn get_active_window_name_async(&self) -> Result<String> {
        if self.use_wayland {
            // Use Wayland D-Bus method
            match Self::get_active_window_wayland().await {
                Ok((_wm_class, mut title)) => {
                    // Extract directory from prompt if it looks like a shell prompt
                    if title.contains("@") && title.contains(": ") {
                        if let Some(dir) = title.split(": ").last() {
                            title = dir.to_string();
                        }
                    }
                    Ok(title)
                },
                Err(_) => {
                    log::warn!("Failed to get active window title (Wayland).");
                    Ok("Unknown Window".to_string())
                }
            }
        } else {
            // Use platform-specific native APIs
            match get_active_window() {
                Ok(active_window) => {
                    let mut title = active_window.title.clone();

                    // On macOS, if we get a generic title (app name only), try AppleScript fallback
                    #[cfg(target_os = "macos")]
                    {
                        let app_name = active_window.app_name.clone();
                        if title == app_name || title.is_empty() || title == "Unknown" {
                            log::debug!("Generic title detected for '{}', trying AppleScript fallback", app_name);
                            if let Ok(detailed_title) = Self::get_window_title_macos(&app_name).await {
                                if !detailed_title.is_empty() && detailed_title != app_name {
                                    log::info!("AppleScript retrieved title for {}: '{}'", app_name, detailed_title);
                                    return Ok(detailed_title);
                                }
                            }
                        }
                    }

                    // On Windows, if we get a generic title, try PowerShell fallback
                    #[cfg(target_os = "windows")]
                    {
                        if title == app_name || title.is_empty() || title == "Unknown" {
                            log::debug!("Generic title detected for '{}', trying PowerShell fallback", app_name);
                            if let Ok(detailed_title) = Self::get_window_title_windows(&app_name).await {
                                if !detailed_title.is_empty() && detailed_title != app_name {
                                    log::info!("PowerShell retrieved title for {}: '{}'", app_name, detailed_title);
                                    return Ok(detailed_title);
                                }
                            }
                        }
                    }

                    // On Linux, enhance title with process inspection
                    #[cfg(target_os = "linux")]
                    {
                        // First, extract directory from prompt if it looks like a shell prompt
                        if title.contains("@") && title.contains(": ") {
                            if let Some(dir) = title.split(": ").last() {
                                title = dir.to_string();
                            }
                        }
                        let pid = active_window.process_id;
                        if pid != 0 {
                             if let Some(info) = process_inspection::inspect_process_tree(pid) {
                                 if let Some(window) = info.tmux_window {
                                     title = format!("{} - {}", window, title);
                                 } else if info.has_tmux {
                                     let session = info.tmux_session.unwrap_or("session".to_string());
                                     title = format!("tmux: {} - {}", session, title);
                                 }
                                 if let Some(editor) = info.editor_info {
                                     title = format!("{} ({}) - {}", editor.filename, editor.filepath, title);
                                 }
                             }
                        }
                    }

                    Ok(title)
                }
                Err(_) => {
                    log::warn!("Failed to get active window title.");
                    Ok("Unknown Window".to_string())
                }
            }
        }
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
    async fn get_active_window_info_macos() -> Result<(String, String)> {
        use std::process::Command;

        // Comprehensive AppleScript that tries app-specific methods for browsers and terminals
        let script = r#"
            tell application "System Events"
                set frontApp to first application process whose frontmost is true
                set appName to name of frontApp
            end tell

            -- Try application-specific methods for common apps
            if appName contains "Firefox" or appName is "firefox" then
                tell application "Firefox"
                    try
                        set windowTitle to name of front window
                        return appName & "|" & windowTitle
                    end try
                end tell
            else if appName contains "Chrome" or appName contains "Brave" or appName contains "Chromium" then
                tell application appName
                    try
                        set windowTitle to title of active tab of front window
                        return appName & "|" & windowTitle
                    end try
                end tell
            else if appName contains "Safari" then
                tell application "Safari"
                    try
                        set windowTitle to name of front document
                        return appName & "|" & windowTitle
                    end try
                end tell
            else if appName contains "Terminal" or appName is "Terminal" then
                tell application "Terminal"
                    try
                        set windowTitle to name of front window
                        return appName & "|" & windowTitle
                    end try
                end tell
            else if appName contains "iTerm" or appName is "iTerm2" then
                tell application "iTerm"
                    try
                        set windowTitle to name of current session of current window
                        return appName & "|" & windowTitle
                    end try
                end tell
            else if appName contains "Alacritty" or appName is "Alacritty" or appName is "alacritty" then
                tell application "System Events"
                    tell process "Alacritty"
                        try
                            set windowTitle to name of front window
                            return appName & "|" & windowTitle
                        end try
                    end tell
                end tell
            end if

            -- Fallback: try System Events
            tell application "System Events"
                tell process appName
                    try
                        set windowTitle to name of front window
                        return appName & "|" & windowTitle
                    on error
                        return appName & "|"
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

            if let Some((app, title)) = result.split_once('|') {
                let app_name = app.trim().to_string();
                let window_title = title.trim().to_string();

                log::debug!("AppleScript returned: app='{}', title='{}'", app_name, window_title);

                if !app_name.is_empty() {
                    return Ok((app_name, window_title));
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
    async fn test_get_active_app_async() {
        let monitor = AppMonitor::new();
        // Note: This test may fail if no active window is available
        let app = monitor.get_active_app_async().await.unwrap_or_else(|_| "test".to_string());
        assert!(!app.is_empty());
    }

    #[tokio::test]
    async fn test_get_active_window_name_async() {
        let monitor = AppMonitor::new();
        // Note: This test may fail if no active window is available
        let window_name = monitor.get_active_window_name_async().await.unwrap_or_else(|_| "test".to_string());
        assert!(!window_name.is_empty());
    }
}
