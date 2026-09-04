# Troubleshooting

1. Missing executable: ask the user to reinstall the Lexmount WebFetch
   Connector in WorkBuddy. Do not download a second Skill-local binary.
2. Missing or expired credentials: run `auth status` once, then ask the user to
   reconnect the Connector in WorkBuddy.
3. Thin content or an HTML warning: retry with `dump-dom`, try an explicit
   engine, or switch to a browser skill when interaction or full browser state
   is required.
4. API timeout: increase `--timeout-ms` once; do not retry indefinitely.
5. Need trace or raw DOM: select `--format json-full` before adding
   `--include-trace` or `--include-raw-dom`.
6. Unexpected API shape: use `--format json-full` for diagnosis, but redact
   private page content before sharing output.
7. Unsupported machine: this Connector release supports macOS ARM64, Linux
   x64, and Windows x64. Explain the platform limitation instead of trying a
   binary built for another operating system or architecture.
