#!/usr/bin/env sh
set -eu
MODE=user
for arg in "$@"; do
  case "$arg" in
    --system) MODE=system ;;
    -h|--help)
      echo "Usage: ./uninstall.sh [--system]"
      echo "  --system      Remove the all-users commands (requires sudo)."
      echo "  Note data is always preserved by uninstall."
      exit 0 ;;
    *) echo "Unknown option: $arg" >&2; exit 2 ;;
  esac
done
if [ "$MODE" = system ]; then
  [ "$(id -u)" -eq 0 ] || { echo "System uninstall requires root. Run: sudo ./uninstall.sh --system" >&2; exit 1; }
  ROOT=${CYBERNOTES_INSTALL_ROOT:-/usr/local}
else ROOT=${CYBERNOTES_INSTALL_ROOT:-$HOME/.local}; fi
BIN_DIR="$ROOT/bin"; STATE_DIR="$ROOT/share/cybernotes"; STATE_FILE="$STATE_DIR/installed-commands"

if [ -f "$STATE_FILE" ]; then
  while IFS= read -r rec; do
    case "$rec" in cybernotes|cbn) rm -f "$BIN_DIR/$rec" ;; esac
  done < "$STATE_FILE"
  while IFS= read -r rec; do
    case "$rec" in
      backup:cybernotes|backup:cbn)
        name=${rec#backup:}; backup="$STATE_DIR/$name.previous"
        if [ -e "$backup" ] || [ -L "$backup" ]; then mkdir -p "$BIN_DIR"; mv "$backup" "$BIN_DIR/$name"; fi ;;
    esac
  done < "$STATE_FILE"
  rm -f "$STATE_FILE"
else
  echo "No CyberNotes install record found at $STATE_FILE."
  for name in cybernotes cbn; do
    if [ -e "$BIN_DIR/$name" ] || [ -L "$BIN_DIR/$name" ]; then
      printf "Untracked '%s' exists in %s. Remove it? [y/N]: " "$name" "$BIN_DIR"
      IFS= read -r answer
      case "$answer" in y|Y|yes|YES) rm -f "$BIN_DIR/$name" ;; esac
    fi
  done
fi
rmdir "$STATE_DIR" 2>/dev/null || true
rmdir "$ROOT/share" 2>/dev/null || true

if [ "$MODE" = user ]; then
  for rc in "$HOME/.bashrc" "$HOME/.zshrc" "$HOME/.profile" "${XDG_CONFIG_HOME:-$HOME/.config}/fish/config.fish"; do
    [ -f "$rc" ] || continue
    tmp="$rc.cybernotes-uninstall.$$"
    awk '
      $0 == "# CyberNotes command-line tools" { skip=1; next }
      skip && ($0 ~ /^export PATH=.*\.local\/bin:\$PATH"$/ || $0 ~ /^fish_add_path .*\.local\/bin"$/) { skip=0; next }
      { if (skip) skip=0; print }
    ' "$rc" > "$tmp"
    if ! cmp -s "$rc" "$tmp"; then cat "$tmp" > "$rc"; fi
    rm -f "$tmp"
  done
fi

echo "Notes and vault data were preserved."
if [ "$MODE" = system ]; then echo "System CyberNotes commands removed; user data was untouched."; else echo "CyberNotes commands removed."; fi
