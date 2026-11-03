//! Native clipboard abstraction adapted from CyberClip's clipboard modules.
use anyhow::Result;
#[cfg(target_os = "linux")] mod linux_wayland;
#[cfg(any(target_os = "linux", target_os = "windows", target_os = "macos"))] mod native;

pub fn set_text(text: &str) -> Result<()> {
    #[cfg(target_os = "linux")] {
        if std::env::var_os("WAYLAND_DISPLAY").is_some() { return linux_wayland::set(text.as_bytes()); }
        return native::set_text(text);
    }
    #[cfg(any(target_os = "windows", target_os = "macos"))] { return native::set_text(text); }
    #[cfg(not(any(target_os = "linux", target_os = "windows", target_os = "macos")))] {
        let _ = text; anyhow::bail!("Clipboard backend is unsupported on this operating system")
    }
}

pub fn get_text() -> Result<String> {
    #[cfg(target_os = "linux")] {
        if std::env::var_os("WAYLAND_DISPLAY").is_some() { return linux_wayland::get_text(); }
        return native::get_text();
    }
    #[cfg(any(target_os = "windows", target_os = "macos"))] { return native::get_text(); }
    #[cfg(not(any(target_os = "linux", target_os = "windows", target_os = "macos")))] {
        anyhow::bail!("Clipboard backend is unsupported on this operating system")
    }
}

#[cfg(target_os = "linux")]
pub fn serve(payload: &std::path::Path, ready: &std::path::Path) -> Result<()> { linux_wayland::serve(payload, ready) }
