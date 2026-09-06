enum UiDestination { devices, send, transfers, settings }

typedef UiTransitionChangedDelegate = void Function();

class UiTransition {
  UiTransition(this._onChanged);

  final UiTransitionChangedDelegate _onChanged;
  UiDestination _destination = UiDestination.devices;
  bool _filePreviewVisible = false;

  UiDestination get destination => _destination;
  bool get filePreviewVisible => _filePreviewVisible;

  void show(UiDestination destination) {
    if (_destination == destination) {
      return;
    }
    _destination = destination;
    _onChanged();
  }

  void showFilePreview() {
    if (_filePreviewVisible) {
      return;
    }
    _filePreviewVisible = true;
    _onChanged();
  }

  void hideFilePreview() {
    if (!_filePreviewVisible) {
      return;
    }
    _filePreviewVisible = false;
    _onChanged();
  }
}
