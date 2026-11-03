#!/usr/bin/env sh
set -eu

ROOT=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
MODE=user
case "${1:-}" in
  "") ;;
  --system) MODE=system ;;
  -h|--help)
    echo "Usage: ./install.sh [--system]"
    echo "  Default: install for the current user in ~/.local/bin."
    echo "  --system: install for all users in /usr/local/bin (requires sudo)."
    exit 0 ;;
  *) echo "Unknown option: $1" >&2; exit 2 ;;
esac

OS=$(uname -s 2>/dev/null || echo unknown)
case "$OS" in
  Linux) if [ -n "${WAYLAND_DISPLAY:-}" ]; then echo "Platform: Linux / Wayland"; elif [ -n "${DISPLAY:-}" ]; then echo "Platform: Linux / X11"; else echo "Platform: Linux / headless (clipboard unavailable)"; fi ;;
  Darwin) echo "Platform: macOS / native pasteboard" ;;
  *) echo "Unsupported platform for this installer: $OS" >&2; exit 1 ;;
esac

if [ "$MODE" = system ]; then
  [ "$(id -u)" -eq 0 ] || { echo "System install requires root. Run: sudo ./install.sh --system" >&2; exit 1; }
  INSTALL_ROOT=${CYBERNOTES_INSTALL_ROOT:-/usr/local}
else
  INSTALL_ROOT=${CYBERNOTES_INSTALL_ROOT:-$HOME/.local}
fi
BIN_DIR="$INSTALL_ROOT/bin"
STATE_DIR="$INSTALL_ROOT/share/cybernotes"
STATE_FILE="$STATE_DIR/installed-commands"

user_home() {
  user=$1
  if command -v getent >/dev/null 2>&1; then
    getent passwd "$user" 2>/dev/null | cut -d: -f6
  elif command -v dscl >/dev/null 2>&1; then
    dscl . -read "/Users/$user" NFSHomeDirectory 2>/dev/null | awk '{print $2}'
  fi
}

CARGO_BIN=$(command -v cargo 2>/dev/null || true)
if [ -z "$CARGO_BIN" ] && [ "$MODE" = system ] && [ -n "${SUDO_USER:-}" ]; then
  SUDO_HOME=$(user_home "$SUDO_USER" || true)
  if [ -n "$SUDO_HOME" ] && [ -x "$SUDO_HOME/.cargo/bin/cargo" ]; then
    CARGO_BIN="$SUDO_HOME/.cargo/bin/cargo"
    export CARGO_HOME="$SUDO_HOME/.cargo" RUSTUP_HOME="$SUDO_HOME/.rustup"
    PATH="$SUDO_HOME/.cargo/bin:$PATH"; export PATH
  fi
fi
[ -n "$CARGO_BIN" ] || { echo "Cargo is required. Install Rust or use a prebuilt GitHub release binary." >&2; exit 1; }

cd "$ROOT"
echo "Building CyberNotes..."
"$CARGO_BIN" build --release
BUILT="$ROOT/target/release/cybernotes"
[ -x "$BUILT" ] || { echo "Build did not produce $BUILT" >&2; exit 1; }
mkdir -p "$BIN_DIR" "$STATE_DIR"

OLD_CYBERNOTES=0; OLD_CBN=0; OLD_BACKUP_CYBERNOTES=0; OLD_BACKUP_CBN=0
if [ -f "$STATE_FILE" ]; then
  while IFS= read -r rec; do
    case "$rec" in
      cybernotes) OLD_CYBERNOTES=1 ;;
      cbn) OLD_CBN=1 ;;
      backup:cybernotes) OLD_BACKUP_CYBERNOTES=1 ;;
      backup:cbn) OLD_BACKUP_CBN=1 ;;
    esac
  done < "$STATE_FILE"
fi

for name in cybernotes cbn; do
  old=0
  [ "$name" = cybernotes ] && old=$OLD_CYBERNOTES
  [ "$name" = cbn ] && old=$OLD_CBN
  if { [ -e "$BIN_DIR/$name" ] || [ -L "$BIN_DIR/$name" ]; } && [ "$old" -eq 0 ]; then
    printf "'%s' already exists at %s. Replace it? A backup will be kept. [y/N]: " "$name" "$BIN_DIR/$name"
    IFS= read -r answer
    case "$answer" in y|Y|yes|YES) ;; *) echo "Installation cancelled without changing commands."; exit 0 ;; esac
    backup="$STATE_DIR/$name.previous"
    [ ! -e "$backup" ] && [ ! -L "$backup" ] || { echo "Backup already exists: $backup" >&2; exit 1; }
    mv "$BIN_DIR/$name" "$backup"
  fi
done

TMP="$BIN_DIR/.cybernotes.new.$$"
cp "$BUILT" "$TMP"
chmod 755 "$TMP"
mv -f "$TMP" "$BIN_DIR/cybernotes"
ln -sfn cybernotes "$BIN_DIR/cbn"
{
  echo cybernotes
  echo cbn
  [ "$OLD_BACKUP_CYBERNOTES" -eq 1 ] && echo backup:cybernotes || { [ -e "$STATE_DIR/cybernotes.previous" ] && echo backup:cybernotes || true; }
  [ "$OLD_BACKUP_CBN" -eq 1 ] && echo backup:cbn || { [ -e "$STATE_DIR/cbn.previous" ] && echo backup:cbn || true; }
} > "$STATE_FILE.new"
mv -f "$STATE_FILE.new" "$STATE_FILE"

if [ "$MODE" = user ]; then
  case ":${PATH}:" in *":$BIN_DIR:"*) ;;
    *) SHELL_NAME=$(basename "${SHELL:-sh}")
       case "$SHELL_NAME" in
         fish) RC="${XDG_CONFIG_HOME:-$HOME/.config}/fish/config.fish"; LINE="fish_add_path \"$BIN_DIR\"" ;;
         zsh) RC="$HOME/.zshrc"; LINE="export PATH=\"$BIN_DIR:\$PATH\"" ;;
         bash) RC="$HOME/.bashrc"; LINE="export PATH=\"$BIN_DIR:\$PATH\"" ;;
         *) RC="$HOME/.profile"; LINE="export PATH=\"$BIN_DIR:\$PATH\"" ;;
       esac
       mkdir -p "$(dirname "$RC")"; touch "$RC"
       if ! grep -Fqx "$LINE" "$RC"; then printf '\n# CyberNotes command-line tools\n%s\n' "$LINE" >> "$RC"; fi
       echo "Added $BIN_DIR to PATH in $RC. Reload your shell." ;;
  esac
fi

# First-run setup is performed as the invoking desktop user for system installs.
if [ "$MODE" = user ]; then
  DATA_HOME=${XDG_DATA_HOME:-$HOME/.local/share}
  if [ ! -f "$DATA_HOME/cybernotes/vault.cbn" ]; then "$BIN_DIR/cybernotes" setup; else echo "Existing notes detected; data and password settings were preserved."; fi
elif [ -n "${SUDO_USER:-}" ]; then
  SUDO_HOME=$(user_home "$SUDO_USER" || true)
  if [ -z "$SUDO_HOME" ]; then echo "Could not locate the invoking user's home directory; run cybernotes setup as your normal user."; exit 0; fi
  DATA_HOME="$SUDO_HOME/.local/share"
  if [ ! -f "$DATA_HOME/cybernotes/vault.cbn" ]; then
    if command -v runuser >/dev/null 2>&1; then runuser -u "$SUDO_USER" -- env HOME="$SUDO_HOME" PATH="$BIN_DIR:$PATH" "$BIN_DIR/cybernotes" setup
    else echo "Run '$BIN_DIR/cybernotes setup' as your normal user to initialize your vault."; fi
  else echo "Existing user notes detected; data and password settings were preserved."; fi
else
  echo "System binaries installed. Run 'cybernotes setup' as each user to initialize their own vault."
fi

echo "Installed cybernotes and cbn in $BIN_DIR"
echo "Note data is stored separately and is not removed by uninstall.sh."
