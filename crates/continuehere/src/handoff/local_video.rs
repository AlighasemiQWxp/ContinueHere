use std::{
    path::{Path, PathBuf},
    time::Duration,
};

use crate::transfer::{FileTransfer, FileTransferId};

use super::{HandoffError, PlaybackPosition};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalVideoHandoff {
    file_path: PathBuf,
    playback_position: PlaybackPosition,
    transfer_id: Option<FileTransferId>,
}

impl LocalVideoHandoff {
    pub(crate) fn new(file_path: PathBuf, position: Duration) -> Result<Self, HandoffError> {
        validate_video_path(&file_path)?;
        Ok(Self {
            file_path,
            playback_position: PlaybackPosition::new(position)?,
            transfer_id: None,
        })
    }

    pub fn file_path(&self) -> &Path {
        &self.file_path
    }

    pub const fn playback_position(&self) -> PlaybackPosition {
        self.playback_position
    }

    pub fn transfer_id(&self) -> Option<&FileTransferId> {
        self.transfer_id.as_ref()
    }

    pub(crate) fn received(
        transfer: &FileTransfer,
        position: PlaybackPosition,
    ) -> Result<Self, HandoffError> {
        let path = transfer
            .destination()
            .ok_or(HandoffError::InvalidLocalVideo)?;
        validate_video_path(path)?;
        Ok(Self {
            file_path: path.to_path_buf(),
            playback_position: position,
            transfer_id: Some(transfer.id().clone()),
        })
    }
}

fn validate_video_path(path: &Path) -> Result<(), HandoffError> {
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .ok_or(HandoffError::InvalidLocalVideo)?
        .to_ascii_lowercase();
    if !path.is_absolute()
        || !matches!(
            extension.as_str(),
            "mp4" | "m4v" | "mkv" | "webm" | "mov" | "avi"
        )
    {
        return Err(HandoffError::InvalidLocalVideo);
    }
    let metadata = path
        .symlink_metadata()
        .map_err(|_| HandoffError::InvalidLocalVideo)?;
    if !metadata.file_type().is_file() || metadata.len() == 0 {
        return Err(HandoffError::InvalidLocalVideo);
    }
    Ok(())
}
