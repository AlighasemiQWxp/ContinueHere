use std::{
    collections::BTreeMap,
    fs::{self, File, OpenOptions},
    io::{self, Read, Seek, SeekFrom, Write},
    path::Path,
};

use super::model::{MAX_FILE_SIZE, TRANSFER_CHUNK_SIZE, validate_file_name};

const MAGIC: &[u8; 8] = b"CHFOLD01";
const MAX_ENTRIES: usize = 4096;
const MAX_PATH_BYTES: usize = 1024;
const MAX_DEPTH: usize = 32;

struct Entry {
    path: String,
    directory: bool,
    size: u64,
    offset: u64,
}

pub(super) fn validate_root(path: &Path) -> io::Result<()> {
    let metadata = fs::symlink_metadata(path)?;
    if !path.is_absolute() || !metadata.is_dir() || linked(&metadata) {
        return Err(invalid());
    }
    Ok(())
}

pub(super) fn pack(
    source: &Path,
    cancelled: impl Fn() -> bool,
) -> io::Result<tempfile::NamedTempFile> {
    check_cancel(&cancelled)?;
    validate_root(source)?;
    let mut entries = Vec::new();
    collect(source, source, &mut entries, &cancelled, 0)?;
    let mut output = tempfile::NamedTempFile::new()?;
    output.write_all(MAGIC)?;
    output.write_all(&(entries.len() as u32).to_le_bytes())?;
    let mut total = 12_u64;
    for entry in entries {
        check_cancel(&cancelled)?;
        total = total
            .checked_add(11 + entry.path.len() as u64)
            .and_then(|value| value.checked_add(entry.size))
            .filter(|value| *value <= MAX_FILE_SIZE)
            .ok_or_else(invalid)?;
        output.write_all(&[u8::from(entry.directory)])?;
        output.write_all(&(entry.path.len() as u16).to_le_bytes())?;
        output.write_all(&entry.size.to_le_bytes())?;
        output.write_all(entry.path.as_bytes())?;
        if !entry.directory {
            let path = source.join(&entry.path);
            let metadata = fs::symlink_metadata(&path)?;
            if !metadata.is_file() || linked(&metadata) || metadata.len() != entry.size {
                return Err(invalid());
            }
            let mut file = File::open(path)?;
            copy_exact(&mut file, &mut output, entry.size, &cancelled)?;
            if file.metadata()?.len() != entry.size {
                return Err(invalid());
            }
        }
    }
    output.flush()?;
    output.seek(SeekFrom::Start(0))?;
    Ok(output)
}

fn collect(
    root: &Path,
    directory: &Path,
    entries: &mut Vec<Entry>,
    cancelled: &impl Fn() -> bool,
    depth: usize,
) -> io::Result<()> {
    if depth > MAX_DEPTH {
        return Err(invalid());
    }
    // Enumerate incrementally so a directory with millions of entries stays bounded.
    let mut children = Vec::new();
    for child in fs::read_dir(directory)? {
        check_cancel(cancelled)?;
        if children.len() + entries.len() >= MAX_ENTRIES {
            return Err(invalid());
        }
        children.push(child?.path());
    }
    children.sort();
    for path in children {
        check_cancel(cancelled)?;
        if entries.len() >= MAX_ENTRIES {
            return Err(invalid());
        }
        let metadata = fs::symlink_metadata(&path)?;
        if linked(&metadata) || !(metadata.is_dir() || metadata.is_file()) {
            return Err(invalid());
        }
        let relative = path
            .strip_prefix(root)
            .map_err(|_| invalid())?
            .to_str()
            .ok_or_else(invalid)?
            .replace('\\', "/");
        validate_path(&relative)?;
        let size = if metadata.is_dir() { 0 } else { metadata.len() };
        if size > MAX_FILE_SIZE {
            return Err(invalid());
        }
        entries.push(Entry {
            path: relative,
            directory: metadata.is_dir(),
            size,
            offset: 0,
        });
        if metadata.is_dir() {
            collect(root, &path, entries, cancelled, depth + 1)?;
        }
    }
    Ok(())
}

pub(super) fn unpack(
    source: &Path,
    destination: &Path,
    cancelled: impl Fn() -> bool,
) -> io::Result<()> {
    let mut source = File::open(source)?;
    let entries = inspect(&mut source)?;
    check_cancel(&cancelled)?;
    let parent = destination.parent().ok_or_else(invalid)?;
    let staging = tempfile::Builder::new()
        .prefix(".continuehere-folder-")
        .tempdir_in(parent)?;
    for entry in &entries {
        check_cancel(&cancelled)?;
        let path = staging.path().join(&entry.path);
        if entry.directory {
            fs::create_dir(&path)?;
        } else {
            let mut file = OpenOptions::new().write(true).create_new(true).open(path)?;
            source.seek(SeekFrom::Start(entry.offset))?;
            copy_exact(&mut source, &mut file, entry.size, &cancelled)?;
            file.sync_all()?;
        }
    }
    check_cancel(&cancelled)?;
    // Reserve the final name without replacing an existing directory on any platform.
    fs::create_dir(destination)?;
    let result = (|| {
        for entry in &entries {
            check_cancel(&cancelled)?;
            let path = destination.join(&entry.path);
            if entry.directory {
                fs::create_dir(&path)?;
            } else {
                fs::hard_link(staging.path().join(&entry.path), path)?;
            }
        }
        check_cancel(&cancelled)
    })();
    if result.is_err() {
        let _ = fs::remove_dir_all(destination);
    }
    result
}

fn inspect(file: &mut File) -> io::Result<Vec<Entry>> {
    let length = file.metadata()?.len();
    if length > MAX_FILE_SIZE {
        return Err(invalid());
    }
    let mut magic = [0; 8];
    file.read_exact(&mut magic)?;
    if &magic != MAGIC {
        return Err(invalid());
    }
    let mut count = [0; 4];
    file.read_exact(&mut count)?;
    let count = u32::from_le_bytes(count) as usize;
    if count > MAX_ENTRIES {
        return Err(invalid());
    }
    let mut paths = BTreeMap::new();
    let mut entries = Vec::with_capacity(count);
    for _ in 0..count {
        let mut header = [0; 11];
        file.read_exact(&mut header)?;
        let directory = match header[0] {
            0 => false,
            1 => true,
            _ => return Err(invalid()),
        };
        let name_length = u16::from_le_bytes([header[1], header[2]]) as usize;
        if name_length == 0 || name_length > MAX_PATH_BYTES {
            return Err(invalid());
        }
        let size = u64::from_le_bytes(header[3..11].try_into().map_err(|_| invalid())?);
        if size > MAX_FILE_SIZE || (directory && size != 0) {
            return Err(invalid());
        }
        let mut name = vec![0; name_length];
        file.read_exact(&mut name)?;
        let path = String::from_utf8(name).map_err(|_| invalid())?;
        validate_path(&path)?;
        let normalized = path.to_lowercase();
        if let Some((parent, _)) = normalized.rsplit_once('/')
            && paths.get(parent) != Some(&true)
        {
            return Err(invalid());
        }
        if paths.insert(normalized, directory).is_some() {
            return Err(invalid());
        }
        let offset = file.stream_position()?;
        let end = offset
            .checked_add(size)
            .filter(|end| *end <= length)
            .ok_or_else(invalid)?;
        file.seek(SeekFrom::Start(end))?;
        entries.push(Entry {
            path,
            directory,
            size,
            offset,
        });
    }
    if file.stream_position()? != length {
        return Err(invalid());
    }
    Ok(entries)
}

fn validate_path(path: &str) -> io::Result<()> {
    if path.len() > MAX_PATH_BYTES || path.contains('\\') || path.split('/').count() > MAX_DEPTH {
        return Err(invalid());
    }
    for part in path.split('/') {
        validate_file_name(part).map_err(|_| invalid())?;
    }
    Ok(())
}

fn linked(metadata: &fs::Metadata) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        if metadata.file_attributes() & 0x400 != 0 {
            return true;
        }
    }
    metadata.file_type().is_symlink()
}

fn copy_exact(
    reader: &mut impl Read,
    writer: &mut impl Write,
    mut remaining: u64,
    cancelled: &impl Fn() -> bool,
) -> io::Result<()> {
    let mut buffer = [0; TRANSFER_CHUNK_SIZE];
    while remaining > 0 {
        check_cancel(cancelled)?;
        let count = remaining.min(buffer.len() as u64) as usize;
        reader.read_exact(&mut buffer[..count])?;
        writer.write_all(&buffer[..count])?;
        remaining -= count as u64;
    }
    Ok(())
}

fn check_cancel(cancelled: &impl Fn() -> bool) -> io::Result<()> {
    if cancelled() {
        return Err(io::Error::new(
            io::ErrorKind::Interrupted,
            "Folder transfer cancelled",
        ));
    }
    Ok(())
}

fn invalid() -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidData,
        "Invalid folder transfer or unsupported folder contents",
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nested_files_empty_directories_and_conflicts() {
        let source = tempfile::tempdir().unwrap();
        fs::create_dir(source.path().join("empty")).unwrap();
        fs::create_dir(source.path().join("nested")).unwrap();
        fs::write(source.path().join("nested/فیلم.txt"), b"payload").unwrap();
        let packed = pack(source.path(), || false).unwrap();
        let target = tempfile::tempdir().unwrap();
        let destination = target.path().join("received");
        unpack(packed.path(), &destination, || false).unwrap();
        assert!(destination.join("empty").is_dir());
        assert_eq!(
            fs::read(destination.join("nested/فیلم.txt")).unwrap(),
            b"payload"
        );
        assert!(unpack(packed.path(), &destination, || false).is_err());
        assert_eq!(
            fs::read(destination.join("nested/فیلم.txt")).unwrap(),
            b"payload"
        );
    }

    #[test]
    fn invalid_paths_and_cancelled_work_never_create_destination() {
        for path in [
            "../escape",
            "/absolute",
            "C:/file",
            "folder\\file",
            "a//b",
            "NUL",
            "a/../b",
        ] {
            assert!(validate_path(path).is_err(), "{path}");
        }
        let source = tempfile::tempdir().unwrap();
        assert!(pack(source.path(), || true).is_err());
        let packed = pack(source.path(), || false).unwrap();
        let destination = source.path().join("cancelled");
        assert!(unpack(packed.path(), &destination, || true).is_err());
        assert!(!destination.exists());
    }

    #[test]
    fn malformed_and_duplicate_entries_are_rejected_before_extraction() {
        for names in [vec!["../escape"], vec!["a", "A"], vec!["missing/child"]] {
            let mut file = tempfile::NamedTempFile::new().unwrap();
            file.write_all(MAGIC).unwrap();
            file.write_all(&(names.len() as u32).to_le_bytes()).unwrap();
            for name in names {
                file.write_all(&[0]).unwrap();
                file.write_all(&(name.len() as u16).to_le_bytes()).unwrap();
                file.write_all(&0_u64.to_le_bytes()).unwrap();
                file.write_all(name.as_bytes()).unwrap();
            }
            let directory = tempfile::tempdir().unwrap();
            let destination = directory.path().join("result");
            assert!(unpack(file.path(), &destination, || false).is_err());
            assert!(!destination.exists());
        }
    }
}
