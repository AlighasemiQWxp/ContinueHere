import 'dart:async';

import '../../src/rust/api/events.dart';
import '../../src/rust/api/handles.dart';
import '../../src/rust/api/manager.dart';
import '../../src/rust/api/models.dart';
import '../ui_notifications.dart';

class DevicesUiController {
  DevicesUiController(
    this._bridge,
    this._onChanged,
    this._onError,
    this._onActivity,
  );

  final UiBridge _bridge;
  final void Function() _onChanged;
  final void Function(Object) _onError;
  final UiActivityDelegate _onActivity;
  final List<UiDiscoveryHandle> _manualHandles = [];
  StreamSubscription<DevicesUiEvent>? _events;
  UiDiscoveryHandle? _localDiscovery;
  UiDevicesSnapshot? _snapshot;
  int _nextHandle = 0;
  bool _busy = false;

  UiDevicesSnapshot? get snapshot => _snapshot;
  bool get busy => _busy;

  Future<void> start() async {
    _events = _bridge.watchDevices().listen(
      (_) => unawaited(_refreshFromEvent()),
      onError: _onError,
    );
    await refresh();
    await startLocalDiscovery();
  }

  Future<void> _refreshFromEvent() async {
    await refresh();
    _onActivity();
  }

  Future<void> startLocalDiscovery() async {
    if (_localDiscovery != null) {
      return;
    }
    await _run(() async {
      final handle = await _bridge.discoveryHandle(
        identifier: 'ui.discovery.local',
      );
      await handle.browseLocal();
      await handle.useHandle();
      _localDiscovery = handle;
    });
  }

  Future<void> addManualEndpoint(String host, int port) async {
    await _run(() async {
      _nextHandle++;
      final handle = await _bridge.discoveryHandle(
        identifier: 'ui.discovery.manual.$_nextHandle',
      );
      await handle.useManualEndpoint(host: host, port: port);
      await handle.useHandle();
      _manualHandles.add(handle);
    });
  }

  Future<void> connect(UiTrustedDevice device) async {
    final candidate = _candidateFor(device.id);
    if (candidate == null || candidate.endpoints.isEmpty) {
      _onError(StateError('The trusted device is not currently discoverable.'));
      return;
    }
    final endpoint = candidate.endpoints.first;
    await _run(() {
      return _bridge.connect(
        deviceId: device.id,
        host: endpoint.host,
        port: endpoint.port,
      );
    });
  }

  Future<void> disconnect(UiTrustedDevice device) async {
    await _run(() => _bridge.disconnect(deviceId: device.id));
  }

  Future<void> removeTrustedDevice(UiTrustedDevice device) async {
    await _run(() => _bridge.removeTrustedDevice(deviceId: device.id));
  }

  bool isConnected(String deviceId) {
    final connections = _snapshot?.connections ?? [];
    return connections.any((connection) => connection.deviceId == deviceId);
  }

  UiDiscoveryCandidate? _candidateFor(String deviceId) {
    final candidates = _snapshot?.candidates ?? [];
    for (final candidate in candidates) {
      if (candidate.id == deviceId) {
        return candidate;
      }
    }
    return null;
  }

  Future<void> refresh() async {
    try {
      _snapshot = await _bridge.devicesSnapshot();
      _onChanged();
    } catch (error) {
      _onError(error);
    }
  }

  Future<void> _run(Future<void> Function() action) async {
    if (_busy) {
      return;
    }
    _busy = true;
    _onChanged();
    try {
      await action();
      await refresh();
    } catch (error) {
      _onError(error);
    } finally {
      _busy = false;
      _onChanged();
    }
  }

  Future<void> dispose() async {
    await _events?.cancel();
    for (final handle in _manualHandles) {
      await handle.release();
      handle.dispose();
    }
    final localDiscovery = _localDiscovery;
    if (localDiscovery != null) {
      await localDiscovery.release();
      localDiscovery.dispose();
    }
  }
}
