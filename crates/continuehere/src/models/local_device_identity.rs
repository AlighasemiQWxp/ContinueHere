use super::{DeviceId, Platform};

use thiserror::Error;

pub(crate) const MAX_DISPLAY_NAME_SIZE: usize = 128;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub(crate) enum LocalDeviceIdentityError {
    #[error("device display name cannot be empty")]
    InvalidDisplayName,

    #[error("device display name exceeds the {maximum} byte limit")]
    DisplayNameTooLong { maximum: usize },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalDeviceIdentity {
    id: DeviceId,
    display_name: String,
    platform: Platform,
}

impl LocalDeviceIdentity {
    pub(crate) fn new(
        id: DeviceId,
        display_name: impl Into<String>,
        platform: Platform,
    ) -> Result<Self, LocalDeviceIdentityError> {
        let display_name = normalize_display_name(display_name.into())?;
        Ok(Self {
            id,
            display_name,
            platform,
        })
    }

    pub fn id(&self) -> &DeviceId {
        &self.id
    }

    pub fn display_name(&self) -> &str {
        &self.display_name
    }

    pub fn platform(&self) -> Platform {
        self.platform
    }

    pub(crate) fn renamed(
        &self,
        display_name: impl Into<String>,
    ) -> Result<Self, LocalDeviceIdentityError> {
        Self::new(self.id.clone(), display_name, self.platform)
    }
}

fn normalize_display_name(display_name: String) -> Result<String, LocalDeviceIdentityError> {
    let display_name = display_name.trim().to_owned();
    if display_name.is_empty() {
        return Err(LocalDeviceIdentityError::InvalidDisplayName);
    }
    if display_name.len() > MAX_DISPLAY_NAME_SIZE {
        return Err(LocalDeviceIdentityError::DisplayNameTooLong {
            maximum: MAX_DISPLAY_NAME_SIZE,
        });
    }
    Ok(display_name)
}

#[cfg(test)]
mod tests {
    use super::{LocalDeviceIdentity, MAX_DISPLAY_NAME_SIZE};
    use crate::models::{DeviceId, Platform};

    #[test]
    fn identity_normalizes_a_valid_display_name() {
        let identity = LocalDeviceIdentity::new(
            DeviceId::new("device-1").expect("identifier should be valid"),
            "  Desktop  ",
            Platform::Windows,
        )
        .expect("identity should be valid");

        assert_eq!(identity.display_name(), "Desktop");
    }

    #[test]
    fn identity_rejects_invalid_display_names() {
        let id = DeviceId::new("device-1").expect("identifier should be valid");

        assert!(LocalDeviceIdentity::new(id.clone(), "   ", Platform::Windows).is_err());
        assert!(
            LocalDeviceIdentity::new(id, "x".repeat(MAX_DISPLAY_NAME_SIZE + 1), Platform::Windows)
                .is_err()
        );
    }
}
