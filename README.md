# Lexmount WebFetch CLI (Rust)

Native Rust SDK and command-line client for Lexmount WebFetch.

## Build

```bash
cargo build --release
./target/release/webfetch-cli version
```

Credentials come from `LEXMOUNT_API_KEY`, `LEXMOUNT_PROJECT_ID`, optional
`LEXMOUNT_WEBFETCH_BASE_URL`, or `webfetch-cli auth login`. PKCE login stores
credentials at `~/.config/lexmount/webfetch-cli/credentials.json` with mode
`0600` on Unix and never prints the API key.

## Use

```bash
webfetch-cli extract --url https://example.com
webfetch-cli dump-dom --url https://example.com
```

Markdown is the default agent-readable output. `--format text` returns plain
text, `--format json` returns a compact response with quality warnings, and
`--format json-full` preserves the API response for debugging.

## WorkBuddy package

The publishable Skill is in `skills/lexmount-webfetch`. Build a deterministic,
direct-upload SkillHub ZIP with:

```bash
./scripts/package-skill.sh
```

The Skill ZIP contains `SKILL.md`, references, and platform bootstrap scripts;
native executables are published separately. On first use, the matching script
downloads the pinned release from Tencent Cloud COS and verifies its SHA-256
digest. Tagged releases publish the Skill ZIP, `SHA256SUMS`, and exactly two
standalone binaries: signed and notarized macOS ARM64 plus Windows x64. Linux
and macOS Intel are not release platforms.

The macOS signing job reads its certificate and notarization credentials from
the `macos-release` GitHub environment. The publish job uploads both platform
binaries to Tencent Cloud COS through the `cos-release` environment, using
`TENCENT_CLOUD_SECRET_ID` and `TENCENT_CLOUD_SECRET_KEY` secrets plus
`COS_BUCKET`, `COS_REGION`, `COS_PUBLIC_BASE_URL`, and `COS_OBJECT_PREFIX`
variables.
