function Initialize-AndroidEnvironment {
    $androidPlatform = 'android-36'
    $androidBuildToolsVersion = '36.0.0'

    $sdkCandidates = @()
    if ($env:ANDROID_HOME) {
        $sdkCandidates += $env:ANDROID_HOME
    }
    if ($env:ANDROID_SDK_ROOT) {
        $sdkCandidates += $env:ANDROID_SDK_ROOT
    }
    if ($env:LOCALAPPDATA) {
        $sdkCandidates += Join-Path $env:LOCALAPPDATA 'Android\Sdk'
    }

    $sdkPath = $sdkCandidates |
        Where-Object { Test-Path -LiteralPath $_ -PathType Container } |
        Select-Object -First 1
    if (-not $sdkPath) {
        throw 'Android SDK was not found. Install it with Android Studio or set ANDROID_HOME.'
    }

    $env:ANDROID_HOME = $sdkPath
    Remove-Item Env:ANDROID_SDK_ROOT -ErrorAction SilentlyContinue

    $platformPath = Join-Path (Join-Path $sdkPath 'platforms') $androidPlatform
    if (-not (Test-Path -LiteralPath $platformPath -PathType Container)) {
        throw 'Android SDK Platform 36 was not found. Install it with Android Studio.'
    }

    $buildToolsPath = Join-Path (Join-Path $sdkPath 'build-tools') $androidBuildToolsVersion
    if (-not (Test-Path -LiteralPath $buildToolsPath -PathType Container)) {
        throw 'Android SDK Build Tools 36.0.0 were not found. Install them with Android Studio.'
    }

    $env:ANDROID_PLATFORM = $androidPlatform
    $env:ANDROID_BUILD_TOOLS_VERSION = $androidBuildToolsVersion

    $ndkCandidates = @()
    if ($env:ANDROID_NDK_ROOT) {
        $ndkCandidates += Get-Item -LiteralPath $env:ANDROID_NDK_ROOT -ErrorAction SilentlyContinue
    }
    if ($env:ANDROID_NDK_HOME) {
        $ndkCandidates += Get-Item -LiteralPath $env:ANDROID_NDK_HOME -ErrorAction SilentlyContinue
    }

    $ndkDirectory = Join-Path $sdkPath 'ndk'
    if (Test-Path -LiteralPath $ndkDirectory -PathType Container) {
        $ndkCandidates += Get-ChildItem -LiteralPath $ndkDirectory -Directory
    }

    $ndkPath = $ndkCandidates |
        Sort-Object { [version]$_.Name } -Descending |
        Select-Object -First 1 -ExpandProperty FullName
    if (-not $ndkPath) {
        throw 'Android NDK was not found. Install it with Android Studio or set ANDROID_NDK_ROOT.'
    }

    $env:ANDROID_NDK_ROOT = $ndkPath
    $env:ANDROID_NDK_HOME = $ndkPath

    $jdkCandidates = @()
    if ($env:JAVA_HOME) {
        $jdkCandidates += $env:JAVA_HOME
    }
    $jdkCandidates += 'C:\Program Files\Android\Android Studio\jbr'

    $jdkPath = $jdkCandidates |
        Where-Object { Test-Path -LiteralPath (Join-Path $_ 'bin\java.exe') -PathType Leaf } |
        Select-Object -First 1
    if (-not $jdkPath) {
        throw 'JDK was not found. Install Android Studio or set JAVA_HOME.'
    }

    $env:JAVA_HOME = $jdkPath
    $env:JAVA_SOURCE_VERSION = '17'
    $env:JAVA_TARGET_VERSION = '17'
    $env:PATH = "$(Join-Path $jdkPath 'bin');$env:PATH"
}
