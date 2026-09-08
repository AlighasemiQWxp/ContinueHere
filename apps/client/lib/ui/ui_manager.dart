import 'package:flutter/foundation.dart';
import 'package:media_kit_video/media_kit_video.dart';

import '../platform/platform_manager.dart';
import '../src/rust/api/manager.dart';
import '../src/rust/api/models.dart';
import '../src/rust/api/activity.dart';
import 'ui_controller.dart';
import 'ui_file_support.dart';
import 'ui_notifications.dart';
import 'ui_transition.dart';

class UiManager extends ChangeNotifier {
  UiManager._(this._bridge, this._platform) {
    _transition = UiTransition(notifyListeners);
    _notifications = UiNotifications(notifyListeners);
    _controller = UiController(
      _bridge,
      _platform,
      notifyListeners,
      _receiveError,
      _receiveActivity,
      _receiveHistoryActivity,
    );
  }

  final UiBridge _bridge;
  final PlatformManager _platform;
  late final UiController _controller;
  late final UiNotifications _notifications;
  late final UiTransition _transition;
  Object? _error;
  bool _started = false;
  bool _shuttingDown = false;

  static Future<UiManager> start(
    UiBridge bridge,
    PlatformManager platform,
  ) async {
    final manager = UiManager._(bridge, platform);
    try {
      await manager._controller.start();
      manager._started = true;
      manager.notifyListeners();
      return manager;
    } catch (_) {
      try {
        await manager._controller.dispose();
      } finally {
        try {
          await bridge.shutdown();
        } finally {
          bridge.dispose();
        }
      }
      rethrow;
    }
  }

  UiDestination get destination => _transition.destination;
  UiDevicesSnapshot? get devices => _controller.devices.snapshot;
  UiPairingSnapshot? get pairing => _controller.pairing.snapshot;
  UiHandoffSnapshot? get handoffs => _controller.handoff.snapshot;
  List<UiFileTransfer> get transfers => _controller.transfer.transfers;
  UiSettingsSnapshot? get settings => _controller.settings.snapshot;
  List<UiActivity> get history => _controller.activity.entries;
  String? get historyStorageError => _controller.activity.storageError;
  String? get historyDevice => _transition.historyDevice;
  bool get historyBusy => _controller.activity.busy;
  VideoController get filePreviewVideo =>
      _controller.filePreview.videoController;
  UiFilePreview? get filePreview => _controller.filePreview.preview;
  bool get filePreviewVisible => _transition.filePreviewVisible;
  bool get started => _started;
  bool get busy {
    return _controller.devices.busy ||
        _controller.pairing.busy ||
        _controller.handoff.busy ||
        _controller.transfer.busy ||
        _controller.settings.busy ||
        _controller.activity.busy ||
        _controller.filePreview.busy;
  }

  String? get errorMessage {
    final error = _error;
    if (error == null) {
      return null;
    }
    return error.toString();
  }

  bool get isRtl {
    return settings?.textDirection == UiTextDirection.rightToLeft;
  }

  bool hasNotification(UiDestination destination) {
    return _notifications.has(destination);
  }

  void show(UiDestination destination) {
    if (destination == UiDestination.history) {
      _transition.showHistoryDevice(null);
    }
    _notifications.open(destination);
    _transition.show(destination);
  }

  bool hasDeviceNotification(String deviceId) =>
      _notifications.hasDevice(deviceId);

  void showDeviceHistory(String deviceId) {
    _notifications.openDevice(deviceId);
    _transition.showHistoryDevice(deviceId);
  }

  void showHistoryDevices() => _transition.showHistoryDevice(null);

  Future<void> retryActivity(UiActivity activity) =>
      _controller.activity.retry(activity);
  Future<void> removeActivity(UiActivity activity) async {
    await _controller.activity.remove(activity);
    _notifications.retainDevices(
      history.map((entry) => entry.deviceId).toSet(),
    );
    notifyListeners();
  }

  Future<void> clearHistory() async {
    await _controller.activity.clear();
    _notifications.retainDevices(
      history.map((entry) => entry.deviceId).toSet(),
    );
    notifyListeners();
  }

  bool canOpenActivity(UiActivity activity) {
    return activity.direction == UiActivityDirection.incoming &&
        (activity.status == UiActivityStatus.completed ||
            activity.status == UiActivityStatus.delivered) &&
        (activity.filePath != null || activity.url != null);
  }

  Future<void> openActivity(UiActivity activity) async {
    if (!canOpenActivity(activity)) {
      return;
    }
    final path = activity.filePath;
    if (path == null) {
      final url = activity.url;
      if (url != null) {
        try {
          await _platform.openExternalUrl(url);
        } catch (error) {
          _receiveError(error);
        }
      }
      return;
    }
    await _openReceivedFile(
      path,
      position: Duration(milliseconds: activity.positionMillis.toInt()),
    );
  }

  void clearError() {
    if (_error == null) {
      return;
    }
    _error = null;
    notifyListeners();
  }

  Future<void> addManualEndpoint(String host, String portText) async {
    final hostValue = host.trim();
    final port = int.tryParse(portText.trim());
    if (hostValue.isEmpty || port == null || port < 1 || port > 65535) {
      _receiveError(FormatException('Enter a valid host and port.'));
      return;
    }
    await _controller.devices.addManualEndpoint(hostValue, port);
  }

  Future<void> pairWith(UiDiscoveryCandidate candidate) {
    return _controller.pairing.pair(candidate);
  }

  Future<void> receivePairing() {
    return _controller.pairing.receive();
  }

  Future<void> approvePairing() {
    return _controller.pairing.approve();
  }

  Future<void> rejectPairing() {
    return _controller.pairing.reject();
  }

  Future<void> cancelPairing() {
    return _controller.pairing.cancel();
  }

  Future<void> connect(UiTrustedDevice device) {
    return _controller.devices.connect(device);
  }

  Future<void> disconnect(UiTrustedDevice device) {
    return _controller.devices.disconnect(device);
  }

  Future<void> forget(UiTrustedDevice device) {
    return _controller.devices.removeTrustedDevice(device);
  }

  bool isConnected(String deviceId) {
    return _controller.devices.isConnected(deviceId);
  }

  Future<void> sendUrl(String deviceId, String url) async {
    final value = url.trim();
    if (!_canSend(deviceId, value)) {
      return;
    }
    await _controller.handoff.sendUrl(deviceId, value);
  }

  Future<void> sendYoutube(
    String deviceId,
    String url,
    String positionText,
  ) async {
    final value = url.trim();
    final position = _playbackPosition(positionText);
    if (!_canSend(deviceId, value) || position == null) {
      return;
    }
    await _controller.handoff.sendYoutube(deviceId, value, position);
  }

  Future<void> sendLocalVideo(String deviceId, String positionText) async {
    final position = _playbackPosition(positionText);
    if (deviceId.isEmpty || position == null) {
      _receiveError(FormatException('Choose a device and playback position.'));
      return;
    }
    final source = await _platform.selectVideo();
    if (source == null) {
      return;
    }
    await _controller.handoff.sendLocalVideo(deviceId, source, position);
  }

  Future<void> sendFile(String deviceId) async {
    if (deviceId.isEmpty) {
      _receiveError(FormatException('Choose a destination device.'));
      return;
    }
    await _controller.transfer.sendFile(deviceId);
  }

  Future<void> acceptTransfer(
    UiFileTransfer transfer, {
    bool chooseFolder = false,
  }) {
    return _controller.transfer.accept(transfer, chooseFolder: chooseFolder);
  }

  Future<void> rejectTransfer(UiFileTransfer transfer) {
    return _controller.transfer.reject(transfer);
  }

  Future<void> removeTransfer(UiFileTransfer transfer) {
    return _controller.transfer.remove(transfer);
  }

  bool canOpenTransfer(UiFileTransfer transfer) {
    final destination = transfer.destination;
    return transfer.direction == UiFileTransferDirection.incoming &&
        transfer.state == UiFileTransferState.completed &&
        destination != null;
  }

  Future<void> openTransfer(UiFileTransfer transfer) async {
    final destination = transfer.destination;
    if (!canOpenTransfer(transfer) || destination == null) {
      _receiveError(StateError('This transferred file cannot be opened.'));
      return;
    }
    await _openReceivedFile(destination);
  }

  Future<void> _openReceivedFile(
    String destination, {
    Duration position = Duration.zero,
  }) async {
    if (UiFileSupport.isUnsafeToOpen(destination)) {
      _receiveError(
        StateError('Executable and script files cannot be opened here.'),
      );
      return;
    }
    final previewKind = UiFileSupport.previewKind(destination);
    if (previewKind != null) {
      final opened = await _controller.filePreview.open(
        destination,
        position: position,
      );
      if (opened) {
        _transition.showFilePreview();
      }
      return;
    }
    try {
      await _platform.openExternalFile(destination);
    } catch (error) {
      _receiveError(error);
    }
  }

  Future<void> closeFilePreview() async {
    _transition.hideFilePreview();
    await _controller.filePreview.close();
  }

  Future<void> openIncoming(UiIncomingHandoff handoff) async {
    final filePath = handoff.payload.filePath;
    if (filePath != null) {
      final position = Duration(
        milliseconds: handoff.payload.playbackPositionMillis.toInt(),
      );
      await _openReceivedFile(filePath, position: position);
      return;
    }
    await _controller.handoff.openIncoming(handoff);
  }

  Future<void> removeIncoming(UiIncomingHandoff handoff) {
    return _controller.handoff.removeIncoming(handoff);
  }

  Future<void> changeDisplayName(String displayName) async {
    final value = displayName.trim();
    if (value.isEmpty) {
      _receiveError(FormatException('Enter a device name.'));
      return;
    }
    await _controller.settings.changeDisplayName(value);
  }

  Future<void> chooseDefaultDirectory() {
    return _controller.settings.chooseDefaultDirectory();
  }

  Future<void> changeLanguage(UiLanguage language) {
    return _controller.settings.changeLanguage(language);
  }

  bool _canSend(String deviceId, String value) {
    if (deviceId.isEmpty || value.isEmpty) {
      _receiveError(FormatException('Choose a device and enter the content.'));
      return false;
    }
    if (!isConnected(deviceId)) {
      _receiveError(
        StateError('Connect to the trusted device before sending.'),
      );
      return false;
    }
    return true;
  }

  Duration? _playbackPosition(String text) {
    final value = text.trim();
    if (value.isEmpty) {
      return Duration.zero;
    }
    final parts = value.split(':');
    if (parts.length > 3) {
      _receiveError(FormatException('Use seconds, MM:SS, or HH:MM:SS.'));
      return null;
    }
    var seconds = 0;
    for (final part in parts) {
      final number = int.tryParse(part);
      if (number == null || number < 0) {
        _receiveError(FormatException('Enter a valid playback position.'));
        return null;
      }
      seconds = seconds * 60 + number;
    }
    return Duration(seconds: seconds);
  }

  void _receiveError(Object error) {
    _error = error;
    notifyListeners();
  }

  void _receiveActivity(UiDestination destination) {
    _notifications.receive(destination, _transition.destination);
  }

  void _receiveHistoryActivity(String deviceId) {
    _notifications.retainDevices(
      history.map((entry) => entry.deviceId).toSet(),
    );
    _notifications.receiveHistory(
      deviceId,
      _transition.destination,
      _transition.historyDevice,
    );
  }

  Future<void> shutdown() async {
    if (_shuttingDown) {
      return;
    }
    _shuttingDown = true;
    try {
      await _controller.dispose();
    } finally {
      try {
        await _bridge.shutdown();
      } finally {
        _bridge.dispose();
        super.dispose();
      }
    }
  }
}
