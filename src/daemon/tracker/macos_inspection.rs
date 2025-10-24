#[cfg(target_os = "macos")]
use std::process::Command;

/// Enhance window title with process inspection for macOS
#[cfg(target_os = "macos")]
pub fn enhance_macos_title(title: &str, process_id: u64) -> String {
    // Use AppleScript to inspect child processes
    let script = format!(
        r#"
        tell application "System Events"
            set frontApp to first application process whose frontmost is true
            set appName to name of frontApp
            set enhanced to "{}"
            if appName contains "Terminal" or appName is "Terminal" or appName contains "iTerm" or appName is "iTerm2" or appName contains "Alacritty" then
                try
                    -- Check for tmux
                    set tmuxCheck to do shell script "ps -p {} -o command= | grep tmux"
                    if tmuxCheck contains "tmux" then
                        set enhanced to "tmux: session - {}"
                    else
                        -- Check for vim
                        set vimCheck to do shell script "ps -p {} -o command= | grep -E '(vim|nvim)'"
                        if vimCheck contains "vim" or vimCheck contains "nvim" then
                            set file to do shell script "ps -p {} -o command= | sed 's/.*vim[^ ]* //' | head -1"
                            if file is not "" then
                                set enhanced to file & " - {}"
                            end if
                        end if
                    end if
                end try
            end if
            return enhanced
        end tell
        "#,
        title, process_id, title, process_id, process_id, title
    );

    match Command::new("osascript")
        .arg("-e")
        .arg(&script)
        .output()
    {
        Ok(output) if output.status.success() => {
            let enhanced = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !enhanced.is_empty() && enhanced != title {
                enhanced
            } else {
                title.to_string()
            }
        }
        _ => title.to_string(),
    }
}