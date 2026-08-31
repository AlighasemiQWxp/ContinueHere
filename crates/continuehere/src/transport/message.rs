use crate::models::{DeviceId, Platform, ProtocolVersion};

use super::TransportError;

const HELLO_KIND: u8 = 1;
const APPROVED_KIND: u8 = 2;
const REJECTED_KIND: u8 = 3;
const COMMITTED_KIND: u8 = 4;
const MAX_IDENTIFIER_SIZE: usize = 64;
const MAX_DISPLAY_NAME_SIZE: usize = 128;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PairingHello {
    device_id: DeviceId,
    display_name: String,
    platform: Platform,
    protocol_version: ProtocolVersion,
    nonce: [u8; 32],
}

impl PairingHello {
    pub(crate) fn new(
        device_id: DeviceId,
        display_name: String,
        platform: Platform,
        protocol_version: ProtocolVersion,
        nonce: [u8; 32],
    ) -> Result<Self, TransportError> {
        if device_id.as_str().is_empty()
            || device_id.as_str().len() > MAX_IDENTIFIER_SIZE
            || device_id.as_str().trim() != device_id.as_str()
        {
            return Err(TransportError::InvalidMessage);
        }
        let normalized_name = display_name.trim();
        if normalized_name.is_empty()
            || normalized_name.len() > MAX_DISPLAY_NAME_SIZE
            || normalized_name != display_name
        {
            return Err(TransportError::InvalidMessage);
        }
        Ok(Self {
            device_id,
            display_name,
            platform,
            protocol_version,
            nonce,
        })
    }

    pub(crate) fn device_id(&self) -> &DeviceId {
        &self.device_id
    }

    pub(crate) fn display_name(&self) -> &str {
        &self.display_name
    }

    pub(crate) const fn platform(&self) -> Platform {
        self.platform
    }

    pub(crate) const fn protocol_version(&self) -> ProtocolVersion {
        self.protocol_version
    }

    pub(crate) const fn nonce(&self) -> [u8; 32] {
        self.nonce
    }
}

pub(crate) enum PairingMessage {
    Hello(PairingHello),
    Approved,
    Rejected,
    Committed,
}

impl PairingMessage {
    pub(crate) fn encode(&self) -> Result<Vec<u8>, TransportError> {
        let mut bytes = Vec::new();
        match self {
            Self::Hello(hello) => {
                bytes.push(HELLO_KIND);
                write_text(&mut bytes, hello.device_id.as_str(), MAX_IDENTIFIER_SIZE)?;
                write_text(&mut bytes, &hello.display_name, MAX_DISPLAY_NAME_SIZE)?;
                bytes.push(encode_platform(hello.platform));
                bytes.extend_from_slice(&hello.protocol_version.major().to_be_bytes());
                bytes.extend_from_slice(&hello.protocol_version.minor().to_be_bytes());
                bytes.extend_from_slice(&hello.nonce);
            }
            Self::Approved => bytes.push(APPROVED_KIND),
            Self::Rejected => bytes.push(REJECTED_KIND),
            Self::Committed => bytes.push(COMMITTED_KIND),
        }
        Ok(bytes)
    }

    pub(crate) fn decode(bytes: &[u8]) -> Result<Self, TransportError> {
        let Some(kind) = bytes.first().copied() else {
            return Err(TransportError::InvalidMessage);
        };
        if kind == APPROVED_KIND || kind == REJECTED_KIND || kind == COMMITTED_KIND {
            if bytes.len() != 1 {
                return Err(TransportError::InvalidMessage);
            }
            return match kind {
                APPROVED_KIND => Ok(Self::Approved),
                REJECTED_KIND => Ok(Self::Rejected),
                COMMITTED_KIND => Ok(Self::Committed),
                _ => Err(TransportError::InvalidMessage),
            };
        }
        if kind != HELLO_KIND {
            return Err(TransportError::InvalidMessage);
        }

        let mut cursor = 1;
        let device_id = DeviceId::new(read_text(bytes, &mut cursor, MAX_IDENTIFIER_SIZE)?)
            .map_err(|_| TransportError::InvalidMessage)?;
        let display_name = read_text(bytes, &mut cursor, MAX_DISPLAY_NAME_SIZE)?;
        let platform = decode_platform(read_u8(bytes, &mut cursor)?)?;
        let major = read_u16(bytes, &mut cursor)?;
        let minor = read_u16(bytes, &mut cursor)?;
        let nonce_bytes = take(bytes, &mut cursor, 32)?;
        if cursor != bytes.len() {
            return Err(TransportError::InvalidMessage);
        }
        let mut nonce = [0_u8; 32];
        nonce.copy_from_slice(nonce_bytes);
        Ok(Self::Hello(PairingHello::new(
            device_id,
            display_name,
            platform,
            ProtocolVersion::new(major, minor),
            nonce,
        )?))
    }
}

fn write_text(bytes: &mut Vec<u8>, value: &str, maximum: usize) -> Result<(), TransportError> {
    if value.is_empty() || value.len() > maximum {
        return Err(TransportError::InvalidMessage);
    }
    let length = u16::try_from(value.len()).map_err(|_| TransportError::InvalidMessage)?;
    bytes.extend_from_slice(&length.to_be_bytes());
    bytes.extend_from_slice(value.as_bytes());
    Ok(())
}

fn read_text(bytes: &[u8], cursor: &mut usize, maximum: usize) -> Result<String, TransportError> {
    let length = usize::from(read_u16(bytes, cursor)?);
    if length == 0 || length > maximum {
        return Err(TransportError::InvalidMessage);
    }
    let value = std::str::from_utf8(take(bytes, cursor, length)?)
        .map_err(|_| TransportError::InvalidMessage)?;
    Ok(value.to_owned())
}

fn take<'a>(
    bytes: &'a [u8],
    cursor: &mut usize,
    length: usize,
) -> Result<&'a [u8], TransportError> {
    let end = cursor
        .checked_add(length)
        .ok_or(TransportError::InvalidMessage)?;
    let value = bytes
        .get(*cursor..end)
        .ok_or(TransportError::InvalidMessage)?;
    *cursor = end;
    Ok(value)
}

fn read_u8(bytes: &[u8], cursor: &mut usize) -> Result<u8, TransportError> {
    Ok(take(bytes, cursor, 1)?[0])
}

fn read_u16(bytes: &[u8], cursor: &mut usize) -> Result<u16, TransportError> {
    let value = take(bytes, cursor, 2)?;
    Ok(u16::from_be_bytes([value[0], value[1]]))
}

fn encode_platform(platform: Platform) -> u8 {
    match platform {
        Platform::Windows => 1,
        Platform::Linux => 2,
        Platform::MacOs => 3,
        Platform::Android => 4,
        Platform::Ios => 5,
        Platform::Unknown => 0,
    }
}

fn decode_platform(value: u8) -> Result<Platform, TransportError> {
    match value {
        0 => Ok(Platform::Unknown),
        1 => Ok(Platform::Windows),
        2 => Ok(Platform::Linux),
        3 => Ok(Platform::MacOs),
        4 => Ok(Platform::Android),
        5 => Ok(Platform::Ios),
        _ => Err(TransportError::InvalidMessage),
    }
}
