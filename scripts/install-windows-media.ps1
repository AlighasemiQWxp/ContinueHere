param(
    [string]$Version = '1.26.10'
)

$ErrorActionPreference = 'Stop'

if (-not $env:LOCALAPPDATA) {
    throw 'LOCALAPPDATA is required for the private GStreamer installation.'
}

$packageBaseUrl = "https://gstreamer.freedesktop.org/data/pkg/windows/$Version/msvc"
$downloadRoot = Join-Path $env:LOCALAPPDATA "ContinueHere\downloads\gstreamer-$Version"
$extractRoot = Join-Path $env:LOCALAPPDATA "ContinueHere\dependencies\gstreamer-$Version"
$mediaRoot = Join-Path $extractRoot 'PFiles64\gstreamer\1.0\msvc_x86_64'
$packages = @(
    "gstreamer-1.0-msvc-x86_64-$Version.msi",
    "gstreamer-1.0-devel-msvc-x86_64-$Version.msi"
)

New-Item -ItemType Directory -Force -Path $downloadRoot | Out-Null
New-Item -ItemType Directory -Force -Path $extractRoot | Out-Null

foreach ($package in $packages) {
    $packagePath = Join-Path $downloadRoot $package
    $checksumPath = "$packagePath.sha256sum"

    & curl.exe --fail --location --output $checksumPath "$packageBaseUrl/$package.sha256sum"
    if ($LASTEXITCODE -ne 0) {
        throw "Failed to download the checksum for $package."
    }

    $expectedHash = ((Get-Content -LiteralPath $checksumPath -Raw).Trim() -split '\s+')[0]
    $downloadRequired = -not (Test-Path -LiteralPath $packagePath)
    if (-not $downloadRequired) {
        $currentHash = (Get-FileHash -LiteralPath $packagePath -Algorithm SHA256).Hash
        $downloadRequired = $currentHash -ne $expectedHash
    }

    if ($downloadRequired) {
        & curl.exe --fail --location --output $packagePath "$packageBaseUrl/$package"
        if ($LASTEXITCODE -ne 0) {
            throw "Failed to download $package."
        }
    }

    $actualHash = (Get-FileHash -LiteralPath $packagePath -Algorithm SHA256).Hash
    if ($actualHash -ne $expectedHash) {
        throw "Checksum verification failed for $package."
    }

    & msiexec.exe /a $packagePath /qn "TARGETDIR=$extractRoot"
    if ($LASTEXITCODE -ne 0) {
        throw "GStreamer package extraction failed with exit code $LASTEXITCODE."
    }
}

$runtimeReady = Test-Path -LiteralPath (Join-Path $mediaRoot 'bin\gstreamer-1.0-0.dll')
$pkgConfigReady = Test-Path -LiteralPath (Join-Path $mediaRoot 'bin\pkg-config.exe')
$developmentReady = Test-Path -LiteralPath (Join-Path $mediaRoot 'lib\pkgconfig\gstreamer-app-1.0.pc')

if (-not ($runtimeReady -and $pkgConfigReady -and $developmentReady)) {
    throw 'The private GStreamer installation is incomplete.'
}

Write-Host "GStreamer $Version is ready at $mediaRoot"
