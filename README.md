# CyberNotes

**CyberNotes** is a local-first sticky-notes application for the terminal. It provides a command-line interface and a keyboard-driven TUI for Linux (Wayland/Hyprland and X11), Windows, and macOS. It has no graphical desktop window, cloud account, remote server, or background synchronization.

- Repository: <https://github.com/spoilpetal/CyberNotes>
- Releases: <https://github.com/spoilpetal/CyberNotes/releases/latest>

## Features

- Create, list, view, search, edit, delete, pin, and unpin notes.
- Show full note IDs and keywords with `showid`.
- Copy a note body to the system clipboard.
- Create a note from clipboard text.
- Optional master-password encryption for the local vault.
- Password-free `add` command, including when the main vault is encrypted.
- Keyboard-driven TUI with a note list and detail panel.
- Built-in multi-line note editor; press **Esc**, then **C** to save or **Esc**, then **Q** to cancel.
- `cybernotes` full command and `cbn` short alias.
- Notes persist across restarts and reboots. Uninstalling the application does not delete note data.


## Screenshots & Preview

### TUI – Fresh View

![CyberNotes Fresh TUI View](https://raw.githubusercontent.com/spoilpetal/Picture_Vault/main/CyberNotes/V0.2.0/fresh_view.png)

### Adding a New Note Through TUI

![CyberNotes Add New Note UI](https://raw.githubusercontent.com/spoilpetal/Picture_Vault/main/CyberNotes/V0.2.0/add_new_ui.png)

### Built-in Editor – Fresh View

![CyberNotes Built-in Editor](https://raw.githubusercontent.com/spoilpetal/Picture_Vault/main/CyberNotes/V0.2.0/builtin_editor_fresh.png)

### Notes Added – TUI View

![CyberNotes Notes Added](https://raw.githubusercontent.com/spoilpetal/Picture_Vault/main/CyberNotes/V0.2.0/notes_added.png)

### CLI – Adding a Note to the Queue

![CyberNotes CLI Note Add Queue](https://raw.githubusercontent.com/spoilpetal/Picture_Vault/main/CyberNotes/V0.2.0/cli_note_add_que.png)

### CLI – Listing Notes & Queue Import

![CyberNotes Queue Added List Show](https://raw.githubusercontent.com/spoilpetal/Picture_Vault/main/CyberNotes/V0.2.0/que_added_list_show.png)

### CLI – Help Menu

![CyberNotes CLI Help](https://raw.githubusercontent.com/spoilpetal/Picture_Vault/main/CyberNotes/V0.2.0/help.png)

### Built-in Editor – Editing a Note

![CyberNotes While Editing](https://raw.githubusercontent.com/spoilpetal/Picture_Vault/main/CyberNotes/V0.2.0/while%20editing.png)


## Install

### Download a prebuilt binary

Open the [latest GitHub release](https://github.com/spoilpetal/CyberNotes/releases/latest) and download the package matching your operating system and CPU:

| Package | Operating system / CPU |
|---|---|
| `linux-x86_64` | Linux 64-bit Intel/AMD (Arch, Fedora, Debian, Ubuntu, etc.) |
| `linux-aarch64` | Linux ARM64 |
| `windows-x86_64` | Windows 64-bit |
| `macos-x86_64` | macOS Intel |
| `macos-aarch64` | macOS Apple Silicon |

Each release archive contains the executable, README, license, and third-party notice. SHA-256 checksum files are published beside the archives.

**Linux / macOS binary install (current user):**

```sh
# Replace the filename with the package you downloaded.
tar -xzf cybernotes-VERSION-linux-x86_64.tar.gz
mkdir -p "$HOME/.local/bin"
install -m 755 cybernotes "$HOME/.local/bin/cybernotes"
ln -sfn cybernotes "$HOME/.local/bin/cbn"
```

Ensure `$HOME/.local/bin` is in your `PATH`, then initialize the vault:

```sh
cybernotes setup
```

**Windows binary install:** extract `cybernotes.exe` from the release ZIP to a directory in your user `PATH`. To create the short `cbn` command, create a `cbn.cmd` file beside it containing:

```bat
@echo off
"%~dp0cybernotes.exe" %*
```

### Build and install from source

Rust stable (Rust 2024 edition) and Cargo are required. On Arch Linux:

```sh
sudo pacman -S --needed base-devel rust cargo
```

Then:

```sh
git clone https://github.com/spoilpetal/CyberNotes.git
cd CyberNotes
cargo check
cargo test
cargo build --release
```

Cargo creates `Cargo.lock` on the first build when it is not present. Keep that file in version control for reproducible builds.

Install for the current user (default):

```sh
./install.sh
```

The installer builds a release binary, installs `cybernotes` and the `cbn` alias under `~/.local/bin`, and runs first-time setup if no vault exists.

Install system-wide for all users:

```sh
sudo ./install.sh --system
```

This installs commands under `/usr/local/bin`. Each user has their own vault in their own home data directory. A system install does not move or combine users' notes.

Windows PowerShell source installation:

```powershell
.\install.ps1
```

The PowerShell installer builds from source when `target\release\cybernotes.exe` is not already present, then installs into the current user's local application directory. Prebuilt Windows binaries can also be installed manually as described above.

## First-time setup and password protection

On first install, CyberNotes asks whether to enable a master password. You can also run:

```sh
cybernotes setup
```

Choose **Yes** to encrypt the vault with a master password, or **No** to store it locally without encryption. Password protection is optional.

When protection is enabled, opening the TUI and commands that read or modify existing notes ask for the master password. `cybernotes add` is the exception: it does not ask for the master password. New notes are stored in a separate local pending inbox and imported into the main encrypted vault the next time it is unlocked.

Password commands:

```sh
cybernotes password set       # Enable encryption on an unprotected vault
cybernotes password change    # Verify old password and change it
cybernotes password reset     # Alias for password change (not forgotten-password recovery)
cybernotes password disable   # Verify password and turn off encryption
```

A forgotten password cannot be reset to decrypt an existing encrypted vault. Without the password or a separately prepared recovery key, the encrypted data cannot be recovered. `password reset` is an authenticated password change, not account-style password recovery. Keep a secure backup of your vault and remember your password.

## Storage and security

CyberNotes stores its data locally:

- Linux/macOS: `$XDG_DATA_HOME/cybernotes/`, falling back to `~/.local/share/cybernotes/`.
- Windows: the user roaming application-data directory (or `$XDG_DATA_HOME` when set). macOS: the user Application Support directory (or `$XDG_DATA_HOME` when set).

The main vault is `vault.cbn`. Notes remain there across reboots and application upgrades. Normal uninstall removes only the executable and installer metadata; it preserves the vault and pending notes. A note is removed from the vault only when the user explicitly deletes it. Uninstall never deletes notes or vault data.

When password protection is enabled:

- Argon2id derives a 256-bit encryption key from the master password (19 MiB memory, 2 iterations, 1 lane).
- XChaCha20-Poly1305 provides authenticated encryption.
- Every vault save uses a fresh random salt and 24-byte nonce.
- The master password is not stored on disk.
- Password and derived-key buffers are zeroized where supported.

When password protection is disabled, the vault is stored as local plaintext JSON. This mode does not provide encryption at rest; Unix file and directory permissions still restrict access to the current account.

On Unix, the data directory is set to mode `0700` and vault, inbox, key, staging, and temporary files are created with mode `0600`. Vault writes use a private temporary file, file synchronization, and atomic rename to reduce the risk of partial writes.

The password-free pending inbox is encrypted with a random local device key. The key is stored next to the inbox with restrictive file permissions. This design protects against accidental plaintext exposure, but it is not a security boundary against an attacker who already controls the same user account.

## Clipboard support

CyberNotes embeds/adapts the Rust clipboard backend from the CyberClip source rather than invoking a separate CyberClip program. CyberClip does not need to be installed.

| Platform | Backend | Status |
|---|---|---|
| Linux Wayland / Hyprland | `wl-clipboard-rs` with CyberNotes' own detached selection-serving helper | Implemented; test in a live Wayland session |
| Linux X11 | `clipboard-rs` native X11 backend | Implemented; test in a live X11 session |
| Windows x86-64 | `clipboard-rs` native Windows backend | Included in source and release build; runtime testing is welcome |
| macOS Intel / Apple Silicon | `clipboard-rs` native pasteboard backend | Included in source and release build; runtime testing is welcome |
| FreeBSD / OpenBSD / NetBSD | No backend | Not supported |

A graphical session is required for system clipboard access. On Wayland, the CyberNotes helper remains alive while it owns the clipboard selection; it is not CyberClip and does not require the CyberClip executable.

## CLI commands

Both `cybernotes` and `cbn` accept the same commands.

```sh
cybernotes                         # Open the TUI
cybernotes tui                     # Open the TUI explicitly
cybernotes setup                   # First-time setup

cybernotes add                     # Create a note; asks whether to open the text editor
cybernotes add -t "Shopping"       # Create with a title, then ask whether to add text
cybernotes add -t "Idea" -b "Text" # Create with title and body directly

cybernotes list                    # List note IDs and titles
cybernotes showid                  # List full IDs, titles, and keywords
cybernotes showid rust             # Filter ID/keyword output
cybernotes show <id>               # Display a note (full ID or unique prefix)
cybernotes search <query>          # Search note titles and bodies
cybernotes edit <id>               # Edit title and body
cybernotes delete <id>             # Delete a note after confirmation
cybernotes pin <id>                # Pin a note
cybernotes unpin <id>              # Unpin a note
cybernotes copy <id>               # Copy the note body to the clipboard
cybernotes paste                   # Create a note from clipboard text

cybernotes password set
cybernotes password change
cybernotes password reset
cybernotes password disable
```

`add` does not require unlocking the encrypted vault. If the vault is password-protected, the note is queued in the local pending inbox and imported on the next successful unlock. Commands that display, search, copy, or modify existing notes require unlocking when password protection is enabled.

`showid` prints each full note ID, its title, and up to eight unique keywords taken from the title and body. An optional query filters the output. Note IDs can be abbreviated in commands when the prefix uniquely identifies one note.

## TUI keyboard controls

| Key | Action |
|---|---|
| `↑` / `↓` or `k` / `j` | Move through notes |
| `/` | Search notes |
| `Esc` | Clear search |
| `a` | Add a note; choose whether to enter text |
| `e` | Edit selected note |
| `d` | Delete selected note |
| `p` | Pin / unpin selected note |
| `c` | Copy selected note body |
| `v` | Create a note from clipboard text |
| `q` | Quit TUI |

The built-in editor supports normal text entry, Enter for a new line, Backspace/Delete, arrow keys, Home, and End. **Press Esc, then C to save. Press Esc, then Q to cancel.** This same editor is used by CLI note creation and editing.

## Uninstall

Remove the current-user installation while preserving all notes:

```sh
./uninstall.sh
```

Remove a system-wide installation:

```sh
sudo ./uninstall.sh --system
```

The installer also provides wrapper scripts `install-system.sh` and `uninstall-system.sh` for system-wide use. Uninstall does not remove the vault, pending inbox, or note data.

Uninstall never deletes notes or vault data. Remove individual notes explicitly with `cybernotes delete <id>` before uninstalling if you no longer need them.

Windows current-user commands can be removed with:

```powershell
.\uninstall.ps1
```

This preserves the user's note data.

## Build and platform verification

GitHub Actions runs Cargo checks, tests, and release builds on Linux x86-64, Linux ARM64, Windows x86-64, macOS Intel, and macOS Apple Silicon. Platform archives and SHA-256 checksum files are produced for supported release builds.

Clipboard runtime behavior still needs to be tested on each actual desktop session (Wayland/Hyprland, X11, Windows, and macOS); a successful cross-platform compile alone does not prove native clipboard behavior on every host.

## License and third-party code

The MIT license in this repository applies to original CyberNotes code. The clipboard module includes code adapted from the CyberClip repository (<https://github.com/spoilpetal/CyberClip>). The supplied CyberClip source archive did not include a conventional standalone license grant and its README contained a custom restriction. See [`NOTICE`](NOTICE) for the source and licensing caveat. Do not assume the CyberClip-derived portions are relicensed under MIT; obtain the author's permission or a clear license grant before redistributing those adapted portions.
