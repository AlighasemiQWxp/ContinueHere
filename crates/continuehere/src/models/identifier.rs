use std::{fmt, str::FromStr};

use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum DeviceIdError {
    #[error("device identifier cannot be empty")]
    Empty,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DeviceId(String);

impl DeviceId {
    pub fn new(value: impl Into<String>) -> Result<Self, DeviceIdError> {
        let value = value.into();
        if value.trim().is_empty() {
            return Err(DeviceIdError::Empty);
        }

        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for DeviceId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl FromStr for DeviceId {
    type Err = DeviceIdError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::new(value)
    }
}

impl TryFrom<String> for DeviceId {
    type Error = DeviceIdError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl TryFrom<&str> for DeviceId {
    type Error = DeviceIdError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::{DeviceId, DeviceIdError};

    #[test]
    fn rejects_empty_identifiers() {
        assert_eq!(DeviceId::new("   "), Err(DeviceIdError::Empty));
    }

    #[test]
    fn equal_identifiers_share_equality_and_hashing() {
        let first = DeviceId::new("device-1").expect("identifier should be valid");
        let second = DeviceId::new("device-1").expect("identifier should be valid");
        let mut identifiers = HashSet::new();

        identifiers.insert(first);

        assert!(identifiers.contains(&second));
        assert_eq!(second.as_str(), "device-1");
        assert_eq!(second.to_string(), "device-1");
    }
}
