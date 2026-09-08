use std::{path::PathBuf, sync::Mutex};

use continuehere::{ContinueHere, DeviceId, DiscoveryEndpoint, FileTransferId, HandoffId};
use tokio::sync::Mutex as AsyncMutex;

use crate::frb_generated::StreamSink;

use super::{
    DevicesUiEvent, HandoffUiEvent, PairingUiEvent, SettingsUiEvent, TransferUiEvent,
    UiBridgeError, UiDevicesSnapshot, UiDiscoveryHandle, UiFileTransfer, UiFileTransferHandle,
    UiHandoffHandle, UiHandoffSnapshot, UiLanguage, UiPairingHandle, UiPairingSnapshot,
    UiSettingsSnapshot,
    events::{
        DevicesUiSubscription, HandoffUiSubscription, PairingUiSubscription,
        SettingsUiSubscription, TransferUiSubscription, UiSubscriptions,
    },
    models::{
        candidate, connection, core_language, discovery_status, file_transfer, handoff,
        incoming_handoff, language, local_device, pairing_session, text_direction, trusted_device,
    },
};

#[flutter_rust_bridge::frb(opaque)]
pub struct UiBridge {
    app: AsyncMutex<Option<ContinueHere>>,
    subscriptions: Mutex<UiSubscriptions>,
}

impl UiBridge {
    pub async fn activity_snapshot(&self) -> Result<super::UiActivitySnapshot, UiBridgeError> {
        let app = self.app.lock().await;
        let snapshot = running_app(&app)?
            .activity()
            .activities()
            .map_err(UiBridgeError::operation)?;
        Ok(super::UiActivitySnapshot {
            entries: snapshot
                .entries()
                .iter()
                .map(super::activity::activity)
                .collect(),
            storage_error: snapshot.storage_error().map(str::to_owned),
        })
    }

    pub async fn retry_activity(&self, activity_id: String) -> Result<(), UiBridgeError> {
        let app = self.app.lock().await;
        running_app(&app)?
            .activity()
            .retry(&activity_id)
            .map_err(UiBridgeError::operation)
    }

    pub async fn remove_activity(&self, activity_id: String) -> Result<(), UiBridgeError> {
        let app = self.app.lock().await;
        running_app(&app)?
            .activity()
            .remove(&activity_id)
            .map_err(UiBridgeError::operation)
    }

    pub async fn clear_history(&self) -> Result<(), UiBridgeError> {
        let app = self.app.lock().await;
        running_app(&app)?
            .activity()
            .clear()
            .map_err(UiBridgeError::operation)
    }

    pub async fn watch_activity(
        &self,
        sink: StreamSink<super::UiActivityEvent>,
    ) -> Result<(), UiBridgeError> {
        let app = self.app.lock().await;
        let subscription =
            running_app(&app)?
                .activity()
                .on_changed(continuehere::ActivityChangedDelegate::new(move |_| {
                    let _ = sink.add(super::UiActivityEvent::Changed);
                }));
        self.subscriptions
            .lock()
            .map_err(|_| UiBridgeError::synchronization())?
            .activity = Some(subscription);
        Ok(())
    }

    pub async fn start(project_directory: String) -> Result<Self, UiBridgeError> {
        let app = ContinueHere::builder(PathBuf::from(project_directory))
            .build()
            .await
            .map_err(UiBridgeError::operation)?;
        Ok(Self {
            app: AsyncMutex::new(Some(app)),
            subscriptions: Mutex::new(UiSubscriptions::default()),
        })
    }

    pub async fn shutdown(&self) -> Result<(), UiBridgeError> {
        self.clear_subscriptions()?;
        let app = self.app.lock().await.take();
        let Some(app) = app else {
            return Ok(());
        };
        app.shutdown().await.map_err(UiBridgeError::operation)
    }

    pub async fn devices_snapshot(&self) -> Result<UiDevicesSnapshot, UiBridgeError> {
        let app = self.app.lock().await;
        let app = running_app(&app)?;
        Ok(UiDevicesSnapshot {
            local_device: local_device(app.devices().identity()),
            discovery_status: discovery_status(app.discovery().status()),
            candidates: app
                .discovery()
                .candidates()
                .into_iter()
                .map(candidate)
                .collect(),
            trusted_devices: app
                .pairing()
                .trusted_devices()
                .into_iter()
                .map(trusted_device)
                .collect(),
            connections: app
                .transport()
                .connections()
                .into_iter()
                .map(connection)
                .collect(),
        })
    }

    pub async fn pairing_snapshot(&self) -> Result<UiPairingSnapshot, UiBridgeError> {
        let app = self.app.lock().await;
        let app = running_app(&app)?;
        let endpoint = app
            .pairing()
            .listening_endpoint()
            .map_err(UiBridgeError::operation)?;
        Ok(UiPairingSnapshot {
            listening_endpoint: super::UiEndpoint {
                host: endpoint.host().to_owned(),
                port: endpoint.port(),
            },
            sessions: app
                .pairing()
                .sessions()
                .into_iter()
                .map(pairing_session)
                .collect(),
        })
    }

    pub async fn handoff_snapshot(&self) -> Result<UiHandoffSnapshot, UiBridgeError> {
        let app = self.app.lock().await;
        let app = running_app(&app)?;
        Ok(UiHandoffSnapshot {
            outgoing: app.handoff().handoffs().into_iter().map(handoff).collect(),
            incoming: app
                .handoff()
                .incoming()
                .into_iter()
                .map(incoming_handoff)
                .collect(),
        })
    }

    pub async fn transfers(&self) -> Result<Vec<UiFileTransfer>, UiBridgeError> {
        let app = self.app.lock().await;
        let app = running_app(&app)?;
        Ok(app
            .file_transfers()
            .transfers()
            .into_iter()
            .map(file_transfer)
            .collect())
    }

    pub async fn settings_snapshot(&self) -> Result<UiSettingsSnapshot, UiBridgeError> {
        let app = self.app.lock().await;
        let app = running_app(&app)?;
        Ok(UiSettingsSnapshot {
            display_name: app.devices().identity().display_name().to_owned(),
            default_transfer_directory: app
                .directories()
                .default_transfer_directory()
                .to_string_lossy()
                .into_owned(),
            language: language(app.localization().language()),
            text_direction: text_direction(app.localization().text_direction()),
        })
    }

    pub async fn discovery_handle(
        &self,
        identifier: String,
    ) -> Result<UiDiscoveryHandle, UiBridgeError> {
        let app = self.app.lock().await;
        let handle = running_app(&app)?
            .discovery()
            .get_handle(&identifier)
            .map_err(UiBridgeError::operation)?;
        Ok(UiDiscoveryHandle::new(handle))
    }

    pub async fn pairing_handle(
        &self,
        identifier: String,
    ) -> Result<UiPairingHandle, UiBridgeError> {
        let app = self.app.lock().await;
        let handle = running_app(&app)?
            .pairing()
            .get_handle(&identifier)
            .map_err(UiBridgeError::operation)?;
        Ok(UiPairingHandle::new(handle))
    }

    pub async fn handoff_handle(
        &self,
        identifier: String,
    ) -> Result<UiHandoffHandle, UiBridgeError> {
        let app = self.app.lock().await;
        let handle = running_app(&app)?
            .handoff()
            .get_handle(&identifier)
            .map_err(UiBridgeError::operation)?;
        Ok(UiHandoffHandle::new(handle))
    }

    pub async fn file_transfer_handle(
        &self,
        identifier: String,
    ) -> Result<UiFileTransferHandle, UiBridgeError> {
        let app = self.app.lock().await;
        let handle = running_app(&app)?
            .file_transfers()
            .get_handle(&identifier)
            .map_err(UiBridgeError::operation)?;
        Ok(UiFileTransferHandle::new(handle))
    }

    pub async fn connect(
        &self,
        device_id: String,
        host: String,
        port: u16,
    ) -> Result<(), UiBridgeError> {
        let device_id = DeviceId::new(device_id).map_err(UiBridgeError::invalid_input)?;
        let endpoint = DiscoveryEndpoint::new(host, port).map_err(UiBridgeError::invalid_input)?;
        let app = self.app.lock().await;
        let transport = running_app(&app)?.transport();
        transport
            .connect(&device_id, &endpoint)
            .await
            .map(|_| ())
            .map_err(UiBridgeError::operation)
    }

    pub async fn disconnect(&self, device_id: String) -> Result<(), UiBridgeError> {
        let device_id = DeviceId::new(device_id).map_err(UiBridgeError::invalid_input)?;
        let app = self.app.lock().await;
        let transport = running_app(&app)?.transport();
        transport
            .disconnect(&device_id)
            .await
            .map_err(UiBridgeError::operation)
    }

    pub async fn remove_trusted_device(&self, device_id: String) -> Result<(), UiBridgeError> {
        let device_id = DeviceId::new(device_id).map_err(UiBridgeError::invalid_input)?;
        let app = self.app.lock().await;
        running_app(&app)?
            .pairing()
            .remove_trusted_device(&device_id)
            .map_err(UiBridgeError::operation)
    }

    pub async fn accept_transfer(
        &self,
        transfer_id: String,
        selected_directory: Option<String>,
    ) -> Result<(), UiBridgeError> {
        let app = self.app.lock().await;
        let app = running_app(&app)?;
        let transfer_id = find_transfer_id(app, &transfer_id)?;
        let selected_directory = selected_directory.map(PathBuf::from);
        app.file_transfers()
            .accept_incoming(&transfer_id, selected_directory.as_deref())
            .map_err(UiBridgeError::operation)
    }

    pub async fn reject_transfer(&self, transfer_id: String) -> Result<(), UiBridgeError> {
        let app = self.app.lock().await;
        let app = running_app(&app)?;
        let transfer_id = find_transfer_id(app, &transfer_id)?;
        app.file_transfers()
            .reject_incoming(&transfer_id)
            .map_err(UiBridgeError::operation)
    }

    pub async fn remove_transfer(&self, transfer_id: String) -> Result<(), UiBridgeError> {
        let app = self.app.lock().await;
        let app = running_app(&app)?;
        let transfer_id = find_transfer_id(app, &transfer_id)?;
        app.file_transfers()
            .remove(&transfer_id)
            .map_err(UiBridgeError::operation)
    }

    pub async fn remove_incoming_handoff(&self, handoff_id: String) -> Result<(), UiBridgeError> {
        let app = self.app.lock().await;
        let app = running_app(&app)?;
        let handoff_id = find_incoming_handoff_id(app, &handoff_id)?;
        app.handoff()
            .remove_incoming(&handoff_id)
            .map_err(UiBridgeError::operation)
    }

    pub async fn set_display_name(&self, display_name: String) -> Result<(), UiBridgeError> {
        let app = self.app.lock().await;
        running_app(&app)?
            .devices()
            .set_display_name(display_name)
            .map_err(UiBridgeError::operation)
    }

    pub async fn set_default_transfer_directory(
        &self,
        directory: String,
    ) -> Result<(), UiBridgeError> {
        let app = self.app.lock().await;
        running_app(&app)?
            .settings()
            .directories()
            .set_default_transfer_directory(PathBuf::from(directory))
            .map_err(UiBridgeError::operation)
    }

    pub async fn set_language(&self, selected_language: UiLanguage) -> Result<(), UiBridgeError> {
        let app = self.app.lock().await;
        running_app(&app)?
            .settings()
            .localization()
            .set_language(core_language(selected_language))
            .map_err(UiBridgeError::operation)
    }

    pub async fn watch_devices(
        &self,
        sink: StreamSink<DevicesUiEvent>,
    ) -> Result<(), UiBridgeError> {
        let app = self.app.lock().await;
        let subscription = DevicesUiSubscription::new(running_app(&app)?, sink);
        self.subscriptions
            .lock()
            .map_err(|_| UiBridgeError::synchronization())?
            .devices = Some(subscription);
        Ok(())
    }

    pub async fn watch_pairing(
        &self,
        sink: StreamSink<PairingUiEvent>,
    ) -> Result<(), UiBridgeError> {
        let app = self.app.lock().await;
        let subscription = PairingUiSubscription::new(running_app(&app)?, sink);
        self.subscriptions
            .lock()
            .map_err(|_| UiBridgeError::synchronization())?
            .pairing = Some(subscription);
        Ok(())
    }

    pub async fn watch_handoffs(
        &self,
        sink: StreamSink<HandoffUiEvent>,
    ) -> Result<(), UiBridgeError> {
        let app = self.app.lock().await;
        let subscription = HandoffUiSubscription::new(running_app(&app)?, sink);
        self.subscriptions
            .lock()
            .map_err(|_| UiBridgeError::synchronization())?
            .handoff = Some(subscription);
        Ok(())
    }

    pub async fn watch_transfers(
        &self,
        sink: StreamSink<TransferUiEvent>,
    ) -> Result<(), UiBridgeError> {
        let app = self.app.lock().await;
        let subscription = TransferUiSubscription::new(running_app(&app)?, sink);
        self.subscriptions
            .lock()
            .map_err(|_| UiBridgeError::synchronization())?
            .transfer = Some(subscription);
        Ok(())
    }

    pub async fn watch_settings(
        &self,
        sink: StreamSink<SettingsUiEvent>,
    ) -> Result<(), UiBridgeError> {
        let app = self.app.lock().await;
        let subscription = SettingsUiSubscription::new(running_app(&app)?, sink);
        self.subscriptions
            .lock()
            .map_err(|_| UiBridgeError::synchronization())?
            .settings = Some(subscription);
        Ok(())
    }

    fn clear_subscriptions(&self) -> Result<(), UiBridgeError> {
        let mut subscriptions = self
            .subscriptions
            .lock()
            .map_err(|_| UiBridgeError::synchronization())?;
        *subscriptions = UiSubscriptions::default();
        Ok(())
    }
}

fn running_app(app: &Option<ContinueHere>) -> Result<&ContinueHere, UiBridgeError> {
    app.as_ref().ok_or_else(UiBridgeError::not_running)
}

fn find_transfer_id(app: &ContinueHere, value: &str) -> Result<FileTransferId, UiBridgeError> {
    app.file_transfers()
        .transfers()
        .into_iter()
        .find(|transfer| transfer.id().as_str() == value)
        .map(|transfer| transfer.id().clone())
        .ok_or_else(|| UiBridgeError::invalid_input("unknown file transfer"))
}

fn find_incoming_handoff_id(app: &ContinueHere, value: &str) -> Result<HandoffId, UiBridgeError> {
    app.handoff()
        .incoming()
        .into_iter()
        .find(|handoff| handoff.id().as_str() == value)
        .map(|handoff| handoff.id().clone())
        .ok_or_else(|| UiBridgeError::invalid_input("unknown incoming handoff"))
}

#[flutter_rust_bridge::frb(init)]
pub fn init_app() {
    flutter_rust_bridge::setup_default_user_utils();
}
