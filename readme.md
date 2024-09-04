# ssht

> ssh tmux

Runs tmux in an ssh session and exposes a unix socket to move the tmux pane on the remote. Inspired by [awesomewm-vim-tmux-navigator](https://github.com/intrntbrn/awesomewm-vim-tmux-navigator).

## Features
- Communicate with a remote tmux instance through a local UNIX socket. The UNIX socket takes the commands `has_pane <dir>` and `move_pane <dir>`, where `<dir>` is one of `left|right|up|down`

