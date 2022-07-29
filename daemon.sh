#!/bin/bash

root=/home/drew/mine/code/clipboard-sync
name=clipsync

tmux new -s "$name" "
    bash --init-file <(echo '
        source ~/.bashrc
        cd /home/drew/mine/code/clipboard-sync
        ./run.sh
    ')
" || (
    tmux send-keys -t "$name" \
        'C-c' Enter '# ...joined existing session' \
        Enter '# attempted to kill ./run.sh, now restarting...' \
        Enter "$root/run.sh" Enter
    tmux attach -t "$name"
)
