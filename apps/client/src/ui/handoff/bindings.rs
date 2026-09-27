use std::{cell::RefCell, rc::Rc};

use slint::ComponentHandle;

use crate::ui::{
    MainWindow,
    preview::PreviewOpener,
    selection::{ContentSelectionUiController, selected_path},
    shared::{UiResult, playback_position, show_result},
    transition::UiTransitionController,
};

use super::{
    DocumentAction, DocumentDraft, DocumentKind, HandoffKind, HandoffUiController,
    SendHandoffCommand,
};

impl HandoffUiController {
    pub(super) fn bind_callbacks(
        controller: Rc<RefCell<Self>>,
        window: &MainWindow,
        transitions: Rc<UiTransitionController>,
        selection: Rc<RefCell<ContentSelectionUiController>>,
        preview: PreviewOpener,
    ) {
        let weak = Rc::downgrade(&controller);
        let view = window.as_weak();
        let document_transitions = Rc::clone(&transitions);
        let document_selection = Rc::clone(&selection);
        window.on_choose_document(move |device, kind| {
            if let (Some(controller), Some(window)) = (weak.upgrade(), view.upgrade()) {
                let result = (|| -> UiResult {
                    let (device, kind, selection_kind) = controller.borrow().document_selection(
                        device.as_str(),
                        DocumentKind::parse(kind.as_str()).ok_or("Unknown document type.")?,
                    )?;
                    let selected_controller = Rc::downgrade(&controller);
                    let selected_transitions = Rc::clone(&document_transitions);
                    let request = controller.borrow_mut().document.start_selection()?;
                    let result = document_selection.borrow_mut().choose_file(
                        &window,
                        selection_kind,
                        move |window, result| {
                            let Some(controller) = selected_controller.upgrade() else {
                                return;
                            };
                            let mut controller = controller.borrow_mut();
                            if !controller.document.accepts_selection(&request) {
                                return;
                            }
                            let result = selected_path(result).and_then(|path| {
                                let Some(path) = path else {
                                    controller.close_document(window);
                                    return Ok(());
                                };
                                controller.begin_document(
                                    &request,
                                    DocumentDraft { device, kind, path },
                                    window,
                                    selected_transitions.as_ref(),
                                )
                            });
                            if result.is_err() {
                                controller.close_document(window);
                            }
                            drop(controller);
                            show_result(window, result);
                        },
                    );
                    if result.is_err() {
                        controller.borrow_mut().close_document(&window);
                    }
                    result
                })();
                show_result(&window, result);
            }
        });
        let weak = Rc::downgrade(&controller);
        let view = window.as_weak();
        window.on_document_action(move |action, position| {
            if let (Some(controller), Some(window)) = (weak.upgrade(), view.upgrade()) {
                let action = DocumentAction::parse(action.as_str());
                let result = match action {
                    Some(action) => {
                        controller
                            .borrow_mut()
                            .document_action(action, position.as_str(), &window)
                    }
                    None => Err("Unknown document action.".into()),
                };
                if result.is_ok() && matches!(action, Some(DocumentAction::Send)) {
                    window.invoke_show_notice(
                        crate::ui::shared::text(
                            window.get_rtl(),
                            "Document handoff started.",
                            "ارسال سند آغاز شد.",
                        )
                        .into(),
                    );
                }
                show_result(&window, result);
            }
        });
        let weak = Rc::downgrade(&controller);
        let view = window.as_weak();
        let handoff_selection = selection;
        window.on_send_handoff(move |kind, device, value, position| {
            if let (Some(controller), Some(window)) = (weak.upgrade(), view.upgrade()) {
                let command = match SendHandoffCommand::parse(kind.as_str()) {
                    Some(command) => command,
                    None => {
                        let result: UiResult = Err("Unknown handoff kind.".into());
                        show_result(&window, result);
                        return;
                    }
                };
                match command {
                    SendHandoffCommand::Preview => {
                        let preview = preview.clone();
                        let result = handoff_selection.borrow_mut().choose_file(
                            &window,
                            crate::platform::SelectionKind::Video,
                            move |window, result| {
                                let result = selected_path(result).and_then(|path| {
                                    if let Some(path) = path {
                                        preview.open(path, 0)?;
                                    }
                                    Ok(())
                                });
                                show_result(window, result);
                            },
                        );
                        show_result(&window, result);
                    }
                    SendHandoffCommand::CurrentVideo => {
                        let result = controller.borrow_mut().send_current_video(
                            device.as_str(),
                            value.as_str(),
                            window.get_preview_position(),
                        );
                        if result.is_ok() {
                            window.invoke_show_notice(
                                crate::ui::shared::text(
                                    window.get_rtl(),
                                    "Video handoff started.",
                                    "ارسال ویدیو آغاز شد.",
                                )
                                .into(),
                            );
                        }
                        show_result(&window, result);
                    }
                    SendHandoffCommand::Video => {
                        let result = (|| -> UiResult {
                            let device = controller.borrow().available_device(device.as_str())?;
                            let position = playback_position(position.as_str())?;
                            let selected_controller = Rc::downgrade(&controller);
                            handoff_selection.borrow_mut().choose_file(
                                &window,
                                crate::platform::SelectionKind::Video,
                                move |window, result| {
                                    let result = selected_path(result).and_then(|path| {
                                        let Some(path) = path else {
                                            return Ok(());
                                        };
                                        let Some(controller) = selected_controller.upgrade() else {
                                            return Ok(());
                                        };
                                        controller
                                            .borrow_mut()
                                            .send_local_video(device, path, position)
                                    });
                                    show_result(window, result);
                                },
                            )
                        })();
                        show_result(&window, result);
                    }
                    SendHandoffCommand::Url => {
                        let result = controller.borrow_mut().send(
                            HandoffKind::Url,
                            device.as_str(),
                            value.as_str(),
                            position.as_str(),
                        );
                        show_result(&window, result);
                        controller.borrow_mut().refresh(&window);
                    }
                    SendHandoffCommand::YouTube => {
                        let result = controller.borrow_mut().send(
                            HandoffKind::YouTube,
                            device.as_str(),
                            value.as_str(),
                            position.as_str(),
                        );
                        show_result(&window, result);
                        controller.borrow_mut().refresh(&window);
                    }
                }
            }
        });
        let weak = Rc::downgrade(&controller);
        let view = window.as_weak();
        window.on_refresh_handoffs_requested(move || {
            if let (Some(controller), Some(window)) = (weak.upgrade(), view.upgrade()) {
                controller.borrow_mut().refresh(&window);
            }
        });
    }
}
