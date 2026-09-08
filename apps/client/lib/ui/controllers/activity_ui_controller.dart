import 'dart:async';

import '../../src/rust/api/activity.dart';
import '../../src/rust/api/manager.dart';
import '../ui_notifications.dart';

typedef UiHistoryErrorDelegate = void Function(Object error);

class ActivityUiController {
  ActivityUiController(
    this._bridge,
    this._onChanged,
    this._onError,
    this._onActivity,
  );

  final UiBridge _bridge;
  final UiActivityDelegate _onChanged;
  final UiHistoryErrorDelegate _onError;
  final UiDeviceActivityDelegate _onActivity;
  StreamSubscription<UiActivityEvent>? _events;
  UiActivitySnapshot? _snapshot;
  final Map<String, BigInt> _revisions = {};
  Future<void> _pending = Future.value();
  bool _disposed = false;
  bool _busy = false;
  bool _initialized = false;

  List<UiActivity> get entries => List.unmodifiable(_snapshot?.entries ?? []);
  String? get storageError => _snapshot?.storageError;
  bool get busy => _busy;

  Future<void> start() async {
    _events = _bridge.watchActivity().listen((_) {
      _pending = _pending.then((_) => _refresh(notify: true));
    }, onError: _onError);
    _pending = _pending.then((_) => _refresh(notify: false));
    await _pending;
    _initialized = true;
  }

  Future<void> _refresh({required bool notify}) async {
    if (_disposed) {
      return;
    }
    try {
      final snapshot = await _bridge.activitySnapshot();
      if (_disposed) {
        return;
      }
      final changedDevices = <String>{};
      for (final entry in snapshot.entries) {
        final revision = _revisions[entry.id];
        if (notify &&
            _initialized &&
            (revision == null || revision < entry.revision)) {
          changedDevices.add(entry.deviceId);
        }
      }
      _snapshot = snapshot;
      _revisions
        ..clear()
        ..addEntries(
          snapshot.entries.map((entry) => MapEntry(entry.id, entry.revision)),
        );
      _onChanged();
      for (final deviceId in changedDevices) {
        _onActivity(deviceId);
      }
    } catch (error) {
      if (!_disposed) {
        _onError(error);
      }
    }
  }

  Future<void> retry(UiActivity activity) =>
      _run(() => _bridge.retryActivity(activityId: activity.id));
  Future<void> remove(UiActivity activity) =>
      _run(() => _bridge.removeActivity(activityId: activity.id));
  Future<void> clear() => _run(_bridge.clearHistory);

  Future<void> _run(Future<void> Function() action) async {
    if (_busy || _disposed) {
      return;
    }
    _busy = true;
    _onChanged();
    try {
      await action();
      _pending = _pending.then((_) => _refresh(notify: true));
      await _pending;
    } catch (error) {
      if (!_disposed) {
        _onError(error);
      }
    } finally {
      _busy = false;
      if (!_disposed) {
        _onChanged();
      }
    }
  }

  Future<void> dispose() async {
    _disposed = true;
    await _events?.cancel();
    await _pending;
  }
}
