import 'package:flutter/foundation.dart';
import 'package:media_kit_video/media_kit_video.dart';

import '../platform/platform_manager.dart';
import '../src/rust/api/manager.dart';
import '../src/rust/api/models.dart';
import 'ui_controller.dart';
import 'ui_transition.dart';

class UiManager extends ChangeNotifier {
  UiManager._(this._bridge, this._platform) {
    _transition = UiTransition(notifyListeners);
    _controller = UiController(
      _bridge,
      _platform,
      notifyListeners,
      _receiveError,
    );
  }

  final UiBridge _bridge;
  final PlatformManager _platform;
  late final UiController _controller;
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
  VideoController get playbackVideo => _controller.playback.videoController;
  String? get playbackFile => _controller.playback.filePath;
  bool get started => _started;
  bool get busy {
    return _controller.devices.busy ||
        _controller.pairing.busy ||
        _controller.handoff.busy ||
        _controller.transfer.busy ||
        _controller.settings.busy ||
        _controller.playback.busy;
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

  void show(UiDestination destination) {
    _transition.show(destination);
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

  Future<void> openIncoming(UiIncomingHandoff handoff) async {
    final filePath = handoff.payload.filePath;
    if (filePath != null) {
      final position = Duration(
        milliseconds: handoff.payload.playbackPositionMillis.toInt(),
      );
      await _controller.playback.open(filePath, position);
      _transition.show(UiDestination.playback);
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
