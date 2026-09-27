mod image;

use image::ImagePreview;

use std::{
    cell::RefCell,
    path::{Path, PathBuf},
    rc::{Rc, Weak},
};

use slint::ComponentHandle;

use crate::platform::{
    self,
    media::{MediaCommand, MediaPlayer},
};

use super::{
    MainWindow,
    shared::{UiResult, show_result},
    transition::{UiTransition, UiTransitionController, UiTransitionHandle},
};

pub(super) struct PreviewUiController {
    player: Option<MediaPlayer>,
    generation: i32,
    animation: Option<ImagePreview>,
    timer: slint::Timer,
    transitions: Rc<UiTransitionController>,
    transition: Option<UiTransitionHandle>,
}

#[derive(Clone, Copy)]
enum PreviewAction {
    Close,
    Pause,
    Seek(f32),
    Volume(f32),
}

impl PreviewAction {
    fn parse(action: &str, value: f32) -> Option<Self> {
        match action {
            "close" => Some(Self::Close),
            "pause" => Some(Self::Pause),
            "seek" if value.is_finite() && value >= 0.0 => Some(Self::Seek(value)),
            "volume" if value.is_finite() => Some(Self::Volume(value)),
            _ => None,
        }
    }
}

#[derive(Clone)]
pub(super) struct PreviewOpener {
    controller: Weak<RefCell<PreviewUiController>>,
    window: slint::Weak<MainWindow>,
}

impl PreviewOpener {
    pub(super) fn open(&self, path: PathBuf, position: u64) -> UiResult {
        let Some(controller) = self.controller.upgrade() else {
            return Ok(());
        };
        let Some(window) = self.window.upgrade() else {
            return Ok(());
        };
        let result = controller.borrow_mut().open(&path, position, &window);
        if result.is_err() {
            controller.borrow_mut().close(&window);
        }
        controller.borrow().schedule_frame(&window);
        result
    }
}

impl PreviewUiController {
    pub(super) fn start(
        window: &MainWindow,
        transitions: Rc<UiTransitionController>,
    ) -> Rc<RefCell<Self>> {
        let controller = Rc::new(RefCell::new(Self {
            player: None,
            generation: 0,
            animation: None,
            timer: slint::Timer::default(),
            transitions,
            transition: None,
        }));
        let weak = Rc::downgrade(&controller);
        let view = window.as_weak();
        window.on_preview_action(move |action, value| {
            if let (Some(controller), Some(window)) = (weak.upgrade(), view.upgrade()) {
                let result = match PreviewAction::parse(action.as_str(), value) {
                    Some(action) => controller.borrow_mut().act(action, &window),
                    None => Err("Unknown playback action.".into()),
                };
                show_result(&window, result);
            }
        });
        let weak = Rc::downgrade(&controller);
        let view = window.as_weak();
        window.on_next_image_frame(move || {
            if let (Some(controller), Some(window)) = (weak.upgrade(), view.upgrade()) {
                controller.borrow_mut().advance_frame(&window);
            }
        });
        controller
    }

    pub(super) fn opener(controller: &Rc<RefCell<Self>>, window: &MainWindow) -> PreviewOpener {
        PreviewOpener {
            controller: Rc::downgrade(controller),
            window: window.as_weak(),
        }
    }

    fn open(&mut self, path: &Path, position: u64, window: &MainWindow) -> UiResult {
        if path.is_absolute() && path.is_dir() {
            return platform::open_file(path);
        }
        if platform::unsafe_file(path) {
            return Err("Executable and script files cannot be opened here.".into());
        }
        if !path.is_absolute() || !path.is_file() {
            return Err("The received file is unavailable.".into());
        }
        let extension = platform::extension(path);
        let image = matches!(
            extension.as_str(),
            "png" | "jpg" | "jpeg" | "gif" | "webp" | "bmp"
        );
        let video = matches!(
            extension.as_str(),
            "mp4" | "m4v" | "mkv" | "webm" | "mov" | "avi"
        );
        if !image && !video {
            return platform::open_file(path);
        }
        self.close(window);
        window.set_preview_path(path.to_string_lossy().as_ref().into());
        window.set_preview_title(
            path.file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .as_ref()
                .into(),
        );
        window.set_preview_error("".into());
        window.set_preview_video(video);
        window.set_preview_zoom(1.0);
        window.set_preview_position(0.0);
        window.set_preview_duration(0.0);
        window.set_preview_volume(1.0);
        window.set_preview_time("".into());
        if image {
            self.animation = ImagePreview::open(path)?;
            if let Some(animation) = &mut self.animation {
                let (image, delay) = animation.next()?;
                window.set_preview_image(image);
                window.set_image_frame_delay(delay.as_millis() as i32);
            } else {
                window.set_preview_image(slint::Image::load_from_path(path)?);
            }
        } else {
            self.player = Some(MediaPlayer::open(path, position, window, self.generation)?);
            window.set_preview_playing(true);
        }
        let transition = self.transitions.get_handle("preview");
        transition.configure(UiTransition::Preview)?;
        transition.use_handle()?;
        self.transition = Some(transition);
        window.set_preview_visible(true);
        Ok(())
    }

    fn act(&mut self, action: PreviewAction, window: &MainWindow) -> UiResult {
        if matches!(action, PreviewAction::Close) {
            self.close(window);
            return Ok(());
        }
        let player = self.player.as_ref().ok_or("No video is open.")?;
        let command = match action {
            PreviewAction::Close => return Ok(()),
            PreviewAction::Pause => MediaCommand::Pause(window.get_preview_playing()),
            PreviewAction::Seek(value) => MediaCommand::Seek((f64::from(value) * 1000.0) as u64),
            PreviewAction::Volume(value) => MediaCommand::Volume(f64::from(value)),
        };
        player.command(command)
    }

    fn close(&mut self, window: &MainWindow) {
        self.generation = self.generation.wrapping_add(1);
        window.set_preview_generation(self.generation);
        self.player.take();
        self.timer.stop();
        self.animation.take();
        self.transition.take();
        window.set_preview_visible(false);
        window.set_preview_playing(false);
        window.set_preview_image(slint::Image::default());
    }

    fn schedule_frame(&self, window: &MainWindow) {
        if self.animation.is_none() {
            return;
        }
        let view = window.as_weak();
        self.timer.start(
            slint::TimerMode::SingleShot,
            std::time::Duration::from_millis(window.get_image_frame_delay().max(10) as u64),
            move || {
                if let Some(window) = view.upgrade() {
                    window.invoke_next_image_frame();
                }
            },
        );
    }

    fn advance_frame(&mut self, window: &MainWindow) {
        let Some(animation) = &mut self.animation else {
            return;
        };
        match animation.next() {
            Ok((image, delay)) => {
                window.set_preview_image(image);
                window.set_image_frame_delay(delay.as_millis() as i32);
                self.schedule_frame(window);
            }
            Err(error) => {
                self.animation.take();
                window.set_preview_error(error.to_string().into());
            }
        }
    }
}
