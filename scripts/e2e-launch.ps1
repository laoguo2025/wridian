param(
  [string] $ExePath = "",
  [string] $DataDir = "",
  [int] $DebugPort = 9222,
  [switch] $StopExisting
)

$ErrorActionPreference = "Stop"

$workspaceRoot = Resolve-Path (Join-Path $PSScriptRoot "..")
if (-not $ExePath) {
  $candidate = Join-Path $workspaceRoot "src-tauri\target\release\wridian.exe"
  if (Test-Path -LiteralPath $candidate) {
    $ExePath = $candidate
  } else {
    throw "Wridian exe not found. Build first or pass -ExePath."
  }
}

if (-not $DataDir) {
  $DataDir = Join-Path $workspaceRoot ".workbench\runtime\e2e-data"
}

New-Item -ItemType Directory -Force -Path $DataDir | Out-Null
$resolvedDataDir = (Resolve-Path $DataDir).Path
$webviewDataDir = Join-Path $resolvedDataDir "webview2"
New-Item -ItemType Directory -Force -Path $webviewDataDir | Out-Null
$webviewArgs = "--remote-debugging-port=$DebugPort --remote-allow-origins=*"

if ($StopExisting) {
  Get-Process -Name wridian -ErrorAction SilentlyContinue | Stop-Process -Force
  Start-Sleep -Milliseconds 500
}

$logPath = Join-Path $DataDir "e2e-launch.json"
$payload = [ordered]@{
  exePath = (Resolve-Path $ExePath).Path
  dataDir = $resolvedDataDir
  debugPort = $DebugPort
  devtoolsUrl = "http://127.0.0.1:$DebugPort"
  webviewDataDir = $webviewDataDir
  startedAt = (Get-Date).ToString("o")
}
$payload | ConvertTo-Json | Set-Content -Encoding UTF8 -Path $logPath

$env:WRIDIAN_E2E = "1"
$env:WRIDIAN_DATA_DIR = $resolvedDataDir
$env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS = $webviewArgs
$env:WEBVIEW2_USER_DATA_FOLDER = $webviewDataDir
Start-Process -FilePath $ExePath -WorkingDirectory (Split-Path -Parent $ExePath) -WindowStyle Hidden
Write-Host "Wridian E2E launched"
Write-Host "DataDir: $resolvedDataDir"
Write-Host "DevTools: http://127.0.0.1:$DebugPort"
Write-Host "LaunchInfo: $logPath"
