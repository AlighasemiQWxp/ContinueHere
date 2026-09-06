import 'package:file_selector/file_selector.dart';
import 'package:url_launcher/url_launcher.dart';

class PlatformManager {
  static const _videoTypes = XTypeGroup(
    label: 'Video files',
    extensions: ['mp4', 'm4v', 'mkv', 'webm', 'mov', 'avi'],
  );

  Future<String?> selectVideo() async {
    final file = await openFile(acceptedTypeGroups: [_videoTypes]);
    return file?.path;
  }

  Future<String?> selectFile() async {
    final file = await openFile();
    return file?.path;
  }

  Future<String?> selectDirectory() {
    return getDirectoryPath();
  }

  Future<void> openExternalUrl(String value) async {
    final uri = Uri.parse(value);
    final opened = await launchUrl(uri, mode: LaunchMode.externalApplication);
    if (!opened) {
      throw StateError('The URL could not be opened.');
    }
  }
}
