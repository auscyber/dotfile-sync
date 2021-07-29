#!/bin/sh
set -e
curl -LsSf "https://github.com/AusCyberman/dotfile-sync/releases/download/0.2-beta/dots" > /tmp/dots
chmod +x /tmp/dots
/tmp/dots $@ sync
