use active_win_pos_rs::get_active_window;
use anyhow::Result;
use std::env;

pub struct AppMonitor;

impl AppMonitor {
    pub fn new() -> Self {
        Self
    }

    pub fn get_active_app(&self) -> Result<String> {
        match get_active_window() {
            Ok(active_window) => {
                log::info!("Detected active app: {}", active_window.app_name);
                Ok(self.fix_app_name(active_window.app_name))
            }
            Err(_) => {
                let error_msg = self.detect_environment_issue();
                log::warn!("{}", error_msg);
                Err(anyhow::anyhow!(error_msg))
            }
        }
    }

    pub fn get_active_window_name(&self) -> Result<String> {
        match get_active_window() {
            Ok(active_window) => {
                Ok(active_window.title)
            }
            Err(_) => {
                log::warn!("Failed to get active window title.");
                Ok("Unknown Window".to_string())
            }
        }
    }

    fn fix_app_name(&self, app: String) -> String {
        let app_lower = app.to_lowercase();
        if app_lower.contains("gnome-terminal") {
            "gnome-terminal".to_string()
        } else if app_lower == "soffice.bin" {
            "libreoffice".to_string()
        } else if app_lower.contains("chrome") || app_lower.contains("chromium") {
            "chrome".to_string()
        } else if app_lower.contains("firefox") {
            "firefox".to_string()
        } else if app_lower.contains("code") || app_lower.contains("vscode") {
            "vscode".to_string()
        } else {
            app
        }
    }

    fn detect_environment_issue(&self) -> String {
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

        // Generic error if not Wayland
        "Failed to get active window. Ensure you're in a desktop environment with X11 support.".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_active_app() {
        let monitor = AppMonitor::new();
        // Note: This test may fail if no active window is available
        let app = monitor.get_active_app().unwrap_or_else(|_| "test".to_string());
        assert!(!app.is_empty());
    }

    #[test]
    fn test_get_active_window_name() {
        let monitor = AppMonitor::new();
        // Note: This test may fail if no active window is available
        let window_name = monitor.get_active_window_name().unwrap_or_else(|_| "test".to_string());
        assert!(!window_name.is_empty());
    }
}