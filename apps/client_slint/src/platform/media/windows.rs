use std::{
    path::Path,
    sync::{Arc, Mutex, mpsc},
    thread,
    time::Duration,
};

use gstreamer::{self as gst, prelude::*};
use gstreamer_video::prelude::*;
use slint::{ComponentHandle, Rgba8Pixel, SharedPixelBuffer};

use super::MediaCommand;

type Frame = SharedPixelBuffer<Rgba8Pixel>;
type WindowTarget = Arc<Mutex<slint::Weak<crate::ui::MainWindow>>>;

pub(super) struct WindowsPlayer {
    commands: Option<mpsc::Sender<MediaCommand>>,
    worker: Option<thread::JoinHandle<()>>,
}

impl WindowsPlayer {
    pub(super) fn open(
        path: &Path,
        position: u64,
        window: &crate::ui::MainWindow,
        generation: i32,
    ) -> crate::platform::PlatformResult<Self> {
        gst::init()?;
        let uri = url::Url::from_file_path(path)
            .map_err(|_| "Invalid media path.")?
            .to_string();
        let target = Arc::new(Mutex::new(window.as_weak()));
        let (sender, receiver) = mpsc::channel();
        let worker = thread::Builder::new()
            .name("media-playback".into())
            .spawn(move || {
                if let Err(error) = play(&uri, position, &target, generation, receiver) {
                    dispatch(&target, move |window| {
                        if window.get_preview_generation() == generation {
                            window.set_preview_error(error.into());
                            window.set_preview_playing(false);
                        }
                    });
                }
            })?;
        Ok(Self {
            commands: Some(sender),
            worker: Some(worker),
        })
    }

    pub(super) fn command(&self, command: MediaCommand) -> crate::platform::PlatformResult<()> {
        self.commands
            .as_ref()
            .ok_or("The player is closed.")?
            .send(command)
            .map_err(|_| "The player is unavailable.")?;
        Ok(())
    }
}

impl Drop for WindowsPlayer {
    fn drop(&mut self) {
        self.commands.take();
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

struct Pipeline(gst::Element);

impl Drop for Pipeline {
    fn drop(&mut self) {
        let _ = self.0.set_state(gst::State::Null);
    }
}

fn play(
    uri: &str,
    position: u64,
    target: &WindowTarget,
    generation: i32,
    commands: mpsc::Receiver<MediaCommand>,
) -> Result<(), String> {
    let caps = gst::Caps::builder("video/x-raw")
        .field("format", "RGBA")
        .field("width", gst::IntRange::<i32>::new(1, 1920))
        .field("height", gst::IntRange::<i32>::new(1, 1080))
        .field("pixel-aspect-ratio", gst::Fraction::new(1, 1))
        .build();
    let sink = gstreamer_app::AppSink::builder()
        .caps(&caps)
        .max_buffers(1)
        .drop(true)
        .sync(true)
        .build();
    let frames: Arc<Mutex<Option<Frame>>> = Arc::new(Mutex::new(None));
    let pending = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let frame_target = Arc::clone(target);
    let preroll_target = Arc::clone(target);
    let sample_frames = Arc::clone(&frames);
    let sample_pending = Arc::clone(&pending);
    sink.set_callbacks(
        gstreamer_app::AppSinkCallbacks::builder()
            .new_sample(move |sink| {
                let sample = sink.pull_sample().map_err(|_| gst::FlowError::Eos)?;
                present_sample(
                    &sample,
                    &sample_frames,
                    &sample_pending,
                    &frame_target,
                    generation,
                )
            })
            .new_preroll(move |sink| {
                let sample = sink.pull_preroll().map_err(|_| gst::FlowError::Eos)?;
                present_sample(&sample, &frames, &pending, &preroll_target, generation)
            })
            .build(),
    );
    let pipeline = Pipeline(
        gst::ElementFactory::make("playbin")
            .property("uri", uri)
            .property("video-sink", &sink)
            .build()
            .map_err(|error| error.to_string())?,
    );
    let bus = pipeline
        .0
        .bus()
        .ok_or("The media event bus is unavailable.")?;
    pipeline
        .0
        .set_state(gst::State::Paused)
        .map_err(|error| error.to_string())?;
    let mut initial_position = Some(position);
    let mut playing = true;
    let mut ended = false;
    loop {
        loop {
            match commands.try_recv() {
                Ok(MediaCommand::Pause(paused)) => {
                    playing = !paused;
                    if initial_position.is_some() {
                        continue;
                    }
                    if ended && playing {
                        pipeline
                            .0
                            .seek_simple(
                                gst::SeekFlags::FLUSH | gst::SeekFlags::ACCURATE,
                                gst::ClockTime::ZERO,
                            )
                            .map_err(|error| error.to_string())?;
                        ended = false;
                    }
                    let state = if paused {
                        gst::State::Paused
                    } else {
                        gst::State::Playing
                    };
                    pipeline
                        .0
                        .set_state(state)
                        .map_err(|error| error.to_string())?;
                }
                Ok(MediaCommand::Seek(value)) => {
                    ended = false;
                    pipeline
                        .0
                        .seek_simple(
                            gst::SeekFlags::FLUSH | gst::SeekFlags::ACCURATE,
                            gst::ClockTime::from_mseconds(value),
                        )
                        .map_err(|error| error.to_string())?;
                }
                Ok(MediaCommand::Volume(value)) => {
                    pipeline.0.set_property("volume", value.clamp(0.0, 1.0))
                }
                Err(mpsc::TryRecvError::Disconnected) => return Ok(()),
                Err(mpsc::TryRecvError::Empty) => break,
            }
        }
        if let Some(message) = bus.timed_pop(gst::ClockTime::from_mseconds(100)) {
            match message.view() {
                gst::MessageView::Error(error) => return Err(error.error().to_string()),
                gst::MessageView::Eos(_) => {
                    ended = true;
                    playing = false;
                    pipeline
                        .0
                        .set_state(gst::State::Paused)
                        .map_err(|error| error.to_string())?;
                }
                gst::MessageView::AsyncDone(_) => {
                    if let Some(position) = initial_position.take() {
                        if position > 0 {
                            pipeline
                                .0
                                .seek_simple(
                                    gst::SeekFlags::FLUSH | gst::SeekFlags::ACCURATE,
                                    gst::ClockTime::from_mseconds(position),
                                )
                                .map_err(|error| error.to_string())?;
                        }
                        let state = if playing {
                            gst::State::Playing
                        } else {
                            gst::State::Paused
                        };
                        pipeline
                            .0
                            .set_state(state)
                            .map_err(|error| error.to_string())?;
                    }
                }
                _ => {}
            }
        }
        let position = pipeline
            .0
            .query_position::<gst::ClockTime>()
            .map(|value| value.mseconds())
            .unwrap_or(0);
        let duration = pipeline
            .0
            .query_duration::<gst::ClockTime>()
            .map(|value| value.mseconds())
            .unwrap_or(0);
        dispatch(target, move |window| {
            if window.get_preview_generation() == generation {
                window.set_preview_position(position as f32 / 1000.0);
                window.set_preview_duration(duration as f32 / 1000.0);
                window
                    .set_preview_time(format!("{} / {}", clock(position), clock(duration)).into());
                window.set_preview_playing(playing);
            }
        });
        thread::sleep(Duration::from_millis(10));
    }
}

fn clock(milliseconds: u64) -> String {
    let seconds = milliseconds / 1000;
    format!(
        "{}:{:02}:{:02}",
        seconds / 3600,
        seconds / 60 % 60,
        seconds % 60
    )
}

fn present_sample(
    sample: &gst::Sample,
    frames: &Arc<Mutex<Option<Frame>>>,
    pending: &Arc<std::sync::atomic::AtomicBool>,
    target: &WindowTarget,
    generation: i32,
) -> Result<gst::FlowSuccess, gst::FlowError> {
    let info = gstreamer_video::VideoInfo::from_caps(sample.caps().ok_or(gst::FlowError::Error)?)
        .map_err(|_| gst::FlowError::Error)?;
    let frame = gstreamer_video::VideoFrameRef::from_buffer_ref_readable(
        sample.buffer().ok_or(gst::FlowError::Error)?,
        &info,
    )
    .map_err(|_| gst::FlowError::Error)?;
    let data = frame.plane_data(0).map_err(|_| gst::FlowError::Error)?;
    let stride = usize::try_from(frame.plane_stride()[0]).map_err(|_| gst::FlowError::Error)?;
    let row_bytes = info.width() as usize * 4;
    let mut pixels = Frame::new(info.width(), info.height());
    for (row, destination) in pixels
        .make_mut_bytes()
        .chunks_exact_mut(row_bytes)
        .enumerate()
    {
        let start = row * stride;
        destination.copy_from_slice(
            data.get(start..start + row_bytes)
                .ok_or(gst::FlowError::Error)?,
        );
    }
    *frames.lock().unwrap_or_else(|error| error.into_inner()) = Some(pixels);
    if !pending.swap(true, std::sync::atomic::Ordering::AcqRel) {
        let frames = Arc::clone(frames);
        let pending = Arc::clone(pending);
        dispatch(target, move |window| {
            let frame = frames
                .lock()
                .unwrap_or_else(|error| error.into_inner())
                .take();
            pending.store(false, std::sync::atomic::Ordering::Release);
            if window.get_preview_generation() == generation
                && let Some(frame) = frame
            {
                window.set_preview_image(slint::Image::from_rgba8(frame));
            }
        });
    }
    Ok(gst::FlowSuccess::Ok)
}

fn dispatch(target: &WindowTarget, action: impl FnOnce(crate::ui::MainWindow) + Send + 'static) {
    let window = target
        .lock()
        .unwrap_or_else(|error| error.into_inner())
        .clone();
    let _ = window.upgrade_in_event_loop(action);
}
