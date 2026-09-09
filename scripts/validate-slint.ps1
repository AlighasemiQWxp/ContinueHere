$ErrorActionPreference = 'Stop'
& (Join-Path $PSScriptRoot 'use-windows-media.ps1')

function Invoke-SlintValidation {
    param([string[]]$Arguments)
    & cargo @Arguments
    if ($LASTEXITCODE -ne 0) { throw "cargo failed with exit code $LASTEXITCODE." }
}

Push-Location -LiteralPath (Split-Path -Parent $PSScriptRoot)
try {
    Invoke-SlintValidation @('fmt', '--all')
    Invoke-SlintValidation @('fmt', '--all', '--', '--check')
    Invoke-SlintValidation @('check', '--workspace', '--all-targets')
    Invoke-SlintValidation @('clippy', '--workspace', '--all-targets', '--all-features', '--', '-D', 'warnings')
    Invoke-SlintValidation @('test', '--workspace')
    Invoke-SlintValidation @('build', '-p', 'continuehere_client_slint', '--release')
}
finally {
    Pop-Location
}
