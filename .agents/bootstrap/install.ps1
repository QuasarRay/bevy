$ErrorActionPreference = "Stop"
$scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$installer = Join-Path $scriptDir "install.py"
$python = Get-Command python -ErrorAction SilentlyContinue
if ($python) {
    & python $installer @args
} else {
    $py = Get-Command py -ErrorAction SilentlyContinue
    if (-not $py) { throw "Python 3.11+ is required." }
    & py -3 $installer @args
}
if ($LASTEXITCODE -ne 0) { throw "Installation failed with exit code $LASTEXITCODE" }
