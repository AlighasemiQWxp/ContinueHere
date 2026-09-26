$ErrorActionPreference = 'Stop'

$projectRoot = Split-Path -Parent $PSScriptRoot
. (Join-Path $PSScriptRoot 'android-environment.ps1')
Initialize-AndroidEnvironment

Push-Location $projectRoot
try {
    cargo apk build --package continuehere_client --target aarch64-linux-android --lib
    if ($LASTEXITCODE -ne 0) {
        throw "Android build failed with exit code $LASTEXITCODE."
    }
}
finally {
    Pop-Location
}
