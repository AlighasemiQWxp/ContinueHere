use std::{env, io, path::PathBuf};

pub(super) mod media;

type PlatformResult<T> = Result<T, Box<dyn std::error::Error>>;

pub(super) fn select_file(video: bool) -> PlatformResult<Option<PathBuf>> {
    #[cfg(target_os = "windows")]
    {
        let mut dialog = rfd::FileDialog::new();
        if video {
            dialog = dialog.add_filter("Video files", &["mp4", "m4v", "mkv", "webm", "mov", "avi"]);
        }
        Ok(dialog.pick_file())
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = video;
        Err("File selection is currently supported on Windows only.".into())
    }
}

pub(super) fn select_directory() -> PlatformResult<Option<PathBuf>> {
    #[cfg(target_os = "windows")]
    {
        Ok(rfd::FileDialog::new().pick_folder())
    }
    #[cfg(not(target_os = "windows"))]
    {
        Err("Folder selection is currently supported on Windows only.".into())
    }
}

pub(super) fn open_url(value: &str) -> PlatformResult<()> {
    let url = url::Url::parse(value)?;
    if !matches!(url.scheme(), "http" | "https") || url.host_str().is_none() {
        return Err("Only HTTP and HTTPS links can be opened.".into());
    }
    open::that(url.as_str())?;
    Ok(())
}

pub(super) fn open_file(path: &std::path::Path) -> PlatformResult<()> {
    if unsafe_file(path) {
        return Err("Executable and script files cannot be opened here.".into());
    }
    open::that(path)?;
    Ok(())
}

pub(super) fn unsafe_file(path: &std::path::Path) -> bool {
    matches!(
        extension(path).as_str(),
        "apk"
            | "app"
            | "appimage"
            | "appx"
            | "bat"
            | "cmd"
            | "com"
            | "deb"
            | "desktop"
            | "dll"
            | "exe"
            | "ipa"
            | "jar"
            | "js"
            | "jse"
            | "lnk"
            | "msi"
            | "msix"
            | "ps1"
            | "psm1"
            | "reg"
            | "rpm"
            | "scr"
            | "sh"
            | "url"
            | "vbe"
            | "vbs"
            | "wsf"
            | "wsh"
    )
}

pub(super) fn extension(path: &std::path::Path) -> String {
    path.extension()
        .and_then(|value| value.to_str())
        .unwrap_or("")
        .to_ascii_lowercase()
}

pub(super) fn reduce_motion() -> bool {
    #[cfg(target_os = "windows")]
    {
        winreg::RegKey::predef(winreg::enums::HKEY_CURRENT_USER)
            .open_subkey("Control Panel\\Desktop\\WindowMetrics")
            .and_then(|key| key.get_value::<String, _>("MinAnimate"))
            .is_ok_and(|value| value == "0")
    }
    #[cfg(not(target_os = "windows"))]
    {
        false
    }
}

pub(super) fn project_directory() -> io::Result<PathBuf> {
    let directory = platform_data_directory()?;
    std::fs::create_dir_all(&directory)?;
    Ok(directory)
}

#[cfg(target_os = "windows")]
fn platform_data_directory() -> io::Result<PathBuf> {
    Ok(environment_directory("APPDATA")?
        .join("AlighasemiQWxp")
        .join("ContinueHere"))
}

#[cfg(target_os = "macos")]
fn platform_data_directory() -> io::Result<PathBuf> {
    Ok(environment_directory("HOME")?
        .join("Library/Application Support")
        .join("ContinueHere"))
}

#[cfg(all(unix, not(target_os = "macos")))]
fn platform_data_directory() -> io::Result<PathBuf> {
    if let Some(directory) = env::var_os("XDG_DATA_HOME") {
        return Ok(PathBuf::from(directory).join("ContinueHere"));
    }
    Ok(environment_directory("HOME")?
        .join(".local/share")
        .join("ContinueHere"))
}

#[cfg(not(any(target_os = "windows", target_os = "macos", unix)))]
fn platform_data_directory() -> io::Result<PathBuf> {
    env::current_dir()
}

fn environment_directory(name: &str) -> io::Result<PathBuf> {
    env::var_os(name)
        .map(PathBuf::from)
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, format!("{name} is unavailable")))
}

#[cfg(test)]
mod tests {
    use super::unsafe_file;
    use std::path::Path;

    #[test]
    fn opening_keeps_the_reference_executable_and_script_boundary() {
        for path in [
            "photo.jpg.EXE",
            "setup.Msix",
            "shortcut.lnk",
            "script.PS1",
            "page.url",
        ] {
            assert!(unsafe_file(Path::new(path)));
        }
        for path in ["photo.EXE.jpg", "report.pdf", "movie.mp4", "readme"] {
            assert!(!unsafe_file(Path::new(path)));
        }
    }
}
