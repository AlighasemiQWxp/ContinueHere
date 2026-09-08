use super::{Activity, ActivityDirection, ActivityError, ActivityKind, ActivityStatus};
use crate::models::Platform;
use atomic_write_file::AtomicWriteFile;
use std::{
    fs::{self, File},
    io::{Read, Write},
    path::PathBuf,
};

const MAGIC: &[u8] = b"CHHIST01";
const MAX_BYTES: usize = 16 * 1024 * 1024;
pub(super) const MAX_ENTRIES: usize = 1100;
const MAX_TEXT: usize = 16 * 1024;

pub(super) struct ActivityStore {
    path: PathBuf,
}
impl ActivityStore {
    pub(super) fn new(directory: PathBuf) -> Self {
        Self {
            path: directory.join("history.bin"),
        }
    }
    pub(super) fn load(&self) -> Result<Vec<Activity>, ActivityError> {
        let file = match File::open(&self.path) {
            Ok(file) => file,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(error) => return Err(error.into()),
        };
        let mut bytes = Vec::new();
        file.take((MAX_BYTES + 1) as u64).read_to_end(&mut bytes)?;
        decode(&bytes)
    }
    pub(super) fn save(&self, entries: &[Activity]) -> Result<(), ActivityError> {
        let bytes = encode(entries)?;
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut file = AtomicWriteFile::open(&self.path)?;
        file.write_all(&bytes)?;
        file.commit()?;
        Ok(())
    }
}

pub(super) fn encode(entries: &[Activity]) -> Result<Vec<u8>, ActivityError> {
    if entries.len() > MAX_ENTRIES {
        return Err(ActivityError::InvalidData);
    }
    let mut write = Writer(MAGIC.to_vec());
    write.u64(entries.len() as u64);
    for entry in entries {
        encode_entry(&mut write, entry)?;
    }
    if write.0.len() > MAX_BYTES {
        return Err(ActivityError::InvalidData);
    }
    Ok(write.0)
}
pub(super) fn decode(bytes: &[u8]) -> Result<Vec<Activity>, ActivityError> {
    if bytes.len() > MAX_BYTES {
        return Err(ActivityError::InvalidData);
    }
    let mut read = Reader(bytes);
    if read.take(MAGIC.len())? != MAGIC {
        return Err(ActivityError::InvalidData);
    }
    let count = read.u64()?;
    if count > MAX_ENTRIES as u64 {
        return Err(ActivityError::InvalidData);
    }
    let mut entries = Vec::with_capacity(count as usize);
    let mut identifiers = std::collections::HashSet::new();
    for _ in 0..count {
        let entry = decode_entry(&mut read)?;
        if entry.id.is_empty()
            || entry.device_id.trim().is_empty()
            || !identifiers.insert(entry.id.clone())
        {
            return Err(ActivityError::InvalidData);
        }
        if [
            Some(entry.started_at),
            entry.ended_at,
            entry.completed_at,
            entry.file_completed_at,
            entry.disconnected_at,
        ]
        .into_iter()
        .flatten()
        .any(|time| time > 253_402_300_799_999)
            || entry.path.as_ref().is_some_and(|path| !path.is_absolute())
        {
            return Err(ActivityError::InvalidData);
        }
        entries.push(entry);
    }
    if !read.0.is_empty() {
        return Err(ActivityError::InvalidData);
    }
    Ok(entries)
}
fn encode_entry(write: &mut Writer, value: &Activity) -> Result<(), ActivityError> {
    write.text(&value.id)?;
    write.text(&value.device_id)?;
    write.text(&value.device_name)?;
    write.u8(match value.platform {
        Platform::Windows => 0,
        Platform::Linux => 1,
        Platform::MacOs => 2,
        Platform::Android => 3,
        Platform::Ios => 4,
        Platform::Unknown => 5,
    });
    write.u8(match value.kind {
        ActivityKind::Url => 0,
        ActivityKind::YouTube => 1,
        ActivityKind::File => 2,
        ActivityKind::LocalVideo => 3,
        ActivityKind::Session => 4,
    });
    write.u8(match value.direction {
        ActivityDirection::Outgoing => 0,
        ActivityDirection::Incoming => 1,
        ActivityDirection::Connection => 2,
    });
    write.u8(match value.status {
        ActivityStatus::Active => 0,
        ActivityStatus::Delivered => 1,
        ActivityStatus::Completed => 2,
        ActivityStatus::Rejected => 3,
        ActivityStatus::Cancelled => 4,
        ActivityStatus::Failed => 5,
        ActivityStatus::Disconnected => 6,
        ActivityStatus::Interrupted => 7,
    });
    write.text(&value.title)?;
    write.u64(value.started_at);
    write.optional_number(value.ended_at);
    write.optional_number(value.completed_at);
    write.optional_number(value.disconnected_at);
    write.optional_number(value.file_completed_at);
    write.optional_text(value.failure.as_deref())?;
    write.optional_text(value.url.as_deref())?;
    write.optional_text(
        value
            .path
            .as_ref()
            .map(|path| path.to_str().ok_or(ActivityError::InvalidData))
            .transpose()?,
    )?;
    write.u64(value.position_millis);
    write.optional_text(value.retry_of.as_deref())?;
    write.optional_text(value.session_id.as_deref())?;
    write.optional_text(value.transfer_id.as_deref())?;
    write.u64(value.revision);
    write.optional_number(value.source_size);
    write.optional_number(value.source_modified);
    Ok(())
}
fn decode_entry(read: &mut Reader<'_>) -> Result<Activity, ActivityError> {
    Ok(Activity {
        id: read.text()?,
        device_id: read.text()?,
        device_name: read.text()?,
        platform: match read.u8()? {
            0 => Platform::Windows,
            1 => Platform::Linux,
            2 => Platform::MacOs,
            3 => Platform::Android,
            4 => Platform::Ios,
            5 => Platform::Unknown,
            _ => return Err(ActivityError::InvalidData),
        },
        kind: match read.u8()? {
            0 => ActivityKind::Url,
            1 => ActivityKind::YouTube,
            2 => ActivityKind::File,
            3 => ActivityKind::LocalVideo,
            4 => ActivityKind::Session,
            _ => return Err(ActivityError::InvalidData),
        },
        direction: match read.u8()? {
            0 => ActivityDirection::Outgoing,
            1 => ActivityDirection::Incoming,
            2 => ActivityDirection::Connection,
            _ => return Err(ActivityError::InvalidData),
        },
        status: match read.u8()? {
            0 => ActivityStatus::Active,
            1 => ActivityStatus::Delivered,
            2 => ActivityStatus::Completed,
            3 => ActivityStatus::Rejected,
            4 => ActivityStatus::Cancelled,
            5 => ActivityStatus::Failed,
            6 => ActivityStatus::Disconnected,
            7 => ActivityStatus::Interrupted,
            _ => return Err(ActivityError::InvalidData),
        },
        title: read.text()?,
        started_at: read.u64()?,
        ended_at: read.optional_number()?,
        completed_at: read.optional_number()?,
        disconnected_at: read.optional_number()?,
        file_completed_at: read.optional_number()?,
        failure: read.optional_text()?,
        url: read.optional_text()?,
        path: read.optional_text()?.map(PathBuf::from),
        position_millis: read.u64()?,
        retry_of: read.optional_text()?,
        session_id: read.optional_text()?,
        transfer_id: read.optional_text()?,
        revision: read.u64()?,
        source_size: read.optional_number()?,
        source_modified: read.optional_number()?,
    })
}
struct Writer(Vec<u8>);
impl Writer {
    fn u8(&mut self, value: u8) {
        self.0.push(value);
    }
    fn u64(&mut self, value: u64) {
        self.0.extend_from_slice(&value.to_le_bytes());
    }
    fn text(&mut self, value: &str) -> Result<(), ActivityError> {
        if value.len() > MAX_TEXT {
            return Err(ActivityError::InvalidData);
        }
        self.u64(value.len() as u64);
        self.0.extend_from_slice(value.as_bytes());
        Ok(())
    }
    fn optional_text(&mut self, value: Option<&str>) -> Result<(), ActivityError> {
        match value {
            Some(value) => {
                self.u8(1);
                self.text(value)?;
            }
            None => self.u8(0),
        }
        Ok(())
    }
    fn optional_number(&mut self, value: Option<u64>) {
        match value {
            Some(value) => {
                self.u8(1);
                self.u64(value);
            }
            None => self.u8(0),
        }
    }
}
struct Reader<'a>(&'a [u8]);
impl<'a> Reader<'a> {
    fn take(&mut self, count: usize) -> Result<&'a [u8], ActivityError> {
        if count > self.0.len() {
            return Err(ActivityError::InvalidData);
        }
        let (value, remaining) = self.0.split_at(count);
        self.0 = remaining;
        Ok(value)
    }
    fn u8(&mut self) -> Result<u8, ActivityError> {
        Ok(self.take(1)?[0])
    }
    fn u64(&mut self) -> Result<u64, ActivityError> {
        Ok(u64::from_le_bytes(
            self.take(8)?
                .try_into()
                .map_err(|_| ActivityError::InvalidData)?,
        ))
    }
    fn text(&mut self) -> Result<String, ActivityError> {
        let count = self.u64()?;
        if count > MAX_TEXT as u64 {
            return Err(ActivityError::InvalidData);
        }
        Ok(std::str::from_utf8(self.take(count as usize)?)
            .map_err(|_| ActivityError::InvalidData)?
            .to_owned())
    }
    fn optional_text(&mut self) -> Result<Option<String>, ActivityError> {
        match self.u8()? {
            0 => Ok(None),
            1 => Ok(Some(self.text()?)),
            _ => Err(ActivityError::InvalidData),
        }
    }
    fn optional_number(&mut self) -> Result<Option<u64>, ActivityError> {
        match self.u8()? {
            0 => Ok(None),
            1 => Ok(Some(self.u64()?)),
            _ => Err(ActivityError::InvalidData),
        }
    }
}
