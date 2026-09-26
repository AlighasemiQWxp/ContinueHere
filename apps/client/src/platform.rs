#[cfg(not(target_os = "android"))]
use std::env;
use std::{io, path::PathBuf};

#[cfg(target_os = "android")]
mod android;
pub(super) mod media;

pub(super) type PlatformResult<T> = Result<T, Box<dyn std::error::Error>>;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum SelectionKind {
    File,
    Folder,
    Media,
    Image,
    Video,
    Pdf,
    PowerPoint,
}

impl SelectionKind {
    pub(super) fn parse(value: &str) -> PlatformResult<Self> {
        match value {
            "file" => Ok(Self::File),
            "folder" => Ok(Self::Folder),
            "media" => Ok(Self::Media),
            "image" => Ok(Self::Image),
            "video" => Ok(Self::Video),
            "pdf" => Ok(Self::Pdf),
            "powerpoint" => Ok(Self::PowerPoint),
            _ => Err("Unknown content category.".into()),
        }
    }
}

pub(super) type SelectionResult = Result<Option<PathBuf>, String>;

pub(super) fn select_file(
    kind: SelectionKind,
    completion: impl FnOnce(SelectionResult) + Send + 'static,
) -> PlatformResult<()> {
    #[cfg(target_os = "windows")]
    {
        let mut dialog = rfd::FileDialog::new();
        dialog = match kind {
            SelectionKind::Folder => {
                completion(Ok(dialog.pick_folder()));
                return Ok(());
            }
            SelectionKind::File => dialog,
            SelectionKind::Video => {
                dialog.add_filter("Video", &["mp4", "m4v", "mkv", "webm", "mov", "avi"])
            }
            SelectionKind::Image => {
                dialog.add_filter("Images", &["png", "jpg", "jpeg", "gif", "webp", "bmp"])
            }
            SelectionKind::Media => dialog.add_filter(
                "Media",
                &[
                    "mp4", "m4v", "mkv", "webm", "mov", "avi", "png", "jpg", "jpeg", "gif", "webp",
                    "bmp", "mp3", "wav", "flac", "ogg", "m4a", "aac",
                ],
            ),
            SelectionKind::Pdf => dialog.add_filter("PDF", &["pdf"]),
            SelectionKind::PowerPoint => dialog.add_filter("PowerPoint", &["ppt", "pptx"]),
        };
        completion(Ok(dialog.pick_file()));
        Ok(())
    }
    #[cfg(target_os = "android")]
    {
        select_android_file(kind, completion)
    }
    #[cfg(not(any(target_os = "windows", target_os = "android")))]
    {
        let _ = (kind, completion);
        Err("File selection is not supported on this platform.".into())
    }
}

pub(super) fn select_directory(
    completion: impl FnOnce(SelectionResult) + Send + 'static,
) -> PlatformResult<()> {
    #[cfg(target_os = "windows")]
    {
        completion(Ok(rfd::FileDialog::new().pick_folder()));
        Ok(())
    }
    #[cfg(target_os = "android")]
    {
        let _ = completion;
        Err("Android folder selection is not available yet.".into())
    }
    #[cfg(not(any(target_os = "windows", target_os = "android")))]
    {
        let _ = completion;
        Err("Folder selection is not supported on this platform.".into())
    }
}

#[cfg(target_os = "android")]
fn select_android_file(
    kind: SelectionKind,
    completion: impl FnOnce(SelectionResult) + Send + 'static,
) -> PlatformResult<()> {
    let mut dialog = robius_file_picker::FileDialog::new();
    dialog = match kind {
        SelectionKind::File => dialog.set_mime_type("*/*"),
        SelectionKind::Folder => return select_directory(completion),
        SelectionKind::Video => dialog
            .set_mime_type("video/*")
            .add_filter("Video", &["mp4", "m4v", "mkv", "webm", "mov", "avi"]),
        SelectionKind::Image => dialog
            .set_mime_type("image/*")
            .add_filter("Images", &["png", "jpg", "jpeg", "gif", "webp", "bmp"]),
        SelectionKind::Media => dialog.set_mime_type("*/*").add_filter(
            "Media",
            &[
                "mp4", "m4v", "mkv", "webm", "mov", "avi", "png", "jpg", "jpeg", "gif", "webp",
                "bmp", "mp3", "wav", "flac", "ogg", "m4a", "aac",
            ],
        ),
        SelectionKind::Pdf => dialog
            .set_mime_type("application/pdf")
            .add_filter("PDF", &["pdf"]),
        SelectionKind::PowerPoint => dialog
            .set_mime_type("application/vnd.ms-powerpoint")
            .add_filter("PowerPoint", &["ppt", "pptx"]),
    };
    let callback =
        move |result: robius_file_picker::Result<Option<robius_file_picker::PickedFile>>| {
            let result = result
                .map_err(|error| error.to_string())
                .and_then(|picked| picked.map(stage_android_file).transpose());
            completion(result);
        };
    match kind {
        SelectionKind::Image => dialog.pick_image(callback)?,
        SelectionKind::Video => dialog.pick_video(callback)?,
        _ => dialog.pick_file(callback)?,
    }
    Ok(())
}

#[cfg(target_os = "android")]
fn stage_android_file(picked: robius_file_picker::PickedFile) -> Result<PathBuf, String> {
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT_IMPORT: AtomicU64 = AtomicU64::new(1);
    let local = picked
        .into_local_file()
        .map_err(|error| error.to_string())?;
    let file_name = local
        .display_name()
        .or_else(|| local.path().file_name().and_then(|value| value.to_str()))
        .and_then(|value| std::path::Path::new(value).file_name())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| std::ffi::OsStr::new("attachment"));
    let import = project_directory()
        .map_err(|error| error.to_string())?
        .join("imports")
        .join(format!(
            "{}-{}",
            std::process::id(),
            NEXT_IMPORT.fetch_add(1, Ordering::Relaxed)
        ));
    std::fs::create_dir_all(&import).map_err(|error| error.to_string())?;
    let destination = import.join(file_name);
    if let Err(error) = std::fs::copy(local.path(), &destination) {
        let _ = std::fs::remove_dir_all(import);
        return Err(error.to_string());
    }
    Ok(destination)
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

pub(super) fn open_document(
    path: &std::path::Path,
    continuation: continuehere::DocumentContinuation,
) -> PlatformResult<()> {
    if unsafe_file(path) {
        return Err("Executable and script files cannot be opened here.".into());
    }
    let supported = match continuation {
        continuehere::DocumentContinuation::PdfPage(_) => extension(path) == "pdf",
        continuehere::DocumentContinuation::PowerPointSlide(_) => {
            matches!(extension(path).as_str(), "ppt" | "pptx")
        }
    };
    let metadata = path.symlink_metadata()?;
    if !path.is_absolute() || !supported || !metadata.file_type().is_file() || metadata.len() == 0 {
        return Err("This document is missing or no longer matches its saved type.".into());
    }
    #[cfg(target_os = "windows")]
    {
        open_document_windows(path, continuation)
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = (path, continuation);
        Err("Resumable document opening is currently supported on Windows only.".into())
    }
}

#[cfg(target_os = "windows")]
fn open_document_windows(
    path: &std::path::Path,
    continuation: continuehere::DocumentContinuation,
) -> PlatformResult<()> {
    match continuation {
        continuehere::DocumentContinuation::PdfPage(page) => open_pdf(path, page),
        continuehere::DocumentContinuation::PowerPointSlide(slide) => open_powerpoint(path, slide),
    }
}

#[cfg(target_os = "windows")]
fn open_pdf(path: &std::path::Path, page: u32) -> PlatformResult<()> {
    const ACROBAT_SCRIPT: &str = "$ErrorActionPreference='Stop';$app=New-Object -ComObject AcroExch.App;$document=New-Object -ComObject AcroExch.AVDoc;if(-not $document.Open($env:CONTINUEHERE_DOCUMENT_PATH,'')){exit 2};$app.Show();$document.BringToFront();$document.GetAVPageView().Goto(([int]$env:CONTINUEHERE_DOCUMENT_POSITION)-1)";
    if run_document_script(ACROBAT_SCRIPT, path, page)?.success() {
        return Ok(());
    }
    let edge = ["ProgramFiles(x86)", "ProgramFiles", "LOCALAPPDATA"]
        .into_iter()
        .filter_map(env::var_os)
        .map(PathBuf::from)
        .map(|root| root.join("Microsoft/Edge/Application/msedge.exe"))
        .find(|candidate| candidate.is_file())
        .ok_or("Microsoft Edge is required to open a PDF at a specific page.")?;
    let mut url = url::Url::from_file_path(path)
        .map_err(|_| "The PDF path cannot be converted to a local file URL.")?;
    url.set_fragment(Some(&format!("page={page}")));
    std::process::Command::new(edge)
        .arg("--new-window")
        .arg(url.as_str())
        .spawn()?;
    Ok(())
}

#[cfg(target_os = "windows")]
fn open_powerpoint(path: &std::path::Path, slide: u32) -> PlatformResult<()> {
    const SCRIPT: &str = "$ErrorActionPreference='Stop';$app=New-Object -ComObject PowerPoint.Application;$app.Visible=$true;$deck=$app.Presentations.Open($env:CONTINUEHERE_DOCUMENT_PATH,$false,$false,$true);$deck.Windows.Item(1).View.GotoSlide([int]$env:CONTINUEHERE_DOCUMENT_POSITION);$deck.Windows.Item(1).Activate()";
    let status = run_document_script(SCRIPT, path, slide)?;
    if !status.success() {
        return Err(
            "Desktop PowerPoint could not open this presentation at the requested slide.".into(),
        );
    }
    Ok(())
}

#[cfg(target_os = "windows")]
fn run_document_script(
    script: &str,
    path: &std::path::Path,
    position: u32,
) -> PlatformResult<std::process::ExitStatus> {
    Ok(std::process::Command::new("powershell.exe")
        .args([
            "-NoLogo",
            "-NoProfile",
            "-NonInteractive",
            "-WindowStyle",
            "Hidden",
            "-Command",
            script,
        ])
        .env("CONTINUEHERE_DOCUMENT_PATH", path)
        .env("CONTINUEHERE_DOCUMENT_POSITION", position.to_string())
        .status()?)
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

#[cfg(target_os = "android")]
pub(super) fn initialize_android(app: slint::android::AndroidApp) -> PlatformResult<()> {
    android::initialize(app)?;
    Ok(())
}

#[cfg(target_os = "android")]
pub(super) fn shutdown_android() -> PlatformResult<()> {
    android::shutdown()?;
    Ok(())
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

#[cfg(all(unix, not(target_os = "android"), not(target_os = "macos")))]
fn platform_data_directory() -> io::Result<PathBuf> {
    if let Some(directory) = env::var_os("XDG_DATA_HOME") {
        return Ok(PathBuf::from(directory).join("ContinueHere"));
    }
    Ok(environment_directory("HOME")?
        .join(".local/share")
        .join("ContinueHere"))
}

#[cfg(target_os = "android")]
fn platform_data_directory() -> io::Result<PathBuf> {
    android::project_directory()
}

#[cfg(not(any(target_os = "windows", target_os = "macos", unix)))]
fn platform_data_directory() -> io::Result<PathBuf> {
    env::current_dir()
}

#[cfg(not(target_os = "android"))]
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
