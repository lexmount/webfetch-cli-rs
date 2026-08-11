# Troubleshooting

1. Missing command: run the platform bootstrap script, then the doctor script.
2. Missing or expired credentials: run `webfetch-cli auth login --open --client-name WorkBuddy`.
3. Thin content or HTML warning: retry with `dump-dom`, try an explicit engine, or move to the browser Skill when interaction/rendering is required.
4. API timeout: increase `--timeout-ms` once; do not retry indefinitely.
5. Need trace or raw DOM: add `--format json-full` before the debug flag.
6. Unexpected API shape: use `--format json-full` for diagnosis, but redact secrets before sharing output.
