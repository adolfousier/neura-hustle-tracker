#[cfg(target_os = "linux")]
#[derive(Debug)]
pub struct ProcessInfo {
    pub has_tmux: bool,
    pub tmux_session: Option<String>,
    pub tmux_window: Option<String>,
    pub editor_info: Option<EditorInfo>,
}

#[cfg(target_os = "linux")]
#[derive(Debug)]
pub struct EditorInfo {
    pub filename: String,
    pub filepath: String,
}

/// Inspect process tree to find tmux sessions and editors
#[cfg(target_os = "linux")]
pub fn inspect_process_tree(pid: u64) -> Option<ProcessInfo> {
    let mut info = ProcessInfo {
        has_tmux: false,
        tmux_session: None,
        tmux_window: None,
        editor_info: None,
    };

    // Get child processes recursively
    let children = get_child_processes(pid);

    for child_pid in children {
        if let Some(cmdline) = get_process_cmdline(child_pid) {
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
                children.extend(get_child_processes(child_pid));
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