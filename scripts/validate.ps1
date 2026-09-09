$ErrorActionPreference = 'Stop'
& (Join-Path $PSScriptRoot 'use-windows-media.ps1')

function Invoke-Validation {
    param([string[]]$Arguments)
    & cargo @Arguments
    if ($LASTEXITCODE -ne 0) { throw "cargo failed with exit code $LASTEXITCODE." }
}

Push-Location -LiteralPath (Split-Path -Parent $PSScriptRoot)
try {
    Invoke-Validation @('fmt', '--all')
    Invoke-Validation @('fmt', '--all', '--', '--check')
    Invoke-Validation @('check', '--workspace', '--all-targets')
    Invoke-Validation @('clippy', '--workspace', '--all-targets', '--all-features', '--', '-D', 'warnings')
    Invoke-Validation @('test', '--workspace')
    Invoke-Validation @('build', '-p', 'continuehere_client', '--release')
}
finally {
    Pop-Location
}
