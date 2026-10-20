#!/bin/sh
# Installs Bytepet for the current user (no root needed).
#   ./install.sh            install
#   ./install.sh --remove   uninstall (your save is kept)
set -e
here=$(cd "$(dirname "$0")" && pwd)
bin="$HOME/.local/bin"
apps="${XDG_DATA_HOME:-$HOME/.local/share}/applications"
icons="${XDG_DATA_HOME:-$HOME/.local/share}/icons/hicolor/512x512/apps"

if [ "$1" = "--remove" ]; then
    rm -f "$bin/bytepet" "$apps/bytepet.desktop" "$icons/bytepet.png"
    echo "Bytepet removed. Your save is still in ${XDG_DATA_HOME:-$HOME/.local/share}/bytepet"
    exit 0
fi

mkdir -p "$bin" "$apps" "$icons"
install -m 755 "$here/bytepet" "$bin/bytepet"
install -m 644 "$here/icon.png" "$icons/bytepet.png"
sed "s|^Exec=.*|Exec=$bin/bytepet|" "$here/bytepet.desktop" > "$apps/bytepet.desktop"
command -v update-desktop-database >/dev/null 2>&1 && update-desktop-database "$apps" || true
echo "Installed to $bin/bytepet - find Bytepet in your applications menu."
case ":$PATH:" in *":$bin:"*) ;; *) echo "Note: $bin is not on your PATH.";; esac
