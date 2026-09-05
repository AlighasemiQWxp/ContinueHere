#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum Capability {
    UrlHandoff,
    PlaybackPositionHandoff,
    FileTransfer,
    LocalVideoHandoff,
}
