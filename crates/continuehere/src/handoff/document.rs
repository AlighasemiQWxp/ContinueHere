use std::path::{Path, PathBuf};

use crate::transfer::{FileTransfer, FileTransferId};

use super::HandoffError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DocumentContinuation {
    PdfPage(u32),
    PowerPointSlide(u32),
}

impl DocumentContinuation {
    pub fn pdf_page(page: u32) -> Result<Self, HandoffError> {
        if page == 0 {
            return Err(HandoffError::InvalidDocumentContinuation);
        }
        Ok(Self::PdfPage(page))
    }

    pub fn powerpoint_slide(slide: u32) -> Result<Self, HandoffError> {
        if slide == 0 {
            return Err(HandoffError::InvalidDocumentContinuation);
        }
        Ok(Self::PowerPointSlide(slide))
    }

    pub const fn position(self) -> u32 {
        match self {
            Self::PdfPage(page) => page,
            Self::PowerPointSlide(slide) => slide,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalDocumentHandoff {
    file_path: PathBuf,
    continuation: DocumentContinuation,
    transfer_id: Option<FileTransferId>,
}

impl LocalDocumentHandoff {
    pub(crate) fn new(
        file_path: PathBuf,
        continuation: DocumentContinuation,
    ) -> Result<Self, HandoffError> {
        validate_document_path(&file_path, continuation)?;
        Ok(Self {
            file_path,
            continuation,
            transfer_id: None,
        })
    }

    pub fn file_path(&self) -> &Path {
        &self.file_path
    }

    pub const fn continuation(&self) -> DocumentContinuation {
        self.continuation
    }

    pub fn transfer_id(&self) -> Option<&FileTransferId> {
        self.transfer_id.as_ref()
    }

    pub(crate) fn received(
        transfer: &FileTransfer,
        continuation: DocumentContinuation,
    ) -> Result<Self, HandoffError> {
        let path = transfer
            .destination()
            .ok_or(HandoffError::InvalidLocalDocument)?;
        validate_document_path(path, continuation)?;
        Ok(Self {
            file_path: path.to_path_buf(),
            continuation,
            transfer_id: Some(transfer.id().clone()),
        })
    }
}

fn validate_document_path(
    path: &Path,
    continuation: DocumentContinuation,
) -> Result<(), HandoffError> {
    if continuation.position() == 0 {
        return Err(HandoffError::InvalidDocumentContinuation);
    }
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .ok_or(HandoffError::InvalidLocalDocument)?
        .to_ascii_lowercase();
    let supported = match continuation {
        DocumentContinuation::PdfPage(_) => extension == "pdf",
        DocumentContinuation::PowerPointSlide(_) => matches!(extension.as_str(), "ppt" | "pptx"),
    };
    if !path.is_absolute() || !supported {
        return Err(HandoffError::InvalidLocalDocument);
    }
    let metadata = path
        .symlink_metadata()
        .map_err(|_| HandoffError::InvalidLocalDocument)?;
    if !metadata.file_type().is_file() || metadata.len() == 0 {
        return Err(HandoffError::InvalidLocalDocument);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::tempdir;

    use super::{DocumentContinuation, LocalDocumentHandoff};

    #[test]
    fn document_type_must_match_a_non_empty_regular_file() {
        let directory = tempdir().expect("temporary directory should exist");
        let pdf = directory.path().join("guide.pdf");
        fs::write(&pdf, b"pdf").expect("PDF should exist");
        let slide = directory.path().join("deck.pptx");
        fs::write(&slide, b"pptx").expect("presentation should exist");

        assert!(
            LocalDocumentHandoff::new(
                pdf.clone(),
                DocumentContinuation::pdf_page(17).expect("page should be valid"),
            )
            .is_ok()
        );
        assert!(
            LocalDocumentHandoff::new(
                slide,
                DocumentContinuation::powerpoint_slide(4).expect("slide should be valid"),
            )
            .is_ok()
        );
        assert!(
            LocalDocumentHandoff::new(
                pdf,
                DocumentContinuation::powerpoint_slide(4).expect("slide should be valid"),
            )
            .is_err()
        );
    }

    #[test]
    fn document_continuation_is_one_based() {
        assert!(DocumentContinuation::pdf_page(0).is_err());
        assert!(DocumentContinuation::powerpoint_slide(0).is_err());
        assert!(
            LocalDocumentHandoff::new(
                std::path::PathBuf::from("C:/guide.pdf"),
                DocumentContinuation::PdfPage(0),
            )
            .is_err()
        );
    }
}
