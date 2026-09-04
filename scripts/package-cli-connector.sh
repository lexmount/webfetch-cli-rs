#!/bin/sh
set -eu

repo_dir="$(CDPATH='' cd -- "$(dirname -- "$0")/.." && pwd)"
connector_dir="$repo_dir/cli-connector"
dist_dir="$repo_dir/dist"
staging_dir="$(mktemp -d)"
archive="$dist_dir/lexmount-webfetch-cli-connector.zip"
trap 'rm -rf "$staging_dir"' EXIT INT TERM

for required_file in connector-meta.json cli.json icon.svg skills/lexmount-webfetch/SKILL.md; do
  [ -f "$connector_dir/$required_file" ] || {
    echo "Missing Connector file: $required_file" >&2
    exit 2
  }
done

mkdir -p "$dist_dir"
rm -f "$archive"

(
  cd "$connector_dir"
  find . -type f ! -name '.DS_Store' -print | LC_ALL=C sort |
    while IFS= read -r relative_path; do
      mkdir -p "$staging_dir/$(dirname -- "$relative_path")"
      cp "$relative_path" "$staging_dir/$relative_path"
    done
)

find "$staging_dir" -type d -exec chmod 0755 {} +
find "$staging_dir" -type f -exec chmod 0644 {} +
find "$staging_dir/scripts" -type f \( -name '*.sh' -o -name '*.ps1' \) -exec chmod 0755 {} +
find "$staging_dir" -exec touch -t 198001010000 {} +

(
  cd "$staging_dir"
  find . -type f -print | LC_ALL=C sort | sed 's|^./||' |
    zip -X -9 -q "$archive" -@
)

echo "$archive"
