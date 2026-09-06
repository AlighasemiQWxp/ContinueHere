import 'dart:async';

import '../../platform/platform_manager.dart';
import '../../src/rust/api/events.dart';
import '../../src/rust/api/manager.dart';
import '../../src/rust/api/models.dart';
import '../ui_notifications.dart';

class SettingsUiController {
  SettingsUiController(
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
  StreamSubscription<SettingsUiEvent>? _events;
  UiSettingsSnapshot? _snapshot;
  bool _busy = false;

  UiSettingsSnapshot? get snapshot => _snapshot;
  bool get busy => _busy;

  Future<void> start() async {
    _events = _bridge.watchSettings().listen(
      (_) => unawaited(_refreshFromEvent()),
      onError: _onError,
    );
    await refresh();
  }

  Future<void> _refreshFromEvent() async {
    await refresh();
    _onActivity();
  }

  Future<void> changeDisplayName(String displayName) {
    return _run(() => _bridge.setDisplayName(displayName: displayName));
  }

  Future<void> chooseDefaultDirectory() async {
    final directory = await _platform.selectDirectory();
    if (directory == null) {
      return;
    }
    await _run(() => _bridge.setDefaultTransferDirectory(directory: directory));
  }

  Future<void> changeLanguage(UiLanguage language) {
    return _run(() => _bridge.setLanguage(selectedLanguage: language));
  }

  Future<void> refresh() async {
    try {
      _snapshot = await _bridge.settingsSnapshot();
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
  }
}
