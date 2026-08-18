# Authentication

Resolve `<skill-root>` from the loaded `SKILL.md`, then run the Skill-local CLI:

```text
"<skill-root>/bin/webfetch-cli" auth login --open [--client-name "NAME"]
```

On Windows PowerShell, invoke `& "<skill-root>\bin\webfetch-cli.exe"` with the
same arguments. Quote and pass the current Agent's user-facing name when
available. Omit `--client-name` when no name is available; the CLI then uses
`Agent`.

The CLI opens a PKCE approval flow, listens on a random loopback port, exchanges
the returned code, and stores the credential locally. On Unix the file mode is
`0600`.

Environment variables take precedence:

- `LEXMOUNT_PROJECT_ID`
- `LEXMOUNT_API_KEY`
- `LEXMOUNT_WEBFETCH_BASE_URL`
- `LEXMOUNT_WEBFETCH_CONNECT_BASE_URL`
- `LEXMOUNT_WEBFETCH_CREDENTIALS_FILE`

Use the same Skill-local CLI with `auth status` to inspect non-secret state. Use
`auth clear-credentials` only when the user asks to disconnect or when a stored
credential must be replaced.
