$ErrorActionPreference = 'Stop'

$privateMediaRoots = @()
if ($env:LOCALAPPDATA) {
    $privateDependencies = Join-Path $env:LOCALAPPDATA 'ContinueHere\dependencies'
    if (Test-Path -LiteralPath $privateDependencies) {
        $privateMediaRoots = Get-ChildItem -LiteralPath $privateDependencies -Directory -Filter 'gstreamer-*' |
            Sort-Object Name -Descending |
            ForEach-Object {
                Join-Path $_.FullName 'PFiles64\gstreamer\1.0\msvc_x86_64'
            }
    }
}

$mediaCandidates = @(
    $env:GSTREAMER_1_0_ROOT_MSVC_X86_64,
    $privateMediaRoots,
    'C:\gstreamer\1.0\msvc_x86_64',
    'C:\Program Files\gstreamer\1.0\msvc_x86_64'
)
$mediaRoot = $mediaCandidates | Where-Object {
    $_ -and (Test-Path -LiteralPath (Join-Path $_ 'bin\pkg-config.exe')) -and
    (Test-Path -LiteralPath (Join-Path $_ 'lib\pkgconfig\gstreamer-app-1.0.pc'))
} | Select-Object -First 1

if (-not $mediaRoot) {
    throw 'Install the GStreamer MSVC x64 runtime and development packages with scripts/install-windows-media.ps1. See apps/client/README.md.'
}

$env:PATH = (Join-Path $mediaRoot 'bin') + ';' + $env:PATH
$env:PKG_CONFIG_PATH = (Join-Path $mediaRoot 'lib\pkgconfig')
$env:GSTREAMER_1_0_ROOT_MSVC_X86_64 = $mediaRoot
