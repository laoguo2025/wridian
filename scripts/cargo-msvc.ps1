param(
  [Parameter(ValueFromRemainingArguments = $true)]
  [string[]] $CargoArgs
)

$ErrorActionPreference = "Stop"

$vswhere = Join-Path ${env:ProgramFiles(x86)} "Microsoft Visual Studio\Installer\vswhere.exe"
$vsInstall = if (Test-Path -LiteralPath $vswhere) {
  & $vswhere -latest -products * -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath
} else {
  $null
}

$vcvars = if ($vsInstall) {
  Join-Path $vsInstall "VC\Auxiliary\Build\vcvars64.bat"
} else {
  Get-ChildItem "${env:ProgramFiles(x86)}\Microsoft Visual Studio\2022" -Recurse -Filter vcvars64.bat -ErrorAction SilentlyContinue |
    Select-Object -First 1 -ExpandProperty FullName
}

if (-not (Test-Path -LiteralPath $vcvars)) {
  throw "vcvars64.bat not found. Install Visual Studio Build Tools with the C++ workload."
}

$cargoCommand = if ($CargoArgs.Count) {
  "cargo " + (($CargoArgs | ForEach-Object { '"' + ($_ -replace '"', '\"') + '"' }) -join " ")
} else {
  "cargo --version"
}

cmd.exe /d /s /c "`"$vcvars`" >nul && $cargoCommand"
if ($LASTEXITCODE -ne 0) {
  exit $LASTEXITCODE
}
