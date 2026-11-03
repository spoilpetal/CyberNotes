mod clipboard;
mod editor;
mod model;
mod tui;
mod vault;

use anyhow::{Context, Result, bail};
use clap::{Parser, Subcommand};
use model::{Note, VaultData};
use std::{io::{self, Write}, path::PathBuf};
use zeroize::Zeroizing;

#[derive(Parser)]
#[command(name = "cybernotes", version, about = "Local-first terminal notes")]
struct Cli { #[command(subcommand)] command: Option<Action> }
#[derive(Subcommand)]
enum Action {
    /// Create a note without prompting for the master password.
    Add { #[arg(short, long)] title: Option<String>, #[arg(short, long)] body: Option<String> },
    List,
    /// Show full note IDs, titles and searchable keywords.
    #[command(name = "showid", aliases = ["ids", "show-id"])] ShowId { query: Option<String> },
    Show { id: String }, Search { query: String }, Edit { id: String }, Delete { id: String },
    Pin { id: String }, Unpin { id: String }, Copy { id: String },
    /// Create a note from the current system clipboard text (vault unlock required).
    Paste { #[arg(short, long)] title: Option<String> },
    Tui,
    /// Initial interactive setup; also called by install.sh.
    Setup,
    /// Manage optional master-password protection.
    Password { #[command(subcommand)] action: PasswordAction },
    /// Internal detached Wayland clipboard provider; not a user command.
    #[command(name = "__clipboard-serve", hide = true)] ClipboardServe { payload: PathBuf, ready: PathBuf },
}
#[derive(Subcommand)]
enum PasswordAction { Set, #[command(alias = "reset")] Change, Disable }

fn password(confirm: bool) -> Result<Zeroizing<String>> {
    let first = Zeroizing::new(rpassword::prompt_password("Master password: ")?);
    if first.is_empty() { bail!("Master password cannot be empty"); }
    if confirm {
        let second = Zeroizing::new(rpassword::prompt_password("Confirm master password: ")?);
        if *first != *second { bail!("Passwords do not match"); }
    }
    Ok(first)
}
fn new_password() -> Result<Zeroizing<String>> {
    let pass = password(true)?;
    if pass.len() < 8 { bail!("Use a master password of at least 8 characters"); }
    Ok(pass)
}
fn prompt(label: &str) -> Result<String> { print!("{label}"); io::stdout().flush()?; let mut s=String::new(); io::stdin().read_line(&mut s)?; Ok(s.trim_end_matches(['\r','\n']).to_string()) }
fn setup() -> Result<()> {
    if vault::exists()? { println!("CyberNotes is already initialized."); return Ok(()); }
    println!("CyberNotes first-time setup");
    println!("A master password encrypts your notes at rest. Without one, notes are stored locally without encryption.");
    let answer=prompt("Enable master-password protection? [y/N] ")?;
    let data=if answer.eq_ignore_ascii_case("y") || answer.eq_ignore_ascii_case("yes") {
        let pass=new_password()?; vault::create(Some(pass.as_bytes()))?
    } else { vault::create(None)? };
    let _=data;
    println!("CyberNotes is ready. Your vault is stored locally.");
    Ok(())
}
fn unlock() -> Result<(Option<Zeroizing<String>>, VaultData)> {
    if !vault::exists()? { setup()?; }
    let pass=if vault::is_encrypted()? { Some(password(false)?) } else { None };
    let mut data=vault::load(pass.as_ref().map(|p|p.as_bytes()))?;
    let imported=vault::import_pending(pass.as_ref().map(|p|p.as_bytes()),&mut data)?;
    if imported>0 { eprintln!("Imported {imported} pending note(s) into your vault."); }
    Ok((pass,data))
}
fn save(pass: Option<&Zeroizing<String>>, data: &VaultData) -> Result<()> { vault::save(pass.map(|p|p.as_bytes()),data) }
fn resolve_id(data: &VaultData, id: &str) -> Result<String> {
    let matches: Vec<&Note> = data.notes.iter().filter(|n| n.id == id || n.id.starts_with(id)).collect();
    match matches.as_slice() { [note] => Ok(note.id.clone()), [] => bail!("No note found with ID '{id}'"), _ => bail!("Note ID prefix '{id}' is ambiguous; use more characters") }
}
fn find_mut<'a>(data: &'a mut VaultData, id: &str) -> Result<&'a mut Note> { let full=resolve_id(data,id)?; data.notes.iter_mut().find(|n|n.id==full).context("Resolved note disappeared") }
fn find<'a>(data: &'a VaultData, id: &str) -> Result<&'a Note> { let full=resolve_id(data,id)?; data.notes.iter().find(|n|n.id==full).context("Resolved note disappeared") }
fn make_note(title: Option<String>, body: Option<String>) -> Result<Note> {
    let title = match title { Some(s) => s, None => prompt("Title: ")? };
    if title.trim().is_empty() { bail!("Title cannot be empty"); }
    let body = match body {
        Some(s) => s,
        None => {
            let answer = prompt("Add note text now? [y/N] ")?;
            if answer.eq_ignore_ascii_case("y") || answer.eq_ignore_ascii_case("yes") {
                editor::edit_text("", "NEW NOTE", false)?
                    .ok_or_else(|| anyhow::anyhow!("Note creation cancelled"))?
            } else { String::new() }
        }
    };
    Ok(Note::new(title, body))
}

fn add_without_unlock(title: Option<String>, body: Option<String>) -> Result<()> {
    if !vault::exists()? { setup()?; }
    let note=make_note(title,body)?;
    if vault::is_encrypted()? {
        let id=note.id.clone(); vault::queue_pending(&note)?;
        println!("Created pending note {id}. It will be imported the next time you unlock CyberNotes.");
    } else {
        let mut data=vault::load(None)?; let id=note.id.clone(); data.notes.push(note); vault::save(None,&data)?;
        println!("Created note {id}");
    }
    Ok(())
}
fn password_command(action: PasswordAction) -> Result<()> {
    match action {
        PasswordAction::Set => {
            if !vault::exists()? { vault::create(None)?; }
            if vault::is_encrypted()? { bail!("Password protection is already enabled. Use 'cybernotes password change'."); }
            let data=vault::load(None)?; let pass=new_password()?; vault::save(Some(pass.as_bytes()),&data)?;
            println!("Master-password protection enabled. Your vault is now encrypted.");
        }
        PasswordAction::Change => {
            if !vault::exists()? || !vault::is_encrypted()? { bail!("This vault does not currently have a master password. Use 'cybernotes password set'."); }
            let old=password(false)?; let mut data=vault::load(Some(old.as_bytes()))?;
            let imported=vault::import_pending(Some(old.as_bytes()),&mut data)?;
            let new=new_password()?; vault::save(Some(new.as_bytes()),&data)?;
            println!("Master password changed. {imported} pending note(s) imported.");
        }
        PasswordAction::Disable => {
            if !vault::exists()? || !vault::is_encrypted()? { bail!("Master-password protection is not enabled."); }
            let old=password(false)?; let mut data=vault::load(Some(old.as_bytes()))?;
            let imported=vault::import_pending(Some(old.as_bytes()),&mut data)?;
            println!("Disabling password protection stores the vault without encryption.");
            let confirm=prompt("Continue? [y/N] ")?;
            if confirm.eq_ignore_ascii_case("y") { vault::save(None,&data)?; println!("Password protection disabled. {imported} pending note(s) imported."); }
        }
    }
    Ok(())
}
fn sorted_notes(data:&VaultData)->Vec<&Note>{let mut v:Vec<_>=data.notes.iter().collect();v.sort_by_key(|n|(!n.pinned,std::cmp::Reverse(n.updated_at.timestamp_micros())));v}
fn note_keywords(note: &Note) -> String {
    let mut words = Vec::new();
    for word in format!("{} {}", note.title, note.body).split(|c: char| !c.is_alphanumeric()) {
        let word = word.trim();
        if word.chars().count() >= 3 {
            let lower = word.to_lowercase();
            if !words.contains(&lower) { words.push(lower); }
        }
        if words.len() >= 8 { break; }
    }
    if words.is_empty() { "-".to_string() } else { words.join(", ") }
}
fn run()->Result<()> {
    let cli=Cli::parse();
    if let Some(Action::ClipboardServe{payload,ready})=&cli.command { #[cfg(target_os="linux")] {return clipboard::serve(payload,ready);} #[cfg(not(target_os="linux"))] {let _=(payload,ready);bail!("Internal clipboard provider is only available on Linux");} }
    match cli.command {
        Some(Action::Setup)=>setup(),
        Some(Action::Password{action})=>password_command(action),
        Some(Action::Add{title,body})=>add_without_unlock(title,body),
        command=>{
            let (pass,mut data)=unlock()?;
            match command {
                None|Some(Action::Tui)=>tui::run(pass.as_ref().map(|p|p.as_str()),&mut data)?,
                Some(Action::List)=>for n in sorted_notes(&data){println!("{} {} {}",if n.pinned{"[PIN]"}else{"     "},n.id,n.title);},
                Some(Action::ShowId{query})=>{
                    println!("{:<14} {:<28} KEYWORDS", "NOTE ID", "TITLE");
                    let q=query.as_deref().map(str::to_lowercase);
                    for n in sorted_notes(&data) {
                        let keywords=note_keywords(n);
                        if q.as_ref().is_some_and(|q| !n.title.to_lowercase().contains(q.as_str()) && !n.body.to_lowercase().contains(q.as_str()) && !keywords.to_lowercase().contains(q.as_str())) { continue; }
                        println!("{:<14} {:<28} {}", n.id, n.title, keywords);
                    }
                },
                Some(Action::Show{id})=>{let n=find(&data,&id)?;println!("{}\n{}\n\n{}",n.title,"─".repeat(n.title.chars().count().max(1)),n.body);},
                Some(Action::Search{query})=>{let q=query.to_lowercase();for n in sorted_notes(&data).into_iter().filter(|n|n.title.to_lowercase().contains(&q)||n.body.to_lowercase().contains(&q)){println!("{} {} {}",if n.pinned{"[PIN]"}else{"     "},n.id,n.title);}},
                Some(Action::Edit{id})=>{
                    let n=find_mut(&mut data,&id)?;
                    let title=prompt(&format!("Title [{}]: ",n.title))?;
                    let original=n.body.clone();
                    if let Some(body)=editor::edit_text(&original, "EDIT NOTE", false)? {
                        if !title.trim().is_empty(){n.title=title;}
                        n.body=body;n.updated_at=chrono::Utc::now();save(pass.as_ref(),&data)?;
                    } else { println!("Edit cancelled; note unchanged."); }
                },
                Some(Action::Delete{id})=>{let full=resolve_id(&data,&id)?;let n=find(&data,&full)?;let yes=prompt(&format!("Delete '{}' ({})? [y/N] ",n.title,n.id))?;if yes.eq_ignore_ascii_case("y"){data.notes.retain(|n|n.id!=full);save(pass.as_ref(),&data)?;println!("Deleted.");}},
                Some(Action::Pin{id})=>{let n=find_mut(&mut data,&id)?;n.pinned=true;n.updated_at=chrono::Utc::now();save(pass.as_ref(),&data)?;},
                Some(Action::Unpin{id})=>{let n=find_mut(&mut data,&id)?;n.pinned=false;n.updated_at=chrono::Utc::now();save(pass.as_ref(),&data)?;},
                Some(Action::Copy{id})=>{clipboard::set_text(&find(&data,&id)?.body)?;println!("Note copied to system clipboard.");},
                Some(Action::Paste{title})=>{let body=clipboard::get_text()?;let note=make_note(title,Some(body))?;let id=note.id.clone();data.notes.push(note);save(pass.as_ref(),&data)?;println!("Created note {id} from clipboard.");},
                Some(Action::Setup)|Some(Action::Password{..})|Some(Action::Add{..})|Some(Action::ClipboardServe{..})=>unreachable!(),
            }
            Ok(())
        }
    }
}
fn main(){if let Err(e)=run(){eprintln!("Error: {e:#}");std::process::exit(1);}}

#[cfg(test)]
mod tests {
    use super::{note_keywords, resolve_id};
    use crate::model::{Note, VaultData};

    #[test]
    fn showid_keywords_are_unique_and_include_body_terms() {
        let note = Note::new("Rust notes".into(), "Rust ownership and borrowing".into());
        let keywords = note_keywords(&note);
        assert!(keywords.contains("rust"));
        assert!(keywords.contains("ownership"));
        assert_eq!(keywords.matches("rust").count(), 1);
    }

    #[test]
    fn note_ids_accept_unique_prefixes() {
        let note = Note::new("Example".into(), String::new());
        let full_id = note.id.clone();
        let data = VaultData { format_version: 1, notes: vec![note] };
        assert_eq!(resolve_id(&data, &full_id[..6]).unwrap(), full_id);
        assert!(resolve_id(&data, "not-a-note-id").is_err());
    }
}
