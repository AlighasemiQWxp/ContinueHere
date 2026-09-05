use std::time::Duration;

use super::HandoffError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PlaybackPosition {
    milliseconds: u64,
}

impl PlaybackPosition {
    pub fn new(duration: Duration) -> Result<Self, HandoffError> {
        let milliseconds = u64::try_from(duration.as_millis())
            .map_err(|_| HandoffError::PlaybackPositionTooLarge)?;
        Ok(Self { milliseconds })
    }

    pub(crate) const fn from_millis(milliseconds: u64) -> Self {
        Self { milliseconds }
    }

    pub const fn as_millis(self) -> u64 {
        self.milliseconds
    }

    pub const fn duration(self) -> Duration {
        Duration::from_millis(self.milliseconds)
    }
}
