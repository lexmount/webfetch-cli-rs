$ErrorActionPreference = "Stop"
$skillBinary = Join-Path (Split-Path -Parent $PSScriptRoot) "bin\webfetch-cli.exe"
if (Test-Path $skillBinary) { & $skillBinary doctor --json; exit $LASTEXITCODE }
$command = Get-Command webfetch-cli -ErrorAction SilentlyContinue
if (-not $command) { Write-Output '{"ok":false,"error":"command_not_found","message":"Run bootstrap.ps1 first."}'; exit 1 }
& webfetch-cli doctor --json
exit $LASTEXITCODE
