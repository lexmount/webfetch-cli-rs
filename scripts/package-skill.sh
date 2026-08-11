#!/bin/sh
set -eu
repo_dir="$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)"
skill_dir="$repo_dir/skills/lexmount-webfetch"
dist_dir="$repo_dir/dist"
mkdir -p "$dist_dir"
rm -f "$dist_dir/lexmount-webfetch.zip"
python3 - "$skill_dir" "$dist_dir/lexmount-webfetch.zip" <<'PY'
import pathlib, sys, zipfile
root = pathlib.Path(sys.argv[1])
output = pathlib.Path(sys.argv[2])
with zipfile.ZipFile(output, "w", compression=zipfile.ZIP_DEFLATED, compresslevel=9) as archive:
    for path in sorted(p for p in root.rglob("*") if p.is_file()):
        info = zipfile.ZipInfo(path.relative_to(root).as_posix(), (1980, 1, 1, 0, 0, 0))
        info.compress_type = zipfile.ZIP_DEFLATED
        info.external_attr = (0o755 if path.suffix in {".sh", ".ps1"} else 0o644) << 16
        archive.writestr(info, path.read_bytes())
PY
echo "$dist_dir/lexmount-webfetch.zip"
