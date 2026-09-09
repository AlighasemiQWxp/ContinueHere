use std::sync::{Arc, Mutex};

use slint::{ComponentHandle, ModelRc, VecModel};

use super::MainWindow;

pub(super) type UiResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

#[derive(Clone)]
pub(super) struct EventTarget(Arc<Mutex<slint::Weak<MainWindow>>>);

impl EventTarget {
    pub(super) fn new(window: &MainWindow) -> Self {
        Self(Arc::new(Mutex::new(window.as_weak())))
    }

    pub(super) fn dispatch(&self, action: impl FnOnce(MainWindow) + Send + 'static) {
        let window = self
            .0
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .clone();
        let _ = window.upgrade_in_event_loop(action);
    }
}

pub(super) fn model<T: Clone + 'static>(rows: Vec<T>) -> ModelRc<T> {
    ModelRc::new(VecModel::from(rows))
}

pub(super) fn show_result<T, E: std::fmt::Display>(window: &MainWindow, result: Result<T, E>) {
    match result {
        Ok(_) => {}
        Err(error) => window.set_error_message(error.to_string().into()),
    }
}

pub(super) fn notify(window: &MainWindow, page: i32) {
    if window.get_page() == page {
        return;
    }
    match page {
        0 => window.set_devices_unread(true),
        1 => window.set_send_unread(true),
        2 => window.set_transfers_unread(true),
        3 => window.set_history_unread(true),
        _ => {}
    }
}

pub(super) fn text<'a>(rtl: bool, english: &'a str, persian: &'a str) -> &'a str {
    if rtl { persian } else { english }
}

pub(super) fn playback_position(value: &str) -> UiResult<std::time::Duration> {
    let value = value.trim();
    if value.is_empty() {
        return Ok(std::time::Duration::ZERO);
    }
    if value.split(':').count() > 3 {
        return Err("Use seconds, MM:SS, or HH:MM:SS.".into());
    }
    let mut seconds = 0_u64;
    for part in value.split(':') {
        seconds = seconds
            .checked_mul(60)
            .and_then(|total| {
                part.parse::<u64>()
                    .ok()
                    .and_then(|part| total.checked_add(part))
            })
            .ok_or("Enter a valid playback position.")?;
    }
    Ok(std::time::Duration::from_secs(seconds))
}

#[cfg(test)]
mod tests {
    use super::playback_position;

    #[test]
    fn playback_input_accepts_reference_formats_and_rejects_overflow() {
        assert_eq!(playback_position("").unwrap().as_secs(), 0);
        assert_eq!(playback_position("1:02:03").unwrap().as_secs(), 3723);
        for value in ["-1", "1::2", "1:2:3:4", "18446744073709551615:1"] {
            assert!(playback_position(value).is_err());
        }
    }
}
