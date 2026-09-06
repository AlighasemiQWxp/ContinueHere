import 'package:flutter/material.dart';
import 'package:media_kit/media_kit.dart';
import 'package:path_provider/path_provider.dart';

import 'platform/platform_manager.dart';
import 'src/rust/api/manager.dart';
import 'src/rust/frb_generated.dart';
import 'ui/continuehere_app.dart';
import 'ui/ui_manager.dart';

Future<void> main() async {
  WidgetsFlutterBinding.ensureInitialized();
  MediaKit.ensureInitialized();

  try {
    await RustLib.init();
    final supportDirectory = await getApplicationSupportDirectory();
    await supportDirectory.create(recursive: true);
    final bridge = await UiBridge.start(
      projectDirectory: supportDirectory.path,
    );
    final uiManager = await UiManager.start(bridge, PlatformManager());
    runApp(ContinueHereApp(uiManager: uiManager));
  } catch (error) {
    runApp(_StartupFailureApp(message: error.toString()));
  }
}

class _StartupFailureApp extends StatelessWidget {
  const _StartupFailureApp({required this.message});

  final String message;

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      debugShowCheckedModeBanner: false,
      theme: ThemeData.dark(useMaterial3: true),
      home: Scaffold(
        body: Center(
          child: Padding(
            padding: const EdgeInsets.all(32),
            child: SelectableText(message, textAlign: TextAlign.center),
          ),
        ),
      ),
    );
  }
}
