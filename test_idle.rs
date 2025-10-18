use anyhow::Result;
use zbus::Connection;

async fn check_wayland_idle_time() -> Result<u32> {
    let connection = zbus::Connection::session().await?;

    // Try GNOME Mutter Idle Monitor with proper monitor creation
    match get_mutter_idle_time(&connection).await {
        Ok(idle_time) => Ok(idle_time),
        Err(e1) => {
            println!("Mutter IdleMonitor failed: {}", e1);
            // Fallback: try GNOME Session Manager
            match connection.call_method(
                Some("org.gnome.SessionManager"),
                "/org/gnome/SessionManager/Presence",
                Some("org.gnome.SessionManager.Presence"),
                "GetIdleTime",
                &(),
            ).await {
                Ok(response) => {
                    let idle_time: u64 = response.body().deserialize()?;
                    Ok((idle_time / 1000) as u32)
                }
                Err(e2) => {
                    println!("SessionManager Presence failed: {}", e2);
                    // Try logind idle hint (systemd)
                    match connection.call_method(
                        Some("org.freedesktop.login1"),
                        "/org/freedesktop/login1/session/auto",
                        Some("org.freedesktop.login1.Session"),
                        "GetIdleHint",
                        &(),
                    ).await {
                        Ok(response) => {
                            let idle_hint: bool = response.body().deserialize()?;
                            // GetIdleHint returns boolean, not time
                            // If idle, assume high idle time; if not idle, assume 0
                            Ok(if idle_hint { 300 } else { 0 }) // 5 minutes or 0
                        }
                        Err(e3) => {
                            println!("logind IdleHint failed: {}", e3);
                            // Try org.freedesktop.ScreenSaver
                            match connection.call_method(
                                Some("org.freedesktop.ScreenSaver"),
                                "/org/freedesktop/ScreenSaver",
                                Some("org.freedesktop.ScreenSaver"),
                                "GetSessionIdleTime",
                                &(),
                            ).await {
                                Ok(response) => {
                                    let idle_time: u64 = response.body().deserialize()?;
                                    Ok((idle_time / 1000) as u32)
                                }
                                Err(e4) => {
                                    println!("ScreenSaver GetSessionIdleTime failed: {}", e4);
                                    // Try alternative ScreenSaver method
                                    match connection.call_method(
                                        Some("org.freedesktop.ScreenSaver"),
                                        "/org/freedesktop/ScreenSaver",
                                        Some("org.freedesktop.ScreenSaver"),
                                        "GetActiveTime",
                                        &(),
                                    ).await {
                                        Ok(response) => {
                                            let active_time: u64 = response.body().deserialize()?;
                                            Ok((active_time / 1000) as u32)
                                        }
                                        Err(e5) => {
                                            println!("ScreenSaver GetActiveTime failed: {}", e5);
                                            // Last resort: try to detect if we can connect to GNOME Shell
                                            // If GNOME Shell is responding, assume some activity
                                            match connection.call_method(
                                                Some("org.gnome.Shell"),
                                                "/org/gnome/Shell",
                                                Some("org.gnome.Shell"),
                                                "Eval",
                                                &("1 + 1".to_string()),
                                            ).await {
                                                Ok(_) => Ok(0), // GNOME Shell responsive, assume active
                                                Err(e6) => {
                                                    println!("GNOME Shell check failed: {}", e6);
                                                    Err(anyhow::anyhow!("All idle detection methods failed"))
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

// Properly create and query Mutter Idle Monitor
async fn get_mutter_idle_time(connection: &zbus::Connection) -> Result<u32> {
    // First try the existing core monitor
    match connection.call_method(
        Some("org.gnome.Mutter.IdleMonitor"),
        "/org/gnome/Mutter/IdleMonitor/Core",
        Some("org.gnome.Mutter.IdleMonitor"),
        "GetIdletime",
        &(),
    ).await {
        Ok(response) => {
            let idle_time: u64 = response.body().deserialize()?;
            return Ok((idle_time / 1000) as u32);
        }
        Err(_) => {
            // Core monitor doesn't exist, try to create one
            match connection.call_method(
                Some("org.gnome.Mutter.IdleMonitor"),
                "/org/gnome/Mutter/IdleMonitor/Core",
                Some("org.gnome.Mutter.IdleMonitor"),
                "CreateMonitor",
                &(),
            ).await {
                Ok(response) => {
                    let monitor_path: String = response.body().deserialize()?;
                    println!("Created idle monitor at: {}", monitor_path);

                    // Now query the created monitor
                    match connection.call_method(
                        Some("org.gnome.Mutter.IdleMonitor"),
                        monitor_path.as_str(),
                        Some("org.gnome.Mutter.IdleMonitor"),
                        "GetIdletime",
                        &(),
                    ).await {
                        Ok(response) => {
                            let idle_time: u64 = response.body().deserialize()?;
                            Ok((idle_time / 1000) as u32)
                        }
                        Err(e) => {
                            println!("Failed to query created monitor: {}", e);
                            Err(anyhow::anyhow!("Created monitor query failed: {}", e))
                        }
                    }
                }
                Err(e) => {
                    println!("Failed to create idle monitor: {}", e);
                    Err(anyhow::anyhow!("Idle monitor creation failed: {}", e))
                }
            }
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    println!("Testing Wayland D-Bus idle detection...");

    match check_wayland_idle_time().await {
        Ok(idle_time) => {
            println!("✅ Success! Idle time: {} seconds", idle_time);
        }
        Err(e) => {
            println!("❌ Failed: {}", e);
        }
    }

    Ok(())
}