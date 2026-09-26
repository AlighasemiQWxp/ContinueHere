use std::{
    fs::{self, File, OpenOptions},
    io::{self, Write},
    path::Path,
};

pub(super) fn commit_file(source: &Path, destination: &Path) -> io::Result<()> {
    match fs::hard_link(source, destination) {
        Ok(()) => Ok(()),
        Err(error) if destination.exists() => Err(error),
        Err(_) => copy_file_no_clobber(source, destination),
    }
}

fn copy_file_no_clobber(source: &Path, destination: &Path) -> io::Result<()> {
    let mut source = File::open(source)?;
    let mut destination_file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(destination)?;
    let result = (|| {
        io::copy(&mut source, &mut destination_file)?;
        destination_file.flush()?;
        destination_file.sync_all()
    })();
    if result.is_err() {
        drop(destination_file);
        let _ = fs::remove_file(destination);
    }
    result
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::tempdir;

    use super::copy_file_no_clobber;

    #[test]
    fn portable_commit_copies_without_replacing_a_destination() {
        let directory = tempdir().unwrap();
        let source = directory.path().join("source.bin");
        let destination = directory.path().join("destination.bin");
        fs::write(&source, b"received").unwrap();

        copy_file_no_clobber(&source, &destination).unwrap();
        assert_eq!(fs::read(&destination).unwrap(), b"received");
        assert!(copy_file_no_clobber(&source, &destination).is_err());
        assert_eq!(fs::read(&destination).unwrap(), b"received");
    }
}
