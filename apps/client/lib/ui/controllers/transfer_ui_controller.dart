import 'dart:async';

import '../../platform/platform_manager.dart';
import '../../src/rust/api/events.dart';
import '../../src/rust/api/handles.dart';
import '../../src/rust/api/manager.dart';
import '../../src/rust/api/models.dart';
import '../ui_notifications.dart';

class TransferUiController {
  TransferUiController(
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
  final List<UiFileTransferHandle> _handles = [];
  StreamSubscription<TransferUiEvent>? _events;
  List<UiFileTransfer> _transfers = [];
  int _nextHandle = 0;
  bool _busy = false;

  List<UiFileTransfer> get transfers => List.unmodifiable(_transfers);
  bool get busy => _busy;

  Future<void> start() async {
    _events = _bridge.watchTransfers().listen(
      (_) => unawaited(_refreshFromEvent()),
      onError: _onError,
    );
    await refresh();
  }

  Future<void> _refreshFromEvent() async {
    await refresh();
    _onActivity();
  }

  Future<void> sendFile(String deviceId) async {
    final source = await _platform.selectFile();
    if (source == null) {
      return;
    }
    await _run(() async {
      _nextHandle++;
      final handle = await _bridge.fileTransferHandle(
        identifier: 'ui.transfer.$_nextHandle',
      );
      try {
        await handle.prepare(deviceId: deviceId, source: source);
        await handle.useHandle();
        _handles.add(handle);
      } catch (_) {
        await handle.release();
        handle.dispose();
        rethrow;
      }
    });
  }

  Future<void> accept(
    UiFileTransfer transfer, {
    bool chooseFolder = false,
  }) async {
    String? directory;
    if (chooseFolder) {
      directory = await _platform.selectDirectory();
      if (directory == null) {
        return;
      }
    }
    await _run(
      () => _bridge.acceptTransfer(
        transferId: transfer.id,
        selectedDirectory: directory,
      ),
    );
  }

  Future<void> reject(UiFileTransfer transfer) {
    return _run(() => _bridge.rejectTransfer(transferId: transfer.id));
  }

  Future<void> remove(UiFileTransfer transfer) {
    return _run(() => _bridge.removeTransfer(transferId: transfer.id));
  }

  Future<void> refresh() async {
    try {
      _transfers = await _bridge.transfers();
      await _releaseFinishedHandles();
      _onChanged();
    } catch (error) {
      _onError(error);
    }
  }

  Future<void> _releaseFinishedHandles() async {
    for (final handle in List<UiFileTransferHandle>.from(_handles)) {
      final transfer = await handle.transfer();
      if (!_handles.contains(handle) ||
          transfer == null ||
          !_isFinished(transfer.state)) {
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

  bool _isFinished(UiFileTransferState state) {
    return state == UiFileTransferState.completed ||
        state == UiFileTransferState.rejected ||
        state == UiFileTransferState.cancelled ||
        state == UiFileTransferState.failed;
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
