use std::collections::BTreeMap;

use continuehere::{DiscoveryEndpoint, Platform};
use slint::{ModelRc, SharedString, VecModel};

use crate::ui::{DeviceRow, MainWindow};

use super::DevicesAccess;

pub(super) struct DevicesSnapshot {
    local_name: String,
    local_detail: String,
    nearby: Vec<DeviceItem>,
    trusted: Vec<DeviceItem>,
    discovery_active: bool,
}

struct DeviceItem {
    id: String,
    name: String,
    detail: String,
    endpoint: String,
    connected: bool,
}

pub(super) fn snapshot(access: &DevicesAccess) -> DevicesSnapshot {
    let identity = access.devices().identity();
    let connections = access.transport().connections();
    let mut nearby_by_endpoint = BTreeMap::new();
    for candidate in access.discovery().candidates() {
        if let Some(endpoint) = candidate.endpoints().first() {
            insert_nearby(
                &mut nearby_by_endpoint,
                candidate.id().to_string(),
                endpoint.clone(),
            );
        }
    }
    let nearby = nearby_by_endpoint.into_values().collect();
    let trusted = access
        .pairing()
        .trusted_devices()
        .into_iter()
        .map(|device| DeviceItem {
            id: device.device_id().to_string(),
            name: device.display_name().to_owned(),
            detail: platform_name(device.platform()).to_owned(),
            endpoint: access
                .transport()
                .known_endpoint(device.device_id())
                .map(|endpoint| endpoint.to_string())
                .unwrap_or_default(),
            connected: connections
                .iter()
                .any(|connection| connection.device_id() == device.device_id()),
        })
        .collect();

    DevicesSnapshot {
        local_name: identity.display_name().to_owned(),
        local_detail: format!("{} · {}", platform_name(identity.platform()), identity.id()),
        nearby,
        trusted,
        discovery_active: matches!(
            access.discovery().status(),
            continuehere::DiscoveryStatus::Active
        ),
    }
}

fn insert_nearby(
    items: &mut BTreeMap<DiscoveryEndpoint, DeviceItem>,
    id: String,
    endpoint: DiscoveryEndpoint,
) {
    items.entry(endpoint.clone()).or_insert_with(|| DeviceItem {
        name: id.clone(),
        id,
        detail: endpoint.to_string(),
        endpoint: String::new(),
        connected: false,
    });
}

pub(super) fn apply_snapshot(window: &MainWindow, snapshot: DevicesSnapshot) {
    let selected = window.get_destination_device();
    if !snapshot
        .trusted
        .iter()
        .any(|item| item.id == selected.as_str() && item.connected)
    {
        window.set_destination_device(
            snapshot
                .trusted
                .iter()
                .find(|item| item.connected)
                .map(|item| item.id.as_str())
                .unwrap_or("")
                .into(),
        );
    }
    window.set_local_device_name(snapshot.local_name.into());
    let connected: Vec<_> = snapshot
        .trusted
        .iter()
        .filter(|item| item.connected)
        .collect();
    window.set_connected_device_names(crate::ui::shared::model(
        connected
            .iter()
            .map(|item| item.name.as_str().into())
            .collect(),
    ));
    window.set_connected_device_ids(crate::ui::shared::model(
        connected
            .iter()
            .map(|item| item.id.as_str().into())
            .collect(),
    ));
    window.set_connected_device_menu(crate::ui::shared::model(
        connected
            .iter()
            .map(|item| crate::ui::MenuItem {
                text: item.name.as_str().into(),
                enabled: true,
                ..Default::default()
            })
            .collect(),
    ));
    window.set_destination_index(
        connected
            .iter()
            .position(|item| item.id == window.get_destination_device().as_str())
            .map(|index| index as i32)
            .unwrap_or(-1),
    );
    window.set_local_device_detail(snapshot.local_detail.into());
    window.set_nearby_devices(device_model(snapshot.nearby));
    window.set_trusted_devices(device_model(snapshot.trusted));
    window.set_discovery_active(snapshot.discovery_active);
}

fn device_model(items: Vec<DeviceItem>) -> ModelRc<DeviceRow> {
    let rows: Vec<DeviceRow> = items
        .into_iter()
        .map(|item| DeviceRow {
            id: item.id.into(),
            name: SharedString::from(item.name),
            detail: SharedString::from(item.detail),
            endpoint: SharedString::from(item.endpoint),
            connected: item.connected,
        })
        .collect();
    ModelRc::new(VecModel::from(rows))
}

pub(in crate::ui) fn platform_name(platform: Platform) -> &'static str {
    match platform {
        Platform::Windows => "Windows",
        Platform::Linux => "Linux",
        Platform::MacOs => "macOS",
        Platform::Android => "Android",
        Platform::Ios => "iOS",
        Platform::Unknown => "Unknown",
        _ => "Unknown",
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use continuehere::DiscoveryEndpoint;

    use super::insert_nearby;

    #[test]
    fn nearby_candidates_are_coalesced_by_normalized_endpoint() {
        let mut items = BTreeMap::new();
        let first: DiscoveryEndpoint = "EXAMPLE.local:4242".parse().expect("valid endpoint");
        let duplicate: DiscoveryEndpoint = "example.local:4242".parse().expect("valid endpoint");

        insert_nearby(&mut items, "first".to_owned(), first);
        insert_nearby(&mut items, "duplicate".to_owned(), duplicate);

        assert_eq!(items.len(), 1);
        assert_eq!(items.into_values().next().expect("one item").id, "first");
    }
}
