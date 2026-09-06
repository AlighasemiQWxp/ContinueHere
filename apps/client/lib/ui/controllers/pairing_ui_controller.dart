import 'dart:async';

import '../../src/rust/api/events.dart';
import '../../src/rust/api/handles.dart';
import '../../src/rust/api/manager.dart';
import '../../src/rust/api/models.dart';

class PairingUiController {
  PairingUiController(this._bridge, this._onChanged, this._onError);

  final UiBridge _bridge;
  final void Function() _onChanged;
  final void Function(Object) _onError;
  StreamSubscription<PairingUiEvent>? _events;
  UiPairingHandle? _activeHandle;
  UiPairingSnapshot? _snapshot;
  int _nextHandle = 0;
  bool _busy = false;

  UiPairingSnapshot? get snapshot => _snapshot;
  bool get busy => _busy;

  Future<void> start() async {
    _events = _bridge.watchPairing().listen(
      (_) => unawaited(refresh()),
      onError: _onError,
    );
    await refresh();
  }

  Future<void> pair(UiDiscoveryCandidate candidate) async {
    if (candidate.endpoints.isEmpty) {
      _onError(StateError('The discovered device has no usable endpoint.'));
      return;
    }
    final endpoint = candidate.endpoints.first;
    await _replaceHandle((handle) async {
      await handle.prepareInitiator(host: endpoint.host, port: endpoint.port);
      await handle.useHandle();
    });
  }

  Future<void> receive() async {
    await _replaceHandle((handle) async {
      await handle.prepareReceiver();
      await handle.useHandle();
    });
  }

  Future<void> approve() async {
    final handle = _activeHandle;
    if (handle == null) {
      return;
    }
    await _run(handle.approve);
  }

  Future<void> reject() async {
    final handle = _activeHandle;
    if (handle == null) {
      return;
    }
    await _run(handle.reject);
  }

  Future<void> cancel() async {
    final handle = _activeHandle;
    if (handle == null) {
      return;
    }
    _activeHandle = null;
    try {
      await handle.release();
      handle.dispose();
      await refresh();
    } catch (error) {
      _onError(error);
    }
  }

  Future<void> refresh() async {
    try {
      _snapshot = await _bridge.pairingSnapshot();
      await _releaseFinishedHandle();
      _onChanged();
    } catch (error) {
      _onError(error);
    }
  }

  Future<void> _releaseFinishedHandle() async {
    final handle = _activeHandle;
    if (handle == null) {
      return;
    }
    final session = await handle.session();
    if (!identical(_activeHandle, handle) ||
        session == null ||
        !_isFinished(session.state)) {
      return;
    }
    _activeHandle = null;
    try {
      await handle.release();
    } finally {
      handle.dispose();
    }
  }

  bool _isFinished(UiPairingState state) {
    return state == UiPairingState.trusted ||
        state == UiPairingState.rejected ||
        state == UiPairingState.cancelled ||
        state == UiPairingState.expired ||
        state == UiPairingState.failed;
  }

  Future<void> _replaceHandle(
    Future<void> Function(UiPairingHandle) configure,
  ) async {
    await cancel();
    await _run(() async {
      _nextHandle++;
      final handle = await _bridge.pairingHandle(
        identifier: 'ui.pairing.$_nextHandle',
      );
      try {
        await configure(handle);
        _activeHandle = handle;
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
    await cancel();
  }
}
