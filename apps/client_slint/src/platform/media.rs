use std::path::Path;

#[cfg(target_os = "windows")]
mod windows;

pub(crate) enum MediaCommand {
    Pause(bool),
    Seek(u64),
    Volume(f64),
}

pub(crate) struct MediaPlayer {
    #[cfg(target_os = "windows")]
    player: windows::WindowsPlayer,
}

impl MediaPlayer {
    pub(crate) fn open(
        path: &Path,
        position: u64,
        window: &crate::ui::MainWindow,
        generation: i32,
    ) -> super::PlatformResult<Self> {
        #[cfg(target_os = "windows")]
        {
            Ok(Self {
                player: windows::WindowsPlayer::open(path, position, window, generation)?,
            })
        }
        #[cfg(not(target_os = "windows"))]
        {
            let _ = (path, position, window, generation);
            Err("In-app video playback is currently supported on Windows only.".into())
        }
    }

    pub(crate) fn command(&self, command: MediaCommand) -> super::PlatformResult<()> {
        #[cfg(target_os = "windows")]
        {
            self.player.command(command)
        }
        #[cfg(not(target_os = "windows"))]
        {
            match command {
                MediaCommand::Pause(value) => {
                    let _ = value;
                }
                MediaCommand::Seek(value) => {
                    let _ = value;
                }
                MediaCommand::Volume(value) => {
                    let _ = value;
                }
            }
            Err("In-app video playback is currently supported on Windows only.".into())
        }
    }
}
