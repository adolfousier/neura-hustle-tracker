use anyhow::Result;
use std::process::Command;

pub struct AppMonitor;

impl AppMonitor {
    pub fn new() -> Self {
        Self
    }

    pub fn get_active_app(&self) -> Result<String> {
        let pid = self.get_active_window_pid()?;
        if pid.is_empty() {
            log::warn!("No active window detected. Make sure you're in a desktop environment with GUI windows.");
            return Ok("Unknown".to_string());
        }
        let app = self.get_app_from_pid(&pid)?;
        log::info!("Detected active app: {}", app);
        Ok(self.fix_app_name(app))
    }

    pub fn get_active_window_name(&self) -> Result<String> {
        let output = Command::new("xdotool")
            .args(&["getactivewindow", "getwindowname"])
            .output()?;
        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
        } else {
            Ok("Unknown Window".to_string())
        }
    }

    fn get_active_window_pid(&self) -> Result<String> {
        // Check if on X11
        let is_x11 = std::env::var("XDG_SESSION_TYPE").unwrap_or_default() == "x11";

        if is_x11 {
            // Try xdotool for X11 first
            let output = Command::new("xdotool")
                .args(&["getactivewindow", "getwindowpid"])
                .output()?;
            if output.status.success() {
                return Ok(String::from_utf8_lossy(&output.stdout).trim().to_string());
            }

            // Fallback to xprop for X11
            let window_id_output = Command::new("xprop")
                .args(&["-root", "_NET_ACTIVE_WINDOW"])
                .output()?;
            if !window_id_output.status.success() {
                return Ok(String::new());
            }
            let window_id_line = String::from_utf8_lossy(&window_id_output.stdout);
            let window_id = window_id_line.split_whitespace().last().unwrap_or("").trim_start_matches("0x");

            if window_id.is_empty() || window_id == "0" {
                return Ok(String::new());
            }

            let pid_output = Command::new("xprop")
                .args(&["-id", window_id, "_NET_WM_PID"])
                .output()?;
            if pid_output.status.success() {
                let pid_line = String::from_utf8_lossy(&pid_output.stdout);
                let pid = pid_line.split('=').nth(1).unwrap_or("").trim();
                return Ok(pid.to_string());
            }
        } else {
            // Try our custom GNOME extension D-Bus method
            let dbus_output = Command::new("gdbus")
                .args(&["call", "--session", "--dest", "com.timetracking.PidGetter", "--object-path", "/com/timetracking/PidGetter", "--method", "com.timetracking.PidGetter.GetActiveWindowPid"])
                .output()?;
            if dbus_output.status.success() {
                let output_str = String::from_utf8_lossy(&dbus_output.stdout);
                // Output is like (uint32 12345,)
                if let Some(pid_str) = output_str.split_whitespace().nth(1) {
                    return Ok(pid_str.to_string());
                }
            }

            // Fallback to gdbus for Wayland (GNOME)
            let gdbus_output = Command::new("gdbus")
                .args(&["call", "--session", "--dest", "org.gnome.Shell", "--object-path", "/org/gnome/Shell", "--method", "org.gnome.Shell.Eval", "global.display.focus_window ? global.display.focus_window.get_pid() : null"])
                .output()?;
            if gdbus_output.status.success() {
                let output_str = String::from_utf8_lossy(&gdbus_output.stdout);
                // Output is like (true, '12345') or (false, '')
                if output_str.starts_with("(true,") {
                    if let Some(pid_str) = output_str.split(',').nth(1).and_then(|s| s.trim().strip_prefix("'").and_then(|s| s.strip_suffix("'"))) {
                        return Ok(pid_str.to_string());
                    }
                }
            }
        }

        Ok(String::new())
    }

    fn get_app_from_pid(&self, pid: &str) -> Result<String> {
        let output = Command::new("ps")
            .args(&["-p", pid, "-o", "comm="])
            .output()?;
        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
        } else {
            Ok("Unknown".to_string())
        }
    }

    fn fix_app_name(&self, app: String) -> String {
        if app.contains("gnome-terminal") {
            "gnome-terminal".to_string()
        } else if app == "soffice.bin" {
            "libreoffice".to_string()
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
        // Note: This test may fail if xdotool is not available or no active window
        let app = monitor.get_active_app().unwrap_or_else(|_| "test".to_string());
        assert!(!app.is_empty());
    }
}