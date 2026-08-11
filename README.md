# Lexmount WebFetch CLI (Rust)

Native Rust SDK and command-line client for Lexmount WebFetch. It mirrors the
agent-facing Python `webfetch-cli` contract without requiring Python, `uv`, or
Git at runtime.

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

Tagged releases publish `lexmount-webfetch-v<VERSION>-skillhub.zip`,
`SHA256SUMS`, and exactly two raw binaries: macOS ARM64 and Windows x64. Linux
and macOS Intel are not release platforms.
