#!/bin/bash

set -eo pipefail

if systemctl --user is-active --quiet auto-agent-mux.service; then
    echo "Stopping current mux"
    systemctl --user stop auto-agent-mux.service
fi

echo "Downloading & installing auto-agent-mux..."
mkdir -p ~/.local/bin
curl -sL 'https://github.com/xqms/auto-agent-mux/releases/latest/download/auto-agent-mux' > ~/.local/bin/auto-agent-mux
chmod a+x ~/.local/bin/auto-agent-mux

echo "Registering with the user systemd instance...."
mkdir -p ~/.local/share/systemd/user
tee ~/.local/share/systemd/user/auto-agent-mux.service > /dev/null <<EOF
[Unit]
Description=SSH mux

[Service]
Type=simple
ExecStart=${HOME}/.local/bin/auto-agent-mux --socket-dir ${XDG_RUNTIME_DIR}/auto-agent-mux

[Install]
WantedBy=default.target
EOF

systemctl --user daemon-reload
systemctl --user enable auto-agent-mux
systemctl --user start auto-agent-mux

echo "Updating .bashrc..."
if ! grep 'auto-agent-mux' ~/.bashrc > /dev/null; then
    tee -a ~/.bashrc > /dev/null <<EOF

# auto-agent-mux
if [ ! -z $${TMUX+x} ]; then
    export SSH_AUTH_SOCK=${XDG_RUNTIME_DIR}/auto-agent-mux/agent.sock
fi
EOF
fi
