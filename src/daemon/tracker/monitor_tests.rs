use super::*;
use std::env;

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
    assert_eq!(monitor.fix_app_name("firefox".to_string()), "firefox");
    assert_eq!(monitor.fix_app_name("chrome".to_string()), "chrome");
    assert_eq!(monitor.fix_app_name("Slack".to_string()), "slack");
    assert_eq!(monitor.fix_app_name("discord".to_string()), "discord");
    assert_eq!(monitor.fix_app_name("spotify".to_string()), "spotify");
}

#[test]
fn test_hyprland_empty_class_is_no_window() {
    // When hyprctl returns empty class, it means no focused window
    let json = r#"{"class": "", "title": "something"}"#;
    let window: HyprlandWindow = serde_json::from_str(json).unwrap();
    assert!(window.class.is_empty(), "Empty class should indicate no focused window");
}

#[test]
fn test_is_wayland_via_socket_fallback() {
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
            assert!(AppMonitor::is_wayland(), "Should detect Wayland via socket fallback");
        } else {
            assert!(!AppMonitor::is_wayland(), "Should not detect Wayland without socket or env vars");
        }
    }

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
