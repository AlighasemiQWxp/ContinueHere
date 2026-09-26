use std::{path::PathBuf, rc::Rc};

use continuehere::{DeviceId, DocumentContinuation};

use super::super::{
    phase::{PhaseChangedDelegate, PhaseChangedSubscription, PhaseController},
    support::UiResult,
};

#[derive(Clone, Copy)]
pub(super) enum DocumentKind {
    Pdf,
    PowerPoint,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum DocumentPhase {
    Idle,
    SelectingDocument,
    EditingContinuation,
}

pub(super) struct DocumentDraft {
    pub device: DeviceId,
    pub path: PathBuf,
    pub kind: DocumentKind,
}

pub(super) struct DocumentSelection(Rc<()>);

pub(super) struct DocumentPreparation {
    phases: PhaseController<DocumentPhase>,
    selection: Option<Rc<()>>,
    draft: Option<DocumentDraft>,
}

impl DocumentPreparation {
    pub(super) fn new() -> Self {
        Self {
            phases: PhaseController::new(DocumentPhase::Idle),
            selection: None,
            draft: None,
        }
    }

    pub(super) fn phase(&self) -> &DocumentPhase {
        self.phases.phase()
    }

    pub(super) fn is_running(&self) -> bool {
        *self.phase() != DocumentPhase::Idle
    }

    pub(super) fn on_phase_changed(
        &self,
        delegate: PhaseChangedDelegate<DocumentPhase>,
    ) -> PhaseChangedSubscription<DocumentPhase> {
        self.phases.on_phase_changed(delegate)
    }

    pub(super) fn start_selection(&mut self) -> UiResult<DocumentSelection> {
        if self.is_running() {
            return Err("Finish the current document preparation first.".into());
        }
        let request = Rc::new(());
        self.selection = Some(Rc::clone(&request));
        self.phases.transition_to(DocumentPhase::SelectingDocument);
        Ok(DocumentSelection(request))
    }

    pub(super) fn accepts_selection(&self, request: &DocumentSelection) -> bool {
        *self.phase() == DocumentPhase::SelectingDocument
            && self
                .selection
                .as_ref()
                .is_some_and(|active| Rc::ptr_eq(active, &request.0))
    }

    pub(super) fn edit(&mut self, request: &DocumentSelection, draft: DocumentDraft) -> bool {
        if !self.accepts_selection(request) {
            return false;
        }
        self.selection = None;
        self.draft = Some(draft);
        self.phases
            .transition_to(DocumentPhase::EditingContinuation);
        true
    }

    pub(super) fn draft(&self) -> UiResult<&DocumentDraft> {
        self.draft.as_ref().ok_or("No document is selected.".into())
    }

    pub(super) fn continuation(&self, position: &str) -> UiResult<DocumentContinuation> {
        let draft = self.draft()?;
        let position = position
            .trim()
            .parse::<u32>()
            .map_err(|_| "Enter a valid page or slide number.")?;
        let continuation = match draft.kind {
            DocumentKind::Pdf => DocumentContinuation::pdf_page(position)?,
            DocumentKind::PowerPoint => DocumentContinuation::powerpoint_slide(position)?,
        };
        Ok(continuation)
    }

    pub(super) fn cancel(&mut self) {
        self.selection = None;
        self.draft = None;
        self.phases.transition_to(DocumentPhase::Idle);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn draft(kind: DocumentKind) -> DocumentDraft {
        DocumentDraft {
            device: DeviceId::new("test-device").unwrap(),
            path: PathBuf::from("document.pdf"),
            kind,
        }
    }

    #[test]
    fn cancelled_and_replaced_selections_cannot_reopen_preparation() {
        let mut preparation = DocumentPreparation::new();
        let old = preparation.start_selection().unwrap();
        assert!(preparation.start_selection().is_err());
        preparation.cancel();
        assert!(!preparation.is_running());
        assert!(!preparation.edit(&old, draft(DocumentKind::Pdf)));
        let current = preparation.start_selection().unwrap();
        assert!(!preparation.edit(&old, draft(DocumentKind::Pdf)));
        assert!(preparation.edit(&current, draft(DocumentKind::Pdf)));
        assert!(!preparation.edit(&current, draft(DocumentKind::Pdf)));
        assert_eq!(*preparation.phase(), DocumentPhase::EditingContinuation);
        preparation.cancel();
        assert!(preparation.draft().is_err());
        assert_eq!(*preparation.phase(), DocumentPhase::Idle);
    }

    #[test]
    fn invalid_input_preserves_the_document_for_correction() {
        for kind in [DocumentKind::Pdf, DocumentKind::PowerPoint] {
            let mut preparation = DocumentPreparation::new();
            assert!(preparation.continuation("1").is_err());
            let request = preparation.start_selection().unwrap();
            preparation.edit(&request, draft(kind));
            for value in ["", "0", "-1", "text", "4294967296"] {
                assert!(preparation.continuation(value).is_err());
                assert_eq!(*preparation.phase(), DocumentPhase::EditingContinuation);
                assert!(preparation.draft().is_ok());
            }
            assert!(preparation.continuation(" 2 ").is_ok());
        }
    }
}
