import 'dart:io';

import 'package:flutter/material.dart';
import 'package:media_kit_video/media_kit_video.dart';

import '../ui_file_support.dart';
import '../ui_manager.dart';
import '../ui_motion.dart';
import '../ui_strings.dart';

class FilePreviewOverlay extends StatelessWidget {
  const FilePreviewOverlay({
    required this.uiManager,
    required this.strings,
    required this.preview,
    super.key,
  });

  final UiManager uiManager;
  final UiStrings strings;
  final UiFilePreview preview;

  @override
  Widget build(BuildContext context) {
    return Stack(
      children: [
        ModalBarrier(
          dismissible: true,
          onDismiss: uiManager.closeFilePreview,
          color: Colors.black.withValues(alpha: 0.72),
        ),
        SafeArea(
          child: Center(
            child: Dialog(
              insetPadding: const EdgeInsets.all(24),
              child: ConstrainedBox(
                constraints: const BoxConstraints(maxWidth: 1100),
                child: Padding(
                  padding: const EdgeInsets.all(16),
                  child: Column(
                    mainAxisSize: MainAxisSize.min,
                    children: [
                      Row(
                        children: [
                          Expanded(
                            child: Text(
                              _fileName(preview.path),
                              maxLines: 1,
                              overflow: TextOverflow.ellipsis,
                              style: Theme.of(context).textTheme.titleMedium,
                            ),
                          ),
                          IconButton(
                            onPressed: uiManager.closeFilePreview,
                            tooltip: strings.close,
                            icon: const Icon(Icons.close),
                          ),
                        ],
                      ),
                      const SizedBox(height: 12),
                      Flexible(
                        child: AspectRatio(
                          aspectRatio: 16 / 9,
                          child: _viewer(context, preview),
                        ),
                      ),
                    ],
                  ),
                ),
              ),
            ),
          ),
        ),
      ],
    );
  }

  Widget _viewer(BuildContext context, UiFilePreview preview) {
    if (preview.kind == UiFilePreviewKind.image) {
      return InteractiveViewer(
        maxScale: 5,
        child: Center(
          child: Image.file(
            File(preview.path),
            fit: BoxFit.contain,
            frameBuilder: (context, child, frame, wasSynchronouslyLoaded) {
              if (wasSynchronouslyLoaded) {
                return child;
              }
              var opacity = 0.0;
              if (frame != null) {
                opacity = 1.0;
              }
              return AnimatedOpacity(
                opacity: opacity,
                duration: UiMotion.media(context),
                curve: UiMotion.enterCurve,
                child: child,
              );
            },
            errorBuilder: (context, error, stackTrace) {
              return Center(child: Text(strings.mediaLoadFailed));
            },
          ),
        ),
      );
    }
    return Video(controller: uiManager.filePreviewVideo);
  }

  String _fileName(String path) {
    final normalized = path.replaceAll('\\', '/');
    return normalized.substring(normalized.lastIndexOf('/') + 1);
  }
}
