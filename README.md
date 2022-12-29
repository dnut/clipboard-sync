# Clipboard Sync

Synchronizes the clipboard across multiple X11 and wayland instances running on the same machine.

Example use cases:

- **Multiple displays**: You run x11 in one tty for games but otherwise use gnome in wayland, switching between desktops using ctrl-alt-F*, and you want both displays to have a unified clipboard.
- **Nested wayland**: You run a wayland compositor within a window. If you want to copy and paste anything from your primary desktop environment into the nested wayland window (or vice versa), you'll need clipboard-sync to synchronize the clipboards. examples:
  - you primarily use kde, but run sway in a window to consolidate all your messenger apps into a single tiled/tabbed window.
  - you use gnome and develop extensions for gnome, so you run a nested gnome environment for testing.

# Installation
If you want it installed system-wide or want the service to run automatically, use "Generic Linux." If you want to install it only for your user account and would like to manually run the command to start the sync, install with cargo.

## Arch Linux
[clipboard-sync](https://aur.archlinux.org/packages/clipboard-sync) is available in the Arch User Repository.

## Generic Linux
1. Install rust: https://www.rust-lang.org/tools/install
2. Download the code:
```bash
git clone https://github.com/dnut/clipboard-sync.git
cd clipboard-sync
git checkout 0.1.0
```
3. Install ***either*** system-wide ***or*** for only the current user:
```bash
make && sudo make install  # system
make && make user-install  # user
```
It can be easily uninstalled:
```bash
sudo make uninstall  # system
make user-uninstall  # user
```
## Cargo
clipboard-sync is published to crates.io so it can be installed as a normal binary crate.

1. Install rust: https://www.rust-lang.org/tools/install
2. Install clipboard-sync
```bash
cargo install clipboard-sync
```
3. If you want the service, you need to manually download the file and copy it to a systemd folder. For example:
```bash
wget -P "$HOME/.config/systemd/" https://raw.githubusercontent.com/dnut/clipboard-sync/master/clipboard-sync.service
```
It can be easily uninstalled:
```bash
cargo uninstall clipboard-sync
rm -r "$HOME/.config/systemd/clipboard-sync.service"
```

# Usage
The typical set-and-forget approach is to enable to service:
```bash
systemctl --user enable --now clipboard-sync
```

If you don't want it to run constantly, only on-demand, don't use systemd. Directly call the binary as needed:
```bash
clipboard-sync
```
