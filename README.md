# Clipboard Sync

Synchronizes the clipboard across multiple X11 and wayland instances running on the same machine.

Example use cases:

- **Multiple displays**: You run x11 in one tty for games but otherwise use gnome in wayland, switching between desktops using ctrl-alt-F*, and you want both displays to have a unified clipboard.
- **Nested wayland**: You run a wayland compositor within a window. If you want to copy and paste anything from your primary desktop environment into the nested wayland window (or vice versa), you'll need clipboard-sync to synchronize the clipboards. examples:
  - you primarily use kde, but run sway in a window to consoldiate all your messenger apps into a single tiled/tabbed window.
  - you use gnome and develop extensions for gnome, so you run a nested gnome environment for testing.

## Installation
If you want it installed system-wide or you want the systemd service to run as a daemon, use "Generic Linux." If you want to install it only for your user account and would like to manually run the command to start the sync, install with cargo.

### Arch Linux
coming soon...

### Generic Linux
Install rust: https://www.rust-lang.org/tools/install
```bash
git clone https://github.com/dnut/clipboard-sync.git
cd clipboard-sync
make
sudo make install
```
Uninstall with:
```bash
sudo make uninstall
```

### Cargo
Install rust: https://www.rust-lang.org/tools/install

This will only install the executable, not the service.
```bash
cargo install --git https://github.com/dnut/clipboard-sync
```
Uninstall with:
```bash
cargo uninstall clipboard-sync
```

## Usage
The typical set-and-forget approach is to enable to systemd service:
```bash
systemctl --user enable --now clipboard-sync
```

If you don't want it to run constantly, only on-demand, don't use systemd. Directly call the binary as needed:
```bash
clipboard-sync
```
