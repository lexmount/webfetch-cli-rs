# Authentication

WorkBuddy owns the Connector login workflow. The CLI opens Lexmount's PKCE
approval page and persists credentials outside the executable directory at
`~/.config/lexmount/webfetch-cli/credentials.json` on macOS and Linux, or the
equivalent user configuration directory on Windows.

When an extraction reports missing or expired credentials:

1. Run the platform-specific `<webfetch-cli> auth status` once to confirm the
   state. This command is local and has no side effects.
2. Ask the user to reconnect the Lexmount WebFetch Connector in WorkBuddy.
3. Retry the original command after WorkBuddy reports that the Connector is
   connected.

Do not ask the user to paste an API key into chat. Do not read or print the
credential file. `auth clear-credentials` is reserved for WorkBuddy's
Connector disconnection action.

The CLI also recognizes these environment variables when administrators use a
managed deployment:

- `LEXMOUNT_PROJECT_ID`
- `LEXMOUNT_API_KEY`
- `LEXMOUNT_WEBFETCH_BASE_URL`
- `LEXMOUNT_WEBFETCH_CONNECT_BASE_URL`
- `LEXMOUNT_WEBFETCH_CREDENTIALS_FILE`
