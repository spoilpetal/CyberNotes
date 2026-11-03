# Changelog

## 0.2.0

- Added a shared built-in multi-line editor for CLI and TUI note creation/editing.
- Added the beginner-friendly `Add note text now? [y/N]` prompt.
- Added the `showid` command (also `ids` and `show-id`) to list full note IDs, titles, and keywords.
- Preserved the existing two-panel CyberNotes TUI design while routing edits through the built-in editor.
- Added current-user and system-wide Unix installation modes, plus uninstall scripts that preserve note data by default.
- Added Windows current-user PowerShell install and uninstall scripts.
- Added GitHub Actions CI and platform release packaging for Linux x86-64/ARM64, Windows x86-64, and macOS Intel/Apple Silicon.
- Added atomic replacement support for existing vault files on Windows.
- Updated storage path selection to use platform-appropriate local application-data directories while honoring `XDG_DATA_HOME`.
