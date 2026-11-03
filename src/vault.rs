use anyhow::{Context, Result, bail};
use argon2::{Algorithm, Argon2, Params, Version};
use chacha20poly1305::{XChaCha20Poly1305, XNonce, aead::{Aead, KeyInit}};
use rand::{RngCore, rngs::OsRng};
use serde::{Deserialize, Serialize};
use std::{fs::{self, OpenOptions}, io::Write, path::{Path, PathBuf}};
use zeroize::Zeroizing;
use crate::model::{Note, VaultData};

const MAGIC_ENCRYPTED: &[u8; 8] = b"CYBNVLT1";
const MAGIC_PLAIN: &[u8; 8] = b"CYBNPLN1";
const SALT_LEN: usize = 16;
const NONCE_LEN: usize = 24;
const PENDING_MAGIC: &[u8; 8] = b"CYBNPEN1";

#[derive(Serialize, Deserialize)]
struct Plaintext { format_version: u32, notes: Vec<Note> }
#[derive(Serialize)]
struct PlaintextRef<'a> { format_version: u32, notes: &'a [Note] }

pub fn data_dir() -> Result<PathBuf> {
    let base = std::env::var_os("XDG_DATA_HOME").map(PathBuf::from)
        .or_else(dirs::data_dir)
        .or_else(|| dirs::home_dir().map(|h| h.join(".local/share")))
        .context("Cannot determine the local application data directory")?;
    let dir = base.join("cybernotes");
    fs::create_dir_all(&dir).with_context(|| format!("Cannot create {}", dir.display()))?;
    set_dir_permissions(&dir)?;
    Ok(dir)
}
pub fn vault_path() -> Result<PathBuf> { Ok(data_dir()?.join("vault.cbn")) }
fn set_dir_permissions(path: &Path) -> Result<()> {
    #[cfg(unix)] { use std::os::unix::fs::PermissionsExt; fs::set_permissions(path, fs::Permissions::from_mode(0o700))?; }
    Ok(())
}
fn atomic_write(path: &Path, bytes: &[u8]) -> Result<()> {
    let parent = path.parent().context("Storage path has no parent")?;
    let temp = parent.join(format!(".write-{}-{}.tmp", std::process::id(), rand::random::<u64>()));
    let mut options = OpenOptions::new(); options.write(true).create_new(true);
    #[cfg(unix)] { use std::os::unix::fs::OpenOptionsExt; options.mode(0o600); }
    let result = (|| -> Result<()> {
        let mut file = options.open(&temp).context("Cannot create temporary storage file")?;
        file.write_all(bytes).context("Cannot write temporary storage file")?;
        file.sync_all().context("Cannot sync temporary storage file")?;
        replace_file(&temp, path).context("Cannot atomically replace storage file")?;
        #[cfg(unix)] { if let Ok(dir) = fs::File::open(parent) { let _ = dir.sync_all(); } }
        Ok(())
    })();
    if result.is_err() { let _ = fs::remove_file(&temp); }
    result
}

#[cfg(not(target_os = "windows"))]
fn replace_file(temp: &Path, destination: &Path) -> Result<()> {
    fs::rename(temp, destination)?;
    Ok(())
}

#[cfg(target_os = "windows")]
fn replace_file(temp: &Path, destination: &Path) -> Result<()> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::{
        MoveFileExW, MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH,
    };
    let source: Vec<u16> = temp.as_os_str().encode_wide().chain(std::iter::once(0)).collect();
    let target: Vec<u16> = destination.as_os_str().encode_wide().chain(std::iter::once(0)).collect();
    let result = unsafe {
        MoveFileExW(
            source.as_ptr(),
            target.as_ptr(),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
        )
    };
    if result == 0 { return Err(std::io::Error::last_os_error().into()); }
    Ok(())
}

fn derive_key(password: &[u8], salt: &[u8]) -> Result<Zeroizing<[u8; 32]>> {
    let params = Params::new(19 * 1024, 2, 1, Some(32)).map_err(|e| anyhow::anyhow!("Invalid Argon2 parameters: {e}"))?;
    let argon = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);
    let mut key = Zeroizing::new([0u8; 32]);
    argon.hash_password_into(password, salt, &mut *key).map_err(|e| anyhow::anyhow!("Password key derivation failed: {e}"))?;
    Ok(key)
}
pub fn exists() -> Result<bool> { Ok(vault_path()?.exists()) }
pub fn is_encrypted() -> Result<bool> {
    let path = vault_path()?;
    if !path.exists() { return Ok(false); }
    let mut header = [0u8; 8];
    use std::io::Read;
    let mut file = fs::File::open(path)?;
    file.read_exact(&mut header).context("Vault header is truncated")?;
    match &header {
        MAGIC_ENCRYPTED => Ok(true),
        MAGIC_PLAIN => Ok(false),
        _ => bail!("Invalid or unsupported CyberNotes vault format"),
    }
}
pub fn load(password: Option<&[u8]>) -> Result<VaultData> {
    let path = vault_path()?;
    let bytes = Zeroizing::new(fs::read(&path).with_context(|| format!("Cannot read vault {}", path.display()))?);
    if bytes.len() < 8 { bail!("Vault is truncated or has an unsupported format"); }
    let plaintext = if &bytes[..8] == MAGIC_PLAIN {
        Zeroizing::new(bytes[8..].to_vec())
    } else if &bytes[..8] == MAGIC_ENCRYPTED {
        let password = password.context("This vault is password-protected; a master password is required")?;
        if bytes.len() < 8 + SALT_LEN + NONCE_LEN + 16 { bail!("Encrypted vault is truncated"); }
        let salt = &bytes[8..8 + SALT_LEN];
        let nonce_start = 8 + SALT_LEN;
        let nonce = XNonce::from_slice(&bytes[nonce_start..nonce_start + NONCE_LEN]);
        let key = derive_key(password, salt)?;
        let cipher = XChaCha20Poly1305::new_from_slice(&key[..]).map_err(|e| anyhow::anyhow!("Cannot initialize encryption: {e}"))?;
        Zeroizing::new(cipher.decrypt(nonce, &bytes[nonce_start + NONCE_LEN..])
            .map_err(|_| anyhow::anyhow!("Could not unlock vault: incorrect password or damaged data"))?)
    } else { bail!("Invalid or unsupported CyberNotes vault format"); };
    let decoded: Plaintext = serde_json::from_slice(&plaintext).context("Vault data is invalid")?;
    if decoded.format_version != 1 { bail!("Unsupported vault data version: {}", decoded.format_version); }
    Ok(VaultData { format_version: decoded.format_version, notes: decoded.notes })
}
pub fn save(password: Option<&[u8]>, data: &VaultData) -> Result<()> {
    let path = vault_path()?;
    let plaintext = Zeroizing::new(serde_json::to_vec(&PlaintextRef { format_version: 1, notes: &data.notes })?);
    let output = if let Some(password) = password {
        let mut salt = [0u8; SALT_LEN]; let mut nonce_bytes = [0u8; NONCE_LEN];
        OsRng.fill_bytes(&mut salt); OsRng.fill_bytes(&mut nonce_bytes);
        let key = derive_key(password, &salt)?;
        let cipher = XChaCha20Poly1305::new_from_slice(&key[..]).map_err(|e| anyhow::anyhow!("Cannot initialize encryption: {e}"))?;
        let ciphertext = cipher.encrypt(XNonce::from_slice(&nonce_bytes), plaintext.as_ref()).map_err(|_| anyhow::anyhow!("Vault encryption failed"))?;
        let mut out = Vec::with_capacity(8 + SALT_LEN + NONCE_LEN + ciphertext.len());
        out.extend_from_slice(MAGIC_ENCRYPTED); out.extend_from_slice(&salt); out.extend_from_slice(&nonce_bytes); out.extend_from_slice(&ciphertext); out
    } else {
        let mut out = Vec::with_capacity(8 + plaintext.len()); out.extend_from_slice(MAGIC_PLAIN); out.extend_from_slice(plaintext.as_ref()); out
    };
    atomic_write(&path, &output)
}
pub fn create(password: Option<&[u8]>) -> Result<VaultData> {
    if exists()? { bail!("A vault already exists"); }
    let data = VaultData { format_version: 1, notes: Vec::new() };
    save(password, &data)?; Ok(data)
}

// The pending inbox is encrypted with a random per-installation device key.
// Both files are private (0600) and remain local to the user's data directory.
fn pending_paths() -> Result<(PathBuf, PathBuf)> { let dir = data_dir()?; Ok((dir.join("pending.key"), dir.join("pending.cbn"))) }
pub fn queue_pending(note: &Note) -> Result<()> {
    let (key_path, inbox_path) = pending_paths()?;
    let key_bytes = if key_path.exists() {
        let bytes = Zeroizing::new(fs::read(&key_path)?);
        if bytes.len()!=32 { bail!("Pending inbox key is invalid; preserve pending files for recovery"); }
        let mut k=[0u8;32]; k.copy_from_slice(&bytes); k
    } else {
        let mut k=[0u8;32]; OsRng.fill_bytes(&mut k); atomic_write(&key_path,&k)?; k
    };
    let key = Zeroizing::new(key_bytes);
    let mut notes: Vec<Note> = if inbox_path.exists() { read_pending(&key, &inbox_path)? } else { Vec::new() };
    notes.push(note.clone());
    let encoded = Zeroizing::new(serde_json::to_vec(&notes)?);
    let mut nonce=[0u8;NONCE_LEN]; OsRng.fill_bytes(&mut nonce);
    let cipher=XChaCha20Poly1305::new_from_slice(&key[..]).map_err(|e| anyhow::anyhow!("Cannot initialize pending inbox encryption: {e}"))?;
    let ciphertext=cipher.encrypt(XNonce::from_slice(&nonce),encoded.as_ref()).map_err(|_|anyhow::anyhow!("Could not encrypt pending note"))?;
    let mut out=Vec::with_capacity(8+NONCE_LEN+ciphertext.len());out.extend_from_slice(PENDING_MAGIC);out.extend_from_slice(&nonce);out.extend_from_slice(&ciphertext);
    atomic_write(&inbox_path,&out)?;
    Ok(())
}
fn read_pending(key: &[u8;32], path: &Path) -> Result<Vec<Note>> {
    let bytes=fs::read(path)?; if bytes.len()<8+NONCE_LEN+16 || &bytes[..8]!=PENDING_MAGIC { bail!("Pending inbox is damaged; files were preserved"); }
    let cipher=XChaCha20Poly1305::new_from_slice(key).map_err(|e|anyhow::anyhow!("Cannot initialize pending inbox decryption: {e}"))?;
    let plain=Zeroizing::new(cipher.decrypt(XNonce::from_slice(&bytes[8..8+NONCE_LEN]),&bytes[8+NONCE_LEN..]).map_err(|_|anyhow::anyhow!("Cannot decrypt pending inbox; files were preserved"))?);
    Ok(serde_json::from_slice(&plain).context("Pending inbox data is invalid")?)
}
pub fn import_pending(password: Option<&[u8]>, data: &mut VaultData) -> Result<usize> {
    let (key_path,inbox_path)=pending_paths()?; if !inbox_path.exists() { return Ok(0); }
    let key_bytes=Zeroizing::new(fs::read(&key_path).context("Pending inbox key is missing; pending files were preserved")?);
    if key_bytes.len()!=32 { bail!("Pending inbox key is invalid; pending files were preserved"); }
    let mut key_array=[0u8;32];key_array.copy_from_slice(&key_bytes);
    let key=Zeroizing::new(key_array);
    let notes=read_pending(&key,&inbox_path)?;
    let mut count=0usize;
    for note in notes {
        if !data.notes.iter().any(|existing| existing.id == note.id) {
            data.notes.push(note);
            count += 1;
        }
    }
    if count > 0 {
        save(password,data).context("Could not save imported pending notes; pending files were preserved")?;
    }
    fs::remove_file(&inbox_path)?;
    let _=fs::remove_file(&key_path);
    Ok(count)
}
