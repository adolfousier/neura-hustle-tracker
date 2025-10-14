#!/bin/bash

# Install the GNOME extension for Wayland PID retrieval

EXT_DIR="$HOME/.local/share/gnome-shell/extensions/timetracker-pid@timetracking.rs"

echo "Installing GNOME extension to $EXT_DIR..."

mkdir -p "$EXT_DIR"

cp gnome-extension/extension.js "$EXT_DIR/"
cp gnome-extension/metadata.json "$EXT_DIR/"

echo "Extension installed. You may need to:"
echo "1. Restart GNOME Shell (Alt+F2, r, Enter)"
echo "2. Enable the extension: gnome-extensions enable timetracker-pid@timetracking.rs"
echo "3. Or use the Extensions app to enable it"

echo "Done."