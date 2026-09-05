use std::time::Duration;

use url::Url;

use super::{HandoffError, PlaybackPosition, url::validate_url};

const VIDEO_ID_LENGTH: usize = 11;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct YouTubeHandoff {
    video_id: String,
    playback_position: PlaybackPosition,
}

impl YouTubeHandoff {
    pub fn new(url: &str, playback_position: Duration) -> Result<Self, HandoffError> {
        let normalized = validate_url(url)?;
        let parsed = Url::parse(&normalized).map_err(|_| HandoffError::InvalidYouTubeUrl)?;
        if parsed.scheme() != "https" {
            return Err(HandoffError::InvalidYouTubeUrl);
        }
        if parsed.port_or_known_default() != Some(443) {
            return Err(HandoffError::InvalidYouTubeUrl);
        }
        let video_id = extract_video_id(&parsed)?;
        Self::from_parts(video_id, PlaybackPosition::new(playback_position)?)
    }

    pub fn video_id(&self) -> &str {
        &self.video_id
    }

    pub const fn playback_position(&self) -> PlaybackPosition {
        self.playback_position
    }

    pub fn resume_url(&self) -> String {
        let seconds = self.playback_position.as_millis() / 1_000;
        if seconds == 0 {
            return format!("https://www.youtube.com/watch?v={}", self.video_id);
        }
        format!(
            "https://www.youtube.com/watch?v={}&t={seconds}s",
            self.video_id
        )
    }

    pub(crate) fn from_parts(
        video_id: String,
        playback_position: PlaybackPosition,
    ) -> Result<Self, HandoffError> {
        validate_video_id(&video_id)?;
        Ok(Self {
            video_id,
            playback_position,
        })
    }
}

fn extract_video_id(url: &Url) -> Result<String, HandoffError> {
    let host = url.host_str().ok_or(HandoffError::InvalidYouTubeUrl)?;
    let video_id = match host {
        "youtube.com" | "www.youtube.com" | "m.youtube.com" if url.path() == "/watch" => {
            let identifiers = url
                .query_pairs()
                .filter(|(key, _)| key == "v")
                .map(|(_, value)| value.into_owned())
                .collect::<Vec<_>>();
            if identifiers.len() != 1 {
                return Err(HandoffError::InvalidYouTubeUrl);
            }
            identifiers.into_iter().next()
        }
        "youtu.be" => {
            let segments = url
                .path_segments()
                .ok_or(HandoffError::InvalidYouTubeUrl)?
                .filter(|segment| !segment.is_empty())
                .collect::<Vec<_>>();
            if segments.len() != 1 {
                return Err(HandoffError::InvalidYouTubeUrl);
            }
            Some(segments[0].to_owned())
        }
        _ => return Err(HandoffError::InvalidYouTubeUrl),
    }
    .ok_or(HandoffError::InvalidYouTubeUrl)?;
    validate_video_id(&video_id)?;
    Ok(video_id)
}

fn validate_video_id(video_id: &str) -> Result<(), HandoffError> {
    if video_id.len() != VIDEO_ID_LENGTH
        || !video_id
            .bytes()
            .all(|value| value.is_ascii_alphanumeric() || value == b'-' || value == b'_')
    {
        return Err(HandoffError::InvalidYouTubeVideoId);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::{HandoffError, YouTubeHandoff};

    #[test]
    fn normalizes_supported_youtube_urls() {
        let handoff = YouTubeHandoff::new(
            "https://youtu.be/dQw4w9WgXcQ?feature=shared",
            Duration::from_secs(452),
        )
        .expect("YouTube handoff should be valid");

        assert_eq!(handoff.video_id(), "dQw4w9WgXcQ");
        assert_eq!(
            handoff.playback_position().duration(),
            Duration::from_secs(452)
        );
        assert_eq!(
            handoff.resume_url(),
            "https://www.youtube.com/watch?v=dQw4w9WgXcQ&t=452s"
        );
    }

    #[test]
    fn rejects_lookalike_hosts_and_invalid_video_identifiers() {
        assert_eq!(
            YouTubeHandoff::new(
                "https://youtube.example/watch?v=dQw4w9WgXcQ",
                Duration::ZERO,
            ),
            Err(HandoffError::InvalidYouTubeUrl)
        );
        assert_eq!(
            YouTubeHandoff::new("https://www.youtube.com/watch?v=short", Duration::ZERO),
            Err(HandoffError::InvalidYouTubeVideoId)
        );
        assert_eq!(
            YouTubeHandoff::new(
                "https://www.youtube.com:8443/watch?v=dQw4w9WgXcQ",
                Duration::ZERO,
            ),
            Err(HandoffError::InvalidYouTubeUrl)
        );
    }
}
