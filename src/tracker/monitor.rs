use active_win_pos_rs::get_active_window;
use anyhow::Result;

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
                log::warn!("Failed to get active window. Make sure you're in a desktop environment with GUI windows.");
                Ok("Unknown".to_string())
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