# Clipboard Sync

Synchronizes the clipboard across multiple X11 and wayland instances running on the same machine.

Example use cases:

Nested wayland: You use gnome as your primary desktop environment, but you use another wayland compositor, like sway, within a window to consoldiate all your messenger apps into a single tabbed window. If you want to copy and paste anything from your messenger apps in sway to the other apps in gnome (or vice versa), you'll need clipboard-sync to synchronize the sway and gnome clipboards.

Multiple displays: You run x11 in one tty for games but otherwise use gnome in wayland.

## Installation
This hasn't been made into an easily installable package yet. Here are some steps you could follow to get it easier to run:
```bash
git clone git@github.com:dnut/clipboard-sync.git
cd clipboard-sync
sed -i "s_root=/home/drew/mine/code/clipboard-sync_root=$(pwd)_g" daemon.sh
echo "alias clipsync='$(pwd)/daemon.sh'" >> ~/.bashrc
. ~/.bashrc
```
Then you can run `clipsync` at any time to start the service. The service runs in tmux so you can use `tmux attach` to find its output. Use `git pull` in the repo root to update.
