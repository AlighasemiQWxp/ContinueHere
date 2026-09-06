import 'dart:io';

import 'package:media_kit/media_kit.dart';
import 'package:media_kit_video/media_kit_video.dart';

import '../ui_file_support.dart';

typedef FilePreviewChangedDelegate = void Function();
typedef FilePreviewErrorDelegate = void Function(Object error);

class FilePreviewUiController {
  FilePreviewUiController(this._onChanged, this._onError) {
    _player = Player();
    _videoController = VideoController(_player);
  }

  final FilePreviewChangedDelegate _onChanged;
  final FilePreviewErrorDelegate _onError;
  late final Player _player;
  late final VideoController _videoController;
  UiFilePreview? _preview;
  bool _busy = false;

  VideoController get videoController => _videoController;
  UiFilePreview? get preview => _preview;
  bool get busy => _busy;

  Future<bool> open(String path, {Duration position = Duration.zero}) async {
    if (_busy) {
      return false;
    }
    final kind = UiFileSupport.previewKind(path);
    if (kind == null) {
      _onError(FormatException('This file cannot be previewed in the app.'));
      return false;
    }
    _busy = true;
    _onChanged();
    try {
      if (!await File(path).exists()) {
        _onError(StateError('The transferred file no longer exists.'));
        return false;
      }
      await _player.stop();
      if (kind == UiFilePreviewKind.video) {
        await _player.open(Media(path), play: false);
        await _player.seek(position);
        await _player.play();
      }
      _preview = UiFilePreview(path: path, kind: kind);
      return true;
    } catch (error) {
      _onError(error);
      return false;
    } finally {
      _busy = false;
      _onChanged();
    }
  }

  Future<void> close() async {
    if (_busy || _preview == null) {
      return;
    }
    _busy = true;
    _onChanged();
    try {
      await _player.stop();
    } catch (error) {
      _onError(error);
    } finally {
      _preview = null;
      _busy = false;
      _onChanged();
    }
  }

  Future<void> dispose() {
    return _player.dispose();
  }
}
