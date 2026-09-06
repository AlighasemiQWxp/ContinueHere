enum UiFilePreviewKind { image, video }

class UiFilePreview {
  const UiFilePreview({required this.path, required this.kind});

  final String path;
  final UiFilePreviewKind kind;
}

abstract final class UiFileSupport {
  static const _imageExtensions = {'png', 'jpg', 'jpeg', 'gif', 'webp', 'bmp'};
  static const _videoExtensions = {'mp4', 'm4v', 'mkv', 'webm', 'mov', 'avi'};
  static const _unsafeExtensions = {
    'apk',
    'app',
    'appimage',
    'appx',
    'bat',
    'cmd',
    'com',
    'deb',
    'desktop',
    'dll',
    'exe',
    'ipa',
    'jar',
    'js',
    'jse',
    'lnk',
    'msi',
    'msix',
    'ps1',
    'psm1',
    'reg',
    'rpm',
    'scr',
    'sh',
    'url',
    'vbe',
    'vbs',
    'wsf',
    'wsh',
  };

  static UiFilePreviewKind? previewKind(String path) {
    final extension = _extensionOf(path);
    if (_imageExtensions.contains(extension)) {
      return UiFilePreviewKind.image;
    }
    if (_videoExtensions.contains(extension)) {
      return UiFilePreviewKind.video;
    }
    return null;
  }

  static bool isUnsafeToOpen(String path) {
    return _unsafeExtensions.contains(_extensionOf(path));
  }

  static String _extensionOf(String path) {
    final separator = path.lastIndexOf('.');
    if (separator < 0 || separator == path.length - 1) {
      return '';
    }
    return path.substring(separator + 1).toLowerCase();
  }
}
