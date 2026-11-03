use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Note {
    pub id: String,
    pub title: String,
    pub body: String,
    pub pinned: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct VaultData {
    pub format_version: u32,
    pub notes: Vec<Note>,
}

impl Note {
    pub fn new(title: String, body: String) -> Self {
        let now = Utc::now();
        Self { id: make_id(), title, body, pinned: false, created_at: now, updated_at: now }
    }
}

fn make_id() -> String {
    use rand::{RngCore, rngs::OsRng};
    let mut bytes = [0u8; 6];
    OsRng.fill_bytes(&mut bytes);
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
