# Command reference

```text
webfetch-cli version
webfetch-cli doctor --json
webfetch-cli capabilities --json

webfetch-cli auth status
webfetch-cli auth login --open [--client-name WorkBuddy]
  [--connect-base-url https://browser.lexmount.cn] [--timeout-seconds 300]
webfetch-cli auth clear-credentials

webfetch-cli extract (--url URL | --dom-id ID) [--timeout-ms MS]
  [--format md|text|json|json-full]
  [--include-trace] [--include-raw-dom]

webfetch-cli dump-dom --url URL [--timeout-ms MS]
  [--format md|text|json|json-full]
  [--engine auto|http|chrome|chrome_cdp|lightmount_lite|lightmount_dcl|lightmount_domstable]
  [--filter-scripts-styles]
```

`extract --dom-id` reuses a prior DOM dump when the API returned a DOM ID. Default output is Markdown. Debug flags require `--format json-full` so heavy or sensitive diagnostic fields do not appear accidentally.
