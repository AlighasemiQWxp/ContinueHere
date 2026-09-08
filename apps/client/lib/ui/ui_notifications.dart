import 'ui_transition.dart';

typedef UiNotificationsChangedDelegate = void Function();
typedef UiDestinationActivityDelegate = void Function(
  UiDestination destination,
);
typedef UiActivityDelegate = void Function();
typedef UiDeviceActivityDelegate = void Function(String deviceId);

class UiNotifications {
  UiNotifications(this._onChanged);

  final UiNotificationsChangedDelegate _onChanged;
  final Set<UiDestination> _unread = {};
  final Set<String> _unreadDevices = {};

  bool hasDevice(String deviceId) => _unreadDevices.contains(deviceId);

  void receiveHistory(
    String deviceId,
    UiDestination activeDestination,
    String? activeHistoryDevice,
  ) {
    if (activeDestination == UiDestination.history &&
        activeHistoryDevice == deviceId) {
      return;
    }
    final changed = _unreadDevices.add(deviceId);
    receive(UiDestination.history, activeDestination);
    if (changed) {
      _onChanged();
    }
  }

  void openDevice(String deviceId) {
    if (_unreadDevices.remove(deviceId)) {
      _onChanged();
    }
  }

  void retainDevices(Set<String> deviceIds) {
    _unreadDevices.removeWhere((id) => !deviceIds.contains(id));
    if (_unreadDevices.isEmpty) {
      _unread.remove(UiDestination.history);
    }
  }

  bool has(UiDestination destination) {
    return _unread.contains(destination);
  }

  void receive(UiDestination destination, UiDestination activeDestination) {
    if (destination == activeDestination || _unread.contains(destination)) {
      return;
    }
    _unread.add(destination);
    _onChanged();
  }

  void open(UiDestination destination) {
    if (!_unread.remove(destination)) {
      return;
    }
    _onChanged();
  }
}
