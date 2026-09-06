import 'dart:async';

import '../../platform/platform_manager.dart';
import '../../src/rust/api/events.dart';
import '../../src/rust/api/handles.dart';
import '../../src/rust/api/manager.dart';
import '../../src/rust/api/models.dart';
import '../ui_notifications.dart';

class HandoffUiController {
  HandoffUiController(
    this._bridge,
    this._platform,
    this._onChanged,
    this._onError,
    this._onActivity,
  );

  final UiBridge _bridge;
  final PlatformManager _platform;
  final void Function() _onChanged;
  final void Function(Object) _onError;
  final UiActivityDelegate _onActivity;
  final List<UiHandoffHandle> _handles = [];
  StreamSubscription<HandoffUiEvent>? _events;
  UiHandoffSnapshot? _snapshot;
  int _nextHandle = 0;
  bool _busy = false;

  UiHandoffSnapshot? get snapshot => _snapshot;
  bool get busy => _busy;

  Future<void> start() async {
    _events = _bridge.watchHandoffs().listen(
      (_) => unawaited(_refreshFromEvent()),
      onError: _onError,
    );
    await refresh();
  }

  Future<void> _refreshFromEvent() async {
    await refresh();
    _onActivity();
  }

  Future<void> sendUrl(String deviceId, String url) {
    return _send((handle) => handle.prepareUrl(deviceId: deviceId, url: url));
  }

  Future<void> sendYoutube(String deviceId, String url, Duration position) {
    return _send(
      (handle) => handle.prepareYoutube(
        deviceId: deviceId,
        url: url,
        playbackPositionMillis: BigInt.from(position.inMilliseconds),
      ),
    );
  }

  Future<void> sendLocalVideo(
    String deviceId,
    String source,
    Duration position,
  ) {
    return _send(
      (handle) => handle.prepareLocalVideo(
        deviceId: deviceId,
        source: source,
        playbackPositionMillis: BigInt.from(position.inMilliseconds),
      ),
    );
  }

  Future<void> openIncoming(UiIncomingHandoff handoff) async {
    final url = handoff.payload.url;
    if (url != null) {
      try {
        await _platform.openExternalUrl(url);
      } catch (error) {
        _onError(error);
      }
    }
  }

  Future<void> removeIncoming(UiIncomingHandoff handoff) async {
    await _run(() => _bridge.removeIncomingHandoff(handoffId: handoff.id));
  }

  Future<void> refresh() async {
    try {
      _snapshot = await _bridge.handoffSnapshot();
      await _releaseFinishedHandles();
      _onChanged();
    } catch (error) {
      _onError(error);
    }
  }

  Future<void> _releaseFinishedHandles() async {
    for (final handle in List<UiHandoffHandle>.from(_handles)) {
      final handoff = await handle.handoff();
      if (!_handles.contains(handle) ||
          handoff == null ||
          handoff.state == UiHandoffState.sending) {
        continue;
      }
      _handles.remove(handle);
      try {
        await handle.release();
      } finally {
        handle.dispose();
      }
    }
  }

  Future<void> _send(Future<void> Function(UiHandoffHandle) configure) async {
    await _run(() async {
      _nextHandle++;
      final handle = await _bridge.handoffHandle(
        identifier: 'ui.handoff.$_nextHandle',
      );
      try {
        await configure(handle);
        await handle.useHandle();
        _handles.add(handle);
      } catch (_) {
        await handle.release();
        handle.dispose();
        rethrow;
      }
    });
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
    for (final handle in _handles) {
      await handle.release();
      handle.dispose();
    }
  }
}
