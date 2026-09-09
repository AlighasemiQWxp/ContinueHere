$ErrorActionPreference = 'Stop'
& (Join-Path $PSScriptRoot 'use-windows-media.ps1')
Push-Location -LiteralPath (Split-Path -Parent $PSScriptRoot)
try {
    & cargo run -p continuehere_client_slint
    if ($LASTEXITCODE -ne 0) { throw "Slint client exited with code $LASTEXITCODE." }
}
finally {
    Pop-Location
}
