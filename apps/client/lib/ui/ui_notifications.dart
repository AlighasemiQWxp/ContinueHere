import 'ui_transition.dart';

typedef UiNotificationsChangedDelegate = void Function();
typedef UiDestinationActivityDelegate = void Function(
  UiDestination destination,
);
typedef UiActivityDelegate = void Function();

class UiNotifications {
  UiNotifications(this._onChanged);

  final UiNotificationsChangedDelegate _onChanged;
  final Set<UiDestination> _unread = {};

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
