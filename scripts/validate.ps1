$ErrorActionPreference = 'Stop'
& (Join-Path $PSScriptRoot 'use-windows-media.ps1')

function Invoke-ValidationStep {
    param([string]$Program, [string[]]$Arguments)
    & $Program @Arguments
    if ($LASTEXITCODE -ne 0) {
        throw "$Program failed with exit code $LASTEXITCODE."
    }
}

function Resolve-FlutterDependencies {
    & flutter pub get
    if ($LASTEXITCODE -eq 0) {
        return
    }

    Write-Warning 'Online Pub resolution failed. Retrying from the local package cache.'
    Invoke-ValidationStep 'flutter' @('pub', 'get', '--offline')
}

$projectRoot = Split-Path -Parent $PSScriptRoot
Push-Location -LiteralPath $projectRoot
try {
    Push-Location -LiteralPath (Join-Path $projectRoot 'apps/client')
    try {
        Resolve-FlutterDependencies
        Invoke-ValidationStep 'flutter_rust_bridge_codegen' @('generate')
        Invoke-ValidationStep 'dart' @('format', 'lib', 'test')
        Invoke-ValidationStep 'dart' @('format', '--output=none', '--set-exit-if-changed', 'lib', 'test')
    }
    finally {
        Pop-Location
    }

    Invoke-ValidationStep 'cargo' @('fmt', '--all')
    Invoke-ValidationStep 'cargo' @('fmt', '--all', '--', '--check')
    Invoke-ValidationStep 'cargo' @('check', '--workspace', '--all-targets')
    Invoke-ValidationStep 'cargo' @('clippy', '--workspace', '--all-targets', '--all-features', '--', '-D', 'warnings')
    Invoke-ValidationStep 'cargo' @('test', '--workspace')
    Invoke-ValidationStep 'cargo' @('build', '-p', 'continuehere_client_slint', '--release')

    Push-Location -LiteralPath (Join-Path $projectRoot 'apps/client')
    try {
        Invoke-ValidationStep 'flutter' @('analyze')
        Invoke-ValidationStep 'flutter' @('test')
        Invoke-ValidationStep 'flutter' @('build', 'windows')
    }
    finally {
        Pop-Location
    }
}
finally {
    Pop-Location
}
