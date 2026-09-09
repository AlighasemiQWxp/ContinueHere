use std::{
    collections::BTreeMap,
    fs,
    io::{self, Write},
    path::PathBuf,
};

use crate::{DeviceId, DiscoveryEndpoint};

#[derive(Default)]
pub(super) struct EndpointStore {
    path: Option<PathBuf>,
    entries: BTreeMap<String, DiscoveryEndpoint>,
}

impl EndpointStore {
    pub(super) fn persistent(directory: PathBuf) -> Self {
        Self {
            path: Some(directory.join("endpoints.bin")),
            entries: BTreeMap::new(),
        }
    }

    pub(super) fn load(&mut self) -> io::Result<()> {
        let Some(path) = &self.path else {
            return Ok(());
        };
        let metadata = match fs::metadata(path) {
            Ok(value) => value,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(()),
            Err(error) => return Err(error),
        };
        if metadata.len() > 128 * 1024 {
            return Err(invalid());
        }
        let bytes = fs::read(path)?;
        if bytes.len() > 128 * 1024 {
            return Err(invalid());
        }
        let mut reader = minicbor::Decoder::new(&bytes);
        if reader.array().map_err(|_| invalid())? != Some(2)
            || reader.u8().map_err(|_| invalid())? != 1
        {
            return Err(invalid());
        }
        let count = reader.array().map_err(|_| invalid())?.ok_or_else(invalid)?;
        if count > 128 {
            return Err(invalid());
        }
        let mut entries = BTreeMap::new();
        for _ in 0..count {
            if reader.array().map_err(|_| invalid())? != Some(2) {
                return Err(invalid());
            }
            let id = DeviceId::new(reader.str().map_err(|_| invalid())?.to_owned())
                .map_err(|_| invalid())?;
            let endpoint = reader
                .str()
                .map_err(|_| invalid())?
                .parse()
                .map_err(|_| invalid())?;
            if entries.insert(id.as_str().to_owned(), endpoint).is_some() {
                return Err(invalid());
            }
        }
        if reader.position() != bytes.len() {
            return Err(invalid());
        }
        self.entries = entries;
        Ok(())
    }

    pub(super) fn get(&self, id: &DeviceId) -> Option<DiscoveryEndpoint> {
        self.entries.get(id.as_str()).cloned()
    }

    pub(super) fn remember(
        &mut self,
        id: &DeviceId,
        endpoint: &DiscoveryEndpoint,
    ) -> io::Result<()> {
        let mut entries = self.entries.clone();
        if entries.get(id.as_str()) == Some(endpoint) {
            return Ok(());
        }
        if entries.len() >= 128 && !entries.contains_key(id.as_str()) {
            entries.pop_first();
        }
        entries.insert(id.as_str().to_owned(), endpoint.clone());
        if let Some(path) = &self.path {
            let mut writer = minicbor::Encoder::new(Vec::new());
            writer
                .array(2)
                .map_err(|_| invalid())?
                .u8(1)
                .map_err(|_| invalid())?
                .array(entries.len() as u64)
                .map_err(|_| invalid())?;
            for (id, endpoint) in &entries {
                writer
                    .array(2)
                    .map_err(|_| invalid())?
                    .str(id)
                    .map_err(|_| invalid())?
                    .str(&endpoint.to_string())
                    .map_err(|_| invalid())?;
            }
            let mut file = atomic_write_file::AtomicWriteFile::open(path)?;
            file.write_all(&writer.into_writer())?;
            file.commit()?;
        }
        self.entries = entries;
        Ok(())
    }
}

fn invalid() -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidData,
        "Invalid connection endpoint cache",
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn endpoint_hints_round_trip_and_malformed_cache_is_preserved() {
        let directory = tempfile::tempdir().unwrap();
        let id = DeviceId::new("peer").unwrap();
        let endpoint = DiscoveryEndpoint::new("192.168.1.20", 5300).unwrap();
        let mut store = EndpointStore::persistent(directory.path().to_path_buf());
        store.remember(&id, &endpoint).unwrap();
        let mut loaded = EndpointStore::persistent(directory.path().to_path_buf());
        loaded.load().unwrap();
        assert_eq!(loaded.get(&id), Some(endpoint));
        let path = directory.path().join("endpoints.bin");
        fs::write(&path, b"malformed").unwrap();
        assert!(loaded.load().is_err());
        assert_eq!(fs::read(path).unwrap(), b"malformed");
    }
}
