import 'package:media_kit/media_kit.dart';
import 'package:media_kit_video/media_kit_video.dart';

class PlaybackUiController {
  PlaybackUiController(this._onChanged, this._onError) {
    _player = Player();
    _videoController = VideoController(_player);
  }

  final void Function() _onChanged;
  final void Function(Object) _onError;
  late final Player _player;
  late final VideoController _videoController;
  String? _filePath;
  bool _busy = false;

  VideoController get videoController => _videoController;
  String? get filePath => _filePath;
  bool get busy => _busy;

  Future<void> open(String filePath, Duration position) async {
    if (_busy) {
      return;
    }
    _busy = true;
    _onChanged();
    try {
      await _player.open(Media(filePath), play: false);
      await _player.seek(position);
      await _player.play();
      _filePath = filePath;
    } catch (error) {
      _onError(error);
    } finally {
      _busy = false;
      _onChanged();
    }
  }

  Future<void> dispose() {
    return _player.dispose();
  }
}
