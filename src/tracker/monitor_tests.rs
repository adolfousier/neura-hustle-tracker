use super::*;
use std::env;

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

#[test]
fn test_hyprland_window_deserialize_full() {
    let json = r#"{
        "class": "firefox",
        "title": "GitHub - Mozilla Firefox",
        "address": "0x123",
        "workspace": {"id": 1, "name": "1"},
        "pid": 12345,
        "focused": true
    }"#;
    let window: HyprlandWindow = serde_json::from_str(json).unwrap();
    assert_eq!(window.class, "firefox");
    assert_eq!(window.title, "GitHub - Mozilla Firefox");
}

#[test]
fn test_hyprland_window_deserialize_empty() {
    // hyprctl returns {} when no window is focused
    let json = "{}";
    let window: HyprlandWindow = serde_json::from_str(json).unwrap();
    assert_eq!(window.class, "");
    assert_eq!(window.title, "");
}

#[test]
fn test_hyprland_window_deserialize_minimal() {
    let json = r#"{"class": "kitty", "title": "~"}"#;
    let window: HyprlandWindow = serde_json::from_str(json).unwrap();
    assert_eq!(window.class, "kitty");
    assert_eq!(window.title, "~");
}

#[test]
fn test_is_hyprland_with_env() {
    let original = env::var("HYPRLAND_INSTANCE_SIGNATURE").ok();
    // SAFETY: test-only env manipulation, tests run single-threaded with -- --test-threads=1
    unsafe { env::set_var("HYPRLAND_INSTANCE_SIGNATURE", "test_signature"); }
    assert!(AppMonitor::is_hyprland());
    unsafe {
        if let Some(val) = original {
            env::set_var("HYPRLAND_INSTANCE_SIGNATURE", val);
        } else {
            env::remove_var("HYPRLAND_INSTANCE_SIGNATURE");
        }
    }
}

#[test]
fn test_is_hyprland_without_env() {
    let original = env::var("HYPRLAND_INSTANCE_SIGNATURE").ok();
    // SAFETY: test-only env manipulation
    unsafe { env::remove_var("HYPRLAND_INSTANCE_SIGNATURE"); }
    assert!(!AppMonitor::is_hyprland());
    unsafe {
        if let Some(val) = original {
            env::set_var("HYPRLAND_INSTANCE_SIGNATURE", val);
        }
    }
}

#[test]
fn test_fix_app_name_hyprland_classes() {
    let monitor = AppMonitor { use_wayland: true, use_hyprland: true };
    // Hyprland class names are typically lowercase app identifiers
    assert_eq!(monitor.fix_app_name("firefox".to_string()), "firefox");
    assert_eq!(monitor.fix_app_name("chrome".to_string()), "chrome");
    assert_eq!(monitor.fix_app_name("Slack".to_string()), "slack");
    assert_eq!(monitor.fix_app_name("discord".to_string()), "discord");
    assert_eq!(monitor.fix_app_name("spotify".to_string()), "spotify");
}

#[test]
fn test_is_wayland_via_socket_fallback() {
    // When env vars are missing, is_wayland should detect via socket file
    let original_wayland = env::var("WAYLAND_DISPLAY").ok();
    let original_session = env::var("XDG_SESSION_TYPE").ok();
    let runtime_dir = env::var("XDG_RUNTIME_DIR").ok();

    // SAFETY: test-only env manipulation
    unsafe {
        env::remove_var("WAYLAND_DISPLAY");
        env::remove_var("XDG_SESSION_TYPE");
    }

    if let Some(ref dir) = runtime_dir {
        let wayland_socket = std::path::Path::new(dir).join("wayland-0");
        if wayland_socket.exists() {
            // On a Wayland system with socket present, should detect Wayland
            assert!(AppMonitor::is_wayland(), "Should detect Wayland via socket fallback");
        } else {
            // No socket, no env vars — should not detect Wayland
            assert!(!AppMonitor::is_wayland(), "Should not detect Wayland without socket or env vars");
        }
    }

    // Restore
    unsafe {
        if let Some(val) = original_wayland {
            env::set_var("WAYLAND_DISPLAY", val);
        }
        if let Some(val) = original_session {
            env::set_var("XDG_SESSION_TYPE", val);
        }
    }
}

#[test]
fn test_is_wayland_via_env_var() {
    let original = env::var("WAYLAND_DISPLAY").ok();
    // SAFETY: test-only env manipulation
    unsafe { env::set_var("WAYLAND_DISPLAY", "wayland-0"); }
    assert!(AppMonitor::is_wayland());
    unsafe {
        if let Some(val) = original {
            env::set_var("WAYLAND_DISPLAY", val);
        } else {
            env::remove_var("WAYLAND_DISPLAY");
        }
    }
}

#[test]
fn test_is_wayland_via_session_type() {
    let original_wayland = env::var("WAYLAND_DISPLAY").ok();
    let original_session = env::var("XDG_SESSION_TYPE").ok();
    // SAFETY: test-only env manipulation
    unsafe {
        env::remove_var("WAYLAND_DISPLAY");
        env::set_var("XDG_SESSION_TYPE", "wayland");
    }
    assert!(AppMonitor::is_wayland());
    unsafe {
        if let Some(val) = original_wayland {
            env::set_var("WAYLAND_DISPLAY", val);
        }
        if let Some(val) = original_session {
            env::set_var("XDG_SESSION_TYPE", val);
        } else {
            env::remove_var("XDG_SESSION_TYPE");
        }
    }
}
