use std::{
    fs::File,
    io::BufReader,
    path::{Path, PathBuf},
    time::Duration,
};

use image::AnimationDecoder;

use super::support::UiResult;

pub(super) struct ImagePreview {
    path: PathBuf,
    frames: image::Frames<'static>,
}

impl ImagePreview {
    pub(super) fn open(path: &Path) -> UiResult<Option<Self>> {
        Ok(Self::frames(path)?.map(|frames| Self {
            path: path.to_owned(),
            frames,
        }))
    }

    pub(super) fn next(&mut self) -> UiResult<(slint::Image, Duration)> {
        let frame = match self.frames.next() {
            Some(frame) => frame?,
            None => {
                self.frames =
                    Self::frames(&self.path)?.ok_or("The animated image is unavailable.")?;
                self.frames.next().ok_or("The image has no frames.")??
            }
        };
        let (numerator, denominator) = frame.delay().numer_denom_ms();
        let delay = Duration::from_millis(u64::from(numerator / denominator.max(1)).max(10));
        let buffer = frame.into_buffer();
        let pixels = slint::SharedPixelBuffer::<slint::Rgba8Pixel>::clone_from_slice(
            buffer.as_raw(),
            buffer.width(),
            buffer.height(),
        );
        Ok((slint::Image::from_rgba8(pixels), delay))
    }

    fn frames(path: &Path) -> UiResult<Option<image::Frames<'static>>> {
        match crate::platform::extension(path).as_str() {
            "gif" => Ok(Some(
                image::codecs::gif::GifDecoder::new(BufReader::new(File::open(path)?))?
                    .into_frames(),
            )),
            "webp" => {
                let decoder =
                    image::codecs::webp::WebPDecoder::new(BufReader::new(File::open(path)?))?;
                if decoder.has_animation() {
                    Ok(Some(decoder.into_frames()))
                } else {
                    Ok(None)
                }
            }
            _ => Ok(None),
        }
    }
}
