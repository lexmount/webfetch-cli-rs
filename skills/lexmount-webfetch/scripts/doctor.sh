#!/bin/sh
set -eu
skill_dir="$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)"
if [ -x "$skill_dir/bin/webfetch-cli" ]; then exec "$skill_dir/bin/webfetch-cli" doctor --json; fi
command -v webfetch-cli >/dev/null 2>&1 || { echo '{"ok":false,"error":"command_not_found","message":"Run bootstrap.sh first."}'; exit 1; }
exec webfetch-cli doctor --json
