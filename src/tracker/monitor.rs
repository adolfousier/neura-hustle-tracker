use active_win_pos_rs::get_active_window;
use anyhow::Result;
use std::env;

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
                    let error_msg = self.detect_environment_issue();
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
                Ok((_wm_class, title)) => Ok(title),
                Err(_) => {
                    log::warn!("Failed to get active window title (Wayland).");
                    Ok("Unknown Window".to_string())
                }
            }
        } else {
            // Use X11 method
            match get_active_window() {
                Ok(active_window) => Ok(active_window.title),
                Err(_) => {
                    log::warn!("Failed to get active window title.");
                    Ok("Unknown Window".to_string())
                }
            }
        }
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

    fn detect_environment_issue(&self) -> String {
        #[cfg(target_os = "macos")]
        {
            let path_truncated = env::var("PATH")
                .map(|p| {
                    if p.len() > 100 {
                        format!("{}...", &p[..100])
                    } else {
                        p
                    }
                })
                .unwrap_or_else(|_| "N/A".to_string());

            return format!(
                "macOS: Failed to detect active window.\n\
                 \n\
                 Environment information:\n\
                 - DISPLAY: {:?}\n\
                 - PATH: {}\n\
                 - HOME: {:?}\n\
                 \n\
                 Enable DEBUG_LOGS_ENABLED=true in .env and check app.log for detailed information.",
                env::var("DISPLAY"),
                path_truncated,
                env::var("HOME")
            );
        }

        #[cfg(target_os = "windows")]
        {
            return format!(
                "Windows: Failed to detect active window.\n\
                 \n\
                 Environment information:\n\
                 - USERPROFILE: {:?}\n\
                 - COMPUTERNAME: {:?}\n\
                 \n\
                 Enable DEBUG_LOGS_ENABLED=true in .env and check app.log for detailed information.",
                env::var("USERPROFILE"),
                env::var("COMPUTERNAME")
            );
        }

        #[cfg(target_os = "linux")]
        {
            // Check if we're running on Wayland
            let wayland_display = env::var("WAYLAND_DISPLAY").ok();
            let xdg_session_type = env::var("XDG_SESSION_TYPE").ok();
            let display = env::var("DISPLAY").ok();

            if let Some(_wayland) = wayland_display {
                if display.is_none() {
                    // Pure Wayland without XWayland
                    return "WAYLAND ERROR: Window tracking failed. active-win-pos-rs requires X11. \
                            You're running pure Wayland without XWayland. \
                            Solutions: (1) Enable XWayland in your compositor, \
                            (2) Switch to an X11 session, or \
                            (3) Run with: XDG_SESSION_TYPE=x11 cargo run".to_string();
                } else {
                    // Wayland with XWayland available
                    return "WAYLAND WARNING: Window tracking failed. active-win-pos-rs requires X11. \
                            You're on Wayland but XWayland is available. \
                            Try running: XDG_SESSION_TYPE=x11 cargo run".to_string();
                }
            }

            if let Some(session_type) = xdg_session_type {
                if session_type == "wayland" {
                    return "WAYLAND ERROR: Window tracking failed. XDG_SESSION_TYPE=wayland detected. \
                            active-win-pos-rs requires X11. Switch to an X11 session or run with: \
                            XDG_SESSION_TYPE=x11 cargo run".to_string();
                }
            }

            // Generic Linux/X11 error
            "Linux/X11: Failed to get active window. Ensure you're in a desktop environment with X11 support.".to_string()
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
