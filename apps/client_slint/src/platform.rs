use std::{env, io, path::PathBuf};

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
