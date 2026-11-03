# Build status for this source package

This revision is based on the user's previously cleanly built CyberNotes password/TUI package. It adds a shared built-in editor, `showid`, user/system install and uninstall scripts, platform CI, and release packaging workflows.

## Checks performed in the packaging environment

- `sh -n install.sh uninstall.sh install-system.sh uninstall-system.sh`: passed.
- `./install.sh --help` and `./uninstall.sh --help`: passed.
- GitHub Actions YAML files: parsed successfully with a YAML parser.
- `Cargo.toml`: parsed as TOML.
- Current-user install/uninstall flow: exercised with a fake build executable; command links were installed/removed and the vault file was preserved.
- System-wide install/uninstall flow: exercised with a fake build executable; commands were installed/removed and user data was left untouched.
- ZIP archive integrity: checked after packaging.

## Rust build status

A Rust build could not be run in this packaging environment. Neither `cargo`, `rustc`, nor `rustfmt` is installed. Installing the toolchain was attempted, but DNS/network access to the Debian and Rust distribution servers was unavailable. `cargo check` therefore stops immediately with `cargo: command not found`.

**This package is source-complete but is not claimed to be compiled or build-clean in this environment.** The earlier successful build reported for the previous ZIP does not automatically verify this revision.

On Arch Linux, from the project directory, run:

```sh
cargo check
cargo test
cargo build --release
```

The first Cargo invocation resolves dependencies and generates `Cargo.lock`; commit that generated lockfile with the source before creating reproducible release builds. The GitHub Actions workflows also generate a lockfile before running their locked checks and builds.

The GitHub Actions matrix checks native builds on Linux x86-64, Linux ARM64, Windows x86-64, macOS Intel, and macOS Apple Silicon. Native clipboard runtime testing is still required on each real desktop platform, especially Wayland/Hyprland and X11.
