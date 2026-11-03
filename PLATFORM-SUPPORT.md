# Platform support

The GitHub Actions build matrix follows the platform set used by CyberClip's release workflow, narrowed to the platforms with clipboard implementations in the supplied CyberClip source.

| Release artifact | Rust target | Clipboard implementation |
|---|---|---|
| Linux x86-64 | `x86_64-unknown-linux-gnu` | Wayland: `wl-clipboard-rs`; X11: `clipboard-rs` |
| Linux ARM64 | `aarch64-unknown-linux-gnu` | Wayland: `wl-clipboard-rs`; X11: `clipboard-rs` |
| Windows x86-64 | `x86_64-pc-windows-msvc` | `clipboard-rs` native Windows backend |
| macOS Intel | `x86_64-apple-darwin` | `clipboard-rs` native pasteboard backend |
| macOS Apple Silicon | `aarch64-apple-darwin` | `clipboard-rs` native pasteboard backend |

Linux distributions (Arch, Fedora, Debian, Ubuntu, and others) use the same Linux binaries for the same CPU architecture. Wayland and X11 are selected at runtime from the session environment. The Wayland selection provider is a detached hidden helper mode of the CyberNotes executable itself.

FreeBSD, OpenBSD, and NetBSD are not supported because the inspected CyberClip source does not provide a supported native clipboard backend for those systems.

CI compiles and runs Rust tests on the listed native runner platforms. This does not replace interactive clipboard testing in an actual Hyprland/Wayland, X11, Windows, or macOS desktop session.
