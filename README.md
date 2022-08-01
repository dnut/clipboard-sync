# Clipboard Sync

Synchronizes the clipboard across multiple X11 and wayland instances running on the same machine.

Example use cases:

Nested wayland: You use gnome as your primary desktop environment, but you use another wayland compositor, like sway, within a window to consoldiate all your messenger apps into a single tabbed window. If you want to copy and paste anything from your messenger apps in sway to the other apps in gnome (or vice versa), you'll need clipboard-sync to synchronize the sway and gnome clipboards.

Multiple displays: You run x11 in one tty for games but otherwise use gnome in wayland.

## Installation

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

If you don't want it to run constantly, only on-demand, forget about systemd, and feel free to directly call the binary:
```bash
clipboard-sync
```
