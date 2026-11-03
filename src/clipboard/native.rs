//! CyberClip's clipboard-rs native text backend, adapted for CyberNotes.
use anyhow::Result;
use clipboard_rs::{Clipboard, ClipboardContext};
pub fn set_text(text: &str) -> Result<()> {
    ClipboardContext::new().map_err(|e| anyhow::anyhow!("Could not open system clipboard: {e}"))?
        .set_text(text.to_owned()).map_err(|e| anyhow::anyhow!("Could not set system clipboard text: {e}"))?;
    Ok(())
}
pub fn get_text() -> Result<String> {
    ClipboardContext::new().map_err(|e| anyhow::anyhow!("Could not open system clipboard: {e}"))?
        .get_text().map_err(|e| anyhow::anyhow!("The system clipboard does not contain readable text: {e}"))
}
