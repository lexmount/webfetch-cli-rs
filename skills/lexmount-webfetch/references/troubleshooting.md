# Troubleshooting

1. Skill root unknown: resolve the directory containing the loaded `SKILL.md`
   with the current host's locator: Codex supplies its absolute source path in
   Skill metadata, Claude Code provides `${CLAUDE_SKILL_DIR}`, and
   WorkBuddy/CodeBuddy provides `${CODEBUDDY_SKILL_DIR}`. Do not infer it from
   the working directory or search the user's home directory.
2. Missing command: run `sh "<skill-root>/scripts/bootstrap.sh"` on macOS arm64
   or `& "<skill-root>\scripts\bootstrap.ps1"` on Windows x64, then run the
   matching doctor script. Invoke only the Skill-local binary afterward; do not
   rely on `PATH`.
3. Missing or expired credentials: run the Skill-local CLI's
   `auth login --open`. Pass `--client-name "<agent-name>"` when the current
   Agent has a user-facing name; otherwise omit it to use `Agent`.
4. Thin content or HTML warning: retry with `dump-dom`, try an explicit engine, or move to the browser Skill when interaction/rendering is required.
5. API timeout: increase `--timeout-ms` once; do not retry indefinitely.
6. Need trace or raw DOM: add `--format json-full` before the debug flag.
7. Unexpected API shape: use `--format json-full` for diagnosis, but redact secrets before sharing output.
