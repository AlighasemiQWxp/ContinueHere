#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum Platform {
    Windows,
    Linux,
    MacOs,
    Android,
    Ios,
    Unknown,
}

impl Platform {
    pub(crate) const fn current() -> Self {
        if cfg!(target_os = "windows") {
            return Self::Windows;
        }
        if cfg!(target_os = "linux") {
            return Self::Linux;
        }
        if cfg!(target_os = "macos") {
            return Self::MacOs;
        }
        if cfg!(target_os = "android") {
            return Self::Android;
        }
        if cfg!(target_os = "ios") {
            return Self::Ios;
        }
        Self::Unknown
    }

    pub(crate) const fn default_device_name(self) -> &'static str {
        match self {
            Self::Windows => "Windows Device",
            Self::Linux => "Linux Device",
            Self::MacOs => "Mac Device",
            Self::Android => "Android Device",
            Self::Ios => "iOS Device",
            Self::Unknown => "ContinueHere Device",
        }
    }
}
