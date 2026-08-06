$ErrorActionPreference = "Stop"
$scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$script = Join-Path $scriptDir "uninstall.py"
$python = Get-Command python -ErrorAction SilentlyContinue
if ($python) { & python $script @args } else {
    $py = Get-Command py -ErrorAction SilentlyContinue
    if (-not $py) { throw "Python 3.11+ is required." }
    & py -3 $script @args
}
if ($LASTEXITCODE -ne 0) { throw "Uninstall failed with exit code $LASTEXITCODE" }
