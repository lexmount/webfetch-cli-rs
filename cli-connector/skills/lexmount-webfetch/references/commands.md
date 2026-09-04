# Command reference

Use `"$HOME/.lexmount/bin/webfetch-cli"` on macOS and Linux. In Windows
PowerShell use `& "$env:USERPROFILE\.lexmount\bin\webfetch-cli.exe"`. The
placeholder `<webfetch-cli>` below means that platform-specific invocation.

```text
<webfetch-cli> version
<webfetch-cli> capabilities --json
<webfetch-cli> auth status

<webfetch-cli> extract (--url URL | --dom-id ID) [--timeout-ms MS]
  [--format md|text|json|json-full]
  [--include-trace] [--include-raw-dom]

<webfetch-cli> dump-dom --url URL [--timeout-ms MS]
  [--format md|text|json|json-full]
  [--engine auto|http|chrome|chrome_cdp|lightmount_lite|lightmount_dcl|lightmount_domstable]
  [--filter-scripts-styles]
```

`extract --dom-id` reuses a prior DOM dump when the API returned a DOM ID.
Default output is Markdown. Debug flags require `--format json-full`, which
prevents heavy or sensitive diagnostic fields from appearing accidentally.

Use a positive integer for `--timeout-ms`; values below 1000 are raised to 1000
milliseconds. Quote URLs so shell metacharacters in query strings are not
interpreted.
