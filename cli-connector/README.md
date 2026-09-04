# Lexmount WebFetch WorkBuddy Connector

This directory is the publishable WorkBuddy CLI + Skill Connector v0.1.0. It
installs the native `webfetch-cli` v0.1.5 binary and exposes its authentication
and status lifecycle to WorkBuddy.

Supported release targets:

- macOS ARM64
- Linux x64 (static musl executable)
- Windows x64

The installer downloads only HTTPS release assets from Lexmount's Tencent
Cloud COS bucket and verifies the published SHA-256 digest before installation.
Executables are stored under the user's `.lexmount/bin` directory. Credentials
remain separate in the platform user configuration directory.

From the repository root, validate and package the Connector with:

```sh
./scripts/package-cli-connector.sh
```

The resulting `dist/lexmount-webfetch-cli-connector.zip` has
`connector-meta.json`, `cli.json`, and `icon.svg` at its archive root and is
ready for WorkBuddy review.
