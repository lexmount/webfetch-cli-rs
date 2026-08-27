#!/bin/sh
set -eu

version="${LEXMOUNT_WEBFETCH_CLI_VERSION:-0.1.3}"
download_base_url="${LEXMOUNT_WEBFETCH_CLI_DOWNLOAD_BASE_URL:-https://cli-bin-1377899528.cos.ap-nanjing.myqcloud.com/releases/webfetch-cli}"
repo="${download_base_url%/}/v${version}"
case "$(uname -s)-$(uname -m)" in
  Darwin-arm64) target="aarch64-apple-darwin" ;;
  *) echo "Unsupported platform: $(uname -s) $(uname -m). This release supports macOS ARM64 and Windows x64." >&2; exit 2 ;;
esac
asset="webfetch-cli-v${version}-${target}"
tmp_dir="$(mktemp -d)"
trap 'rm -rf "$tmp_dir"' EXIT INT TERM
curl --proto '=https' --tlsv1.2 -fsSL "$repo/$asset" -o "$tmp_dir/$asset"
curl --proto '=https' --tlsv1.2 -fsSL "$repo/SHA256SUMS" -o "$tmp_dir/SHA256SUMS"
expected="$(awk -v name="$asset" '$2 == name {print $1}' "$tmp_dir/SHA256SUMS")"
[ -n "$expected" ] || { echo "No checksum published for $asset" >&2; exit 3; }
actual="$(openssl dgst -sha256 "$tmp_dir/$asset" | awk '{print $NF}')"
[ "$expected" = "$actual" ] || { echo "SHA-256 mismatch for $asset" >&2; exit 4; }
skill_dir="$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)"
install_dir="${LEXMOUNT_WEBFETCH_CLI_INSTALL_DIR:-$skill_dir/bin}"
mkdir -p "$install_dir"
install -m 0755 "$tmp_dir/$asset" "$install_dir/webfetch-cli"
"$install_dir/webfetch-cli" version
echo "Installed webfetch-cli to $install_dir/webfetch-cli"
