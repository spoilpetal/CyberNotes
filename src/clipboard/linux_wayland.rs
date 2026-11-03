//! Persistent Wayland selection provider adapted from CyberClip's linux_wayland.rs.
use anyhow::{Context, Result, bail};
use std::{fs::{self, OpenOptions}, io::Write, path::{Path, PathBuf}, process::{Command, Stdio}, thread, time::{Duration, Instant, SystemTime, UNIX_EPOCH}};
use wl_clipboard_rs::copy::{MimeType, Options, Source};

fn runtime_dir() -> Result<PathBuf> {
    let base = std::env::var_os("XDG_RUNTIME_DIR").context("XDG_RUNTIME_DIR is not set; run CyberNotes inside your Wayland session")?;
    Ok(PathBuf::from(base).join("cybernotes"))
}
pub fn set(data: &[u8]) -> Result<()> {
    let dir = runtime_dir()?; fs::create_dir_all(&dir)?;
    #[cfg(unix)] { use std::os::unix::fs::PermissionsExt; fs::set_permissions(&dir, fs::Permissions::from_mode(0o700))?; }
    let stamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
    let base = format!("{}-{stamp}", std::process::id());
    let payload = dir.join(format!("{base}.data")); let ready = dir.join(format!("{base}.ready"));
    let mut opts = OpenOptions::new(); opts.write(true).create_new(true);
    #[cfg(unix)] { use std::os::unix::fs::OpenOptionsExt; opts.mode(0o600); }
    let mut file = opts.open(&payload).context("Could not create private clipboard staging file")?;
    file.write_all(data).context("Could not stage clipboard text")?; file.sync_all()?;
    let exe = std::env::current_exe().context("Could not locate CyberNotes executable")?;
    let mut child = Command::new(exe).arg("__clipboard-serve").arg(&payload).arg(&ready)
        .stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null()).spawn()
        .context("Could not start CyberNotes Wayland clipboard provider")?;
    let deadline = Instant::now() + Duration::from_secs(3);
    loop {
        if ready.exists() { let _ = fs::remove_file(&ready); return Ok(()); }
        if let Some(status) = child.try_wait()? { let _ = fs::remove_file(&payload); bail!("Wayland clipboard provider exited early ({status})"); }
        if Instant::now() >= deadline { let _ = child.kill(); let _ = child.wait(); let _ = fs::remove_file(&payload); bail!("Timed out starting Wayland clipboard provider"); }
        thread::sleep(Duration::from_millis(20));
    }
}
pub fn serve(payload: &Path, ready: &Path) -> Result<()> {
    let data = fs::read(payload).context("Could not read staged clipboard text")?; let _ = fs::remove_file(payload);
    let mut options = Options::new(); options.foreground(true);
    let prepared = options.prepare_copy(Source::Bytes(data.into()), MimeType::Text).context("Could not prepare Wayland clipboard offer")?;
    fs::write(ready, b"ready").context("Could not notify clipboard readiness")?;
    prepared.serve().context("Could not serve Wayland clipboard selection")?;
    let _ = fs::remove_file(ready); Ok(())
}
pub fn get_text() -> Result<String> {
    use std::io::Read;
    use wl_clipboard_rs::paste::{get_contents, ClipboardType, MimeType, Seat};
    let (mut pipe, _) = get_contents(ClipboardType::Regular, Seat::Unspecified, MimeType::Text)
        .map_err(|e| anyhow::anyhow!("Wayland clipboard is empty or has no text: {e}"))?;
    let mut bytes = Vec::new(); pipe.read_to_end(&mut bytes).context("Could not read Wayland clipboard")?;
    String::from_utf8(bytes).context("Clipboard text is not valid UTF-8")
}
