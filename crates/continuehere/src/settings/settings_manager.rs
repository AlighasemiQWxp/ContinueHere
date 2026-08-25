pub struct SettingsManager {
    _private: (),
}

impl SettingsManager {
    pub(crate) fn new() -> Self {
        Self { _private: () }
    }
}
