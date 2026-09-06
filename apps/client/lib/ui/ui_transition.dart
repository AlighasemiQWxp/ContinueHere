enum UiDestination { devices, send, transfers, settings, playback }

class UiTransition {
  UiTransition(this._onChanged);

  final void Function() _onChanged;
  UiDestination _destination = UiDestination.devices;

  UiDestination get destination => _destination;

  void show(UiDestination destination) {
    if (_destination == destination) {
      return;
    }
    _destination = destination;
    _onChanged();
  }
}
