# Lexmount WebFetch CLI (Rust)

Native Rust SDK and command-line client for Lexmount WebFetch.

## Build

```bash
cargo build --release
./target/release/webfetch-cli version
```

Credentials come from `LEXMOUNT_API_KEY`, `LEXMOUNT_PROJECT_ID`, optional
`LEXMOUNT_WEBFETCH_BASE_URL`, or the Skill-local CLI's `auth login --open` flow.
PKCE login stores credentials at
`~/.config/lexmount/webfetch-cli/credentials.json` with mode `0600` on Unix and
never prints the API key. Pass `--client-name "<name>"` to identify the calling
Agent on the approval page, or omit it to use `Agent`.

## Use

```bash
webfetch-cli extract --url https://example.com
webfetch-cli dump-dom --url https://example.com
```

Markdown is the default agent-readable output. `--format text` returns plain
text, `--format json` returns a compact response with quality warnings, and
`--format json-full` preserves the API response for debugging.

## Agent Skill package

The publishable Skill is in `skills/lexmount-webfetch`. Build a deterministic
release ZIP with:

```bash
./scripts/package-skill.sh
```

The Agent host installs the complete ZIP at its selected Skill root; the ZIP
root is the Skill root. Skill installation and status are host responsibilities,
so the Rust CLI does not provide `skill install` or `skill status`. Agents
resolve bundled scripts and binaries from the directory containing the loaded
`SKILL.md`: Codex uses the absolute source path supplied in Skill metadata,
Claude Code uses `${CLAUDE_SKILL_DIR}`, and WorkBuddy/CodeBuddy uses
`${CODEBUDDY_SKILL_DIR}`. Once started, the bootstrap and doctor scripts locate
the Skill directory from their own path.

The ZIP contains exactly eight files: `SKILL.md`, three references, and the
bootstrap/doctor scripts for all supported platforms. Native executables are published
separately. On first use, the matching script downloads the pinned release from
Tencent Cloud COS and verifies its SHA-256 digest. Tagged releases publish the
Skill ZIP, `SHA256SUMS`, and exactly three standalone binaries: signed and
notarized macOS ARM64, static Linux x64, and Windows x64. macOS Intel is not a
release platform.

The macOS signing job reads its certificate and notarization credentials from
the `macos-release` GitHub environment. The publish job uploads all three platform
binaries to Tencent Cloud COS through the `cos-release` environment, using
`TENCENT_CLOUD_SECRET_ID` and `TENCENT_CLOUD_SECRET_KEY` secrets plus
`COS_BUCKET`, `COS_REGION`, `COS_PUBLIC_BASE_URL`, and `COS_OBJECT_PREFIX`
variables.

## WorkBuddy CLI Connector

The publishable WorkBuddy CLI + Skill Connector is in `cli-connector`. It uses
the same signed/checksummed v0.1.5 release assets, installs the native executable
under the user's `.lexmount/bin` directory, and lets WorkBuddy manage login,
status checks, and disconnection through `cli.json`.

Build its deterministic submission ZIP with:

```bash
./scripts/package-cli-connector.sh
```

The generated `dist/lexmount-webfetch-cli-connector.zip` has the Connector
metadata, CLI lifecycle config, icon, installer scripts, and WorkBuddy-specific
Skill at the archive root.
