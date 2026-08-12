$ErrorActionPreference = "Stop"
$version = if ($env:LEXMOUNT_WEBFETCH_CLI_VERSION) { $env:LEXMOUNT_WEBFETCH_CLI_VERSION } else { "0.1.1" }
$downloadBaseUrl = if ($env:LEXMOUNT_WEBFETCH_CLI_DOWNLOAD_BASE_URL) { $env:LEXMOUNT_WEBFETCH_CLI_DOWNLOAD_BASE_URL.TrimEnd('/') } else { "https://cli-bin-1377899528.cos.ap-nanjing.myqcloud.com/releases/webfetch-cli" }
$architecture = if ($env:PROCESSOR_ARCHITEW6432) { $env:PROCESSOR_ARCHITEW6432 } else { $env:PROCESSOR_ARCHITECTURE }
if ($architecture -ne "AMD64") { throw "Only Windows x64 is supported" }
$asset = "webfetch-cli-v$version-x86_64-pc-windows-msvc.exe"
$repo = "$downloadBaseUrl/v$version"
$tmp = Join-Path ([IO.Path]::GetTempPath()) ([Guid]::NewGuid().ToString())
New-Item -ItemType Directory -Path $tmp | Out-Null
try {
  Invoke-WebRequest -UseBasicParsing "$repo/$asset" -OutFile (Join-Path $tmp $asset)
  Invoke-WebRequest -UseBasicParsing "$repo/SHA256SUMS" -OutFile (Join-Path $tmp "SHA256SUMS")
  # GNU sha256sum prefixes binary filenames with `*`; shasum uses plain whitespace.
  $line = Get-Content (Join-Path $tmp "SHA256SUMS") | Where-Object { $_ -match "\s+\*?$([regex]::Escape($asset))$" } | Select-Object -First 1
  if (-not $line) { throw "No checksum published for $asset" }
  $expected = ($line -split "\s+")[0].ToLowerInvariant()
  $actual = (Get-FileHash (Join-Path $tmp $asset) -Algorithm SHA256).Hash.ToLowerInvariant()
  if ($expected -ne $actual) { throw "SHA-256 mismatch for $asset" }
  $skillDir = Split-Path -Parent $PSScriptRoot
  $installDir = if ($env:LEXMOUNT_WEBFETCH_CLI_INSTALL_DIR) { $env:LEXMOUNT_WEBFETCH_CLI_INSTALL_DIR } else { Join-Path $skillDir "bin" }
  New-Item -ItemType Directory -Path $installDir -Force | Out-Null
  Copy-Item (Join-Path $tmp $asset) (Join-Path $installDir "webfetch-cli.exe") -Force
  & (Join-Path $installDir "webfetch-cli.exe") version
  Write-Output "Installed webfetch-cli to $installDir\webfetch-cli.exe"
} finally { Remove-Item -Recurse -Force $tmp -ErrorAction SilentlyContinue }
