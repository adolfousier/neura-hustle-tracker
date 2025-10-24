#[cfg(target_os = "windows")]
use std::process::Command;

/// Enhance window title with process inspection for Windows
#[cfg(target_os = "windows")]
pub fn enhance_windows_title(title: &str, process_id: u64) -> String {
    // Use PowerShell to inspect child processes
    let script = format!(
        r#"
        $process = Get-Process -Id {0}
        $enhanced = "{1}"
        if ($process) {{
            $processName = $process.Name
            if ($processName -like "*terminal*" -or $processName -like "*cmd*" -or $processName -like "*powershell*" -or $processName -like "*wt*") {{
                try {{
                    $children = Get-Process | Where-Object {{ $_.Parent.Id -eq {0} }}
                    $tmux = $children | Where-Object {{ $_.Name -like "*tmux*" }}
                    if ($tmux) {{
                        $enhanced = "tmux: session - {1}"
                    }} else {{
                        $vim = $children | Where-Object {{ $_.Name -like "*vim*" -or $_.Name -like "*nvim*" }}
                        if ($vim) {{
                            $file = ($vim | Select-Object -First 1).CommandLine -split ' ' | Select-Object -Last 1
                            if ($file -and $file -ne $vim.Name) {{
                                $enhanced = "$file - {1}"
                            }}
                        }}
                    }}
                }} catch {{
                    # Ignore errors
                }}
            }}
        }}
        $enhanced
        "#,
        process_id, title
    );

    match Command::new("powershell")
        .arg("-NoProfile")
        .arg("-Command")
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