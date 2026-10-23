// Optional local compatibility fallback; no shell, network, or source modification.
pub fn extract(path: &std::path::Path) -> Option<Vec<String>> {
    use std::process::{Command, Stdio};
    let executable =
        std::env::var_os("PAPER_MASTER_PDFTOTEXT").unwrap_or_else(|| "pdftotext".into());
    let output =
        std::env::temp_dir().join(format!("paper-master-pdf-{}.txt", uuid::Uuid::new_v4()));
    let mut command = Command::new(executable);
    command
        .args(["-enc", "UTF-8"])
        .arg(path)
        .arg(&output)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }
    let mut child = command.spawn().ok()?;
    let started = std::time::Instant::now();
    let success = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status.success(),
            Ok(None) if started.elapsed().as_secs() < 90 => {
                std::thread::sleep(std::time::Duration::from_millis(50))
            }
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                break false;
            }
        }
    };
    let result = if success && std::fs::metadata(&output).is_ok_and(|m| m.len() <= 32_000_000) {
        std::fs::read_to_string(&output).ok().map(|text| {
            text.trim_end_matches('\u{c}')
                .split('\u{c}')
                .map(str::to_string)
                .collect()
        })
    } else {
        None
    };
    let _ = std::fs::remove_file(output);
    result
}
