# Clipboard Sync

Synchronizes the clipboard across multiple X11 and wayland instances running on the same machine.

Example use cases:

- **Improve Wayland compatibility**: You have already enabled support for wayland in your system, but your computer does not synchronize the clipboard between X11 and wayland windows. clipboard-sync can solve this problem. [more details](https://github.com/dnut/clipboard-sync/issues/9#issuecomment-1502368133)
- **VNC**: You run a VNC server and would like all host and client logins from the same user to share the same clipboard.
- **Multiple displays**: You run two or more desktop environments or window managers in separate ttys, switching between desktops using ctrl-alt-F*. 
- **Nested Wayland**: You run a wayland compositor within a window. examples:
  - you primarily use kde, but run sway in a window to consolidate all your messenger apps into a single tiled/tabbed window.
  - you use gnome and develop extensions for gnome, so you run a nested gnome environment for testing.

# Installation
Most users should install clipboard-sync with their operating system's package manager. If your system is not supported, please vote on [the appropriate issue](https://github.com/dnut/clipboard-sync/issues?q=is%3Aopen+is%3Aissue+label%3Adistribution), or create one if it does not exist.

While waiting for support on your system, there are two other options:
- [Generic Linux](#generic-linux)
  - automatically and cleanly installs or uninstalls the entire package.
  - requires extra steps to acquire the source code.
- [Cargo](#cargo)
  - only installs the executable, which needs to be run manually.
  - requires extra steps to manually edit and install a systemd service if desired.
  - If you use [cargo-update](https://crates.io/crates/cargo-update), it can make updates to the executable easier.

## Arch Linux
[clipboard-sync](https://aur.archlinux.org/packages/clipboard-sync) is available in the Arch User Repository.

## Ubuntu & Debian
Add the repository:
```bash
sudo wget -P /etc/apt/sources.list.d/ https://raw.githubusercontent.com/dnut/deb/master/dnut.list
```

Install the package:
```bash
sudo apt update && sudo apt install clipboard-sync
```

Alternatively, you can [Build from Source](#build-from-source), then run:
```bash
make deb && sudo apt install ./dist/deb/clipboard-sync_*.deb
```

## Generic Linux
[Build from Source](#build-from-source), then install ***either*** system-wide ***or*** for only the current user:
```bash
sudo make install  # system
make user-install  # user
```
It can be easily uninstalled:
```bash
sudo make uninstall  # system
make user-uninstall  # user
```

## Cargo
[clipboard-sync](https://crates.io/crates/clipboard-sync) is published to crates.io, so it can be installed as a normal binary crate.

1. Install rust: https://www.rust-lang.org/tools/install
2. Install clipboard-sync
```bash
cargo install clipboard-sync
```
3. If you want it to run in the background, you can use tmux, or you can manually download the systemd service file and copy it to a systemd folder. You can download it to the correct path using this command, after which you may need to manually edit the file to point the correct binary location:
```bash
wget -P "$HOME/.config/systemd/" https://raw.githubusercontent.com/dnut/clipboard-sync/master/clipboard-sync.service
```
It can be easily uninstalled:
```bash
cargo uninstall clipboard-sync
rm -r "$HOME/.config/systemd/clipboard-sync.service"
```

## Build from Source

1. Ensure you have the build dependencies: rust make gcc libc libxcb
- install rust using rustup: https://www.rust-lang.org/tools/install
- For the rest:
  - arch linux: `sudo pacman -Syu make gcc libxcb`
  - debian/ubuntu: `sudo apt install make gcc libxcb1-dev libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev`

2. Download the source from [the releases page](https://github.com/dnut/clipboard-sync/releases/) and unzip it. Alternatively, select *one* of these commands to download the source:
```bash
wget -O- https://github.com/dnut/clipboard-sync/archive/refs/tags/0.2.0.tar.gz | tar xvz
curl -L https://github.com/dnut/clipboard-sync/archive/refs/tags/0.2.0.tar.gz | tar xvz
git clone https://github.com/dnut/clipboard-sync.git --branch stable
```

3. Compile the program
```bash
cd clipboard-sync*
make
```

The executable is here:
```bash
./target/release/clipboard-sync
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

You can also daemonize clipboard-sync using tmux instead of systemd. ~/.bashrc aliases may be handy for these commands.
```bash
tmux new-session -ds clipboard-sync clipboard-sync  # start in background
tmux attach -t clipboard-sync                       # view status
ctrl-b, d                                           # while viewing status, send back to background
ctrl-c                                              # while viewing status, terminate the process
```
