import 'package:flutter/material.dart';
import 'package:media_kit_video/media_kit_video.dart';

import '../ui_manager.dart';
import '../ui_strings.dart';
import 'screen_frame.dart';

class PlaybackScreen extends StatelessWidget {
  const PlaybackScreen({
    required this.uiManager,
    required this.strings,
    super.key,
  });

  final UiManager uiManager;
  final UiStrings strings;

  @override
  Widget build(BuildContext context) {
    Widget content = Center(child: Text(strings.noVideo));
    if (uiManager.playbackFile != null) {
      content = AspectRatio(
        aspectRatio: 16 / 9,
        child: Video(controller: uiManager.playbackVideo),
      );
    }
    return ScreenFrame(
      title: strings.playback,
      child: Card(
        child: Padding(padding: const EdgeInsets.all(12), child: content),
      ),
    );
  }
}
