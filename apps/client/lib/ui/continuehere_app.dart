import 'package:flutter/material.dart';

import '../src/rust/api/models.dart';
import 'screens/devices_screen.dart';
import 'screens/send_screen.dart';
import 'screens/settings_screen.dart';
import 'screens/transfers_screen.dart';
import 'ui_manager.dart';
import 'ui_motion.dart';
import 'ui_strings.dart';
import 'ui_transition.dart';
import 'widgets/file_preview_overlay.dart';
import 'widgets/navigation_badge.dart';
import 'widgets/ui_page_transition.dart';

class ContinueHereApp extends StatefulWidget {
  const ContinueHereApp({required this.uiManager, super.key});

  final UiManager uiManager;

  @override
  State<ContinueHereApp> createState() => _ContinueHereAppState();
}

class _ContinueHereAppState extends State<ContinueHereApp> {
  late final AppLifecycleListener _lifecycle;

  @override
  void initState() {
    super.initState();
    _lifecycle = AppLifecycleListener(onDetach: widget.uiManager.shutdown);
  }

  @override
  void dispose() {
    _lifecycle.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return AnimatedBuilder(
      animation: widget.uiManager,
      builder: (context, _) {
        final language =
            widget.uiManager.settings?.language ?? UiLanguage.english;
        final strings = UiStrings(language);
        var direction = TextDirection.ltr;
        if (widget.uiManager.isRtl) {
          direction = TextDirection.rtl;
        }
        return MaterialApp(
          debugShowCheckedModeBanner: false,
          title: strings.appTitle,
          theme: _theme(),
          home: Directionality(
            textDirection: direction,
            child: _ClientShell(uiManager: widget.uiManager, strings: strings),
          ),
        );
      },
    );
  }

  ThemeData _theme() {
    final colors = ColorScheme.fromSeed(
      seedColor: const Color(0xFF3D5AFE),
      brightness: Brightness.dark,
      surface: const Color(0xFF11141B),
    );
    return ThemeData(
      colorScheme: colors,
      scaffoldBackgroundColor: const Color(0xFF0B0E13),
      useMaterial3: true,
      cardTheme: const CardThemeData(
        margin: EdgeInsets.zero,
        clipBehavior: Clip.antiAlias,
      ),
      inputDecorationTheme: const InputDecorationTheme(
        border: OutlineInputBorder(),
      ),
    );
  }
}

class _ClientShell extends StatelessWidget {
  const _ClientShell({required this.uiManager, required this.strings});

  final UiManager uiManager;
  final UiStrings strings;

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      body: Stack(
        children: [
          SafeArea(
            child: LayoutBuilder(
              builder: (context, constraints) {
                final content = _content(context);
                if (constraints.maxWidth >= 760) {
                  return Row(
                    children: [
                      NavigationRail(
                        extended: constraints.maxWidth >= 1080,
                        selectedIndex: _selectedIndex(),
                        onDestinationSelected: _select,
                        leading: Padding(
                          padding: const EdgeInsets.symmetric(vertical: 16),
                          child: Text(
                            strings.appTitle,
                            style: Theme.of(context).textTheme.titleLarge,
                          ),
                        ),
                        destinations: _railDestinations(),
                      ),
                      const VerticalDivider(width: 1),
                      Expanded(child: content),
                    ],
                  );
                }
                return Column(
                  children: [
                    Expanded(child: content),
                    NavigationBar(
                      selectedIndex: _selectedIndex(),
                      onDestinationSelected: _select,
                      destinations: _barDestinations(),
                    ),
                  ],
                );
              },
            ),
          ),
          Positioned.fill(child: _filePreview(context)),
        ],
      ),
    );
  }

  Widget _filePreview(BuildContext context) {
    Widget overlay = const SizedBox.shrink(key: ValueKey('no-file-preview'));
    final preview = uiManager.filePreview;
    if (uiManager.filePreviewVisible && preview != null) {
      overlay = FilePreviewOverlay(
        key: ValueKey(preview.path),
        preview: preview,
        uiManager: uiManager,
        strings: strings,
      );
    }
    return AnimatedSwitcher(
      duration: UiMotion.media(context),
      switchInCurve: UiMotion.enterCurve,
      switchOutCurve: UiMotion.exitCurve,
      transitionBuilder: (child, animation) {
        return FadeTransition(opacity: animation, child: child);
      },
      child: overlay,
    );
  }

  Widget _content(BuildContext context) {
    Widget screen;
    switch (uiManager.destination) {
      case UiDestination.devices:
        screen = DevicesScreen(uiManager: uiManager, strings: strings);
        break;
      case UiDestination.send:
        screen = SendScreen(uiManager: uiManager, strings: strings);
        break;
      case UiDestination.transfers:
        screen = TransfersScreen(uiManager: uiManager, strings: strings);
        break;
      case UiDestination.settings:
        screen = SettingsScreen(uiManager: uiManager, strings: strings);
        break;
    }
    final error = uiManager.errorMessage;
    Widget errorBanner = const SizedBox.shrink(key: ValueKey('no-error'));
    if (error != null) {
      errorBanner = MaterialBanner(
        key: ValueKey(error),
        content: Text(error),
        actions: [
          TextButton(
            onPressed: uiManager.clearError,
            child: Text(strings.dismiss),
          ),
        ],
      );
    }
    return Column(
      children: [
        AnimatedSize(
          duration: UiMotion.quick(context),
          alignment: Alignment.topCenter,
          child: AnimatedSwitcher(
            duration: UiMotion.quick(context),
            switchInCurve: UiMotion.enterCurve,
            switchOutCurve: UiMotion.exitCurve,
            child: errorBanner,
          ),
        ),
        Expanded(
          child: UiPageTransition(
            isRtl: uiManager.isRtl,
            child: KeyedSubtree(
              key: ValueKey(uiManager.destination),
              child: screen,
            ),
          ),
        ),
      ],
    );
  }

  int _selectedIndex() {
    switch (uiManager.destination) {
      case UiDestination.devices:
        return 0;
      case UiDestination.send:
        return 1;
      case UiDestination.transfers:
        return 2;
      case UiDestination.settings:
        return 3;
    }
  }

  void _select(int index) {
    uiManager.show(UiDestination.values[index]);
  }

  List<NavigationRailDestination> _railDestinations() {
    return [
      NavigationRailDestination(
        icon: _navigationIcon(Icons.devices_outlined, UiDestination.devices),
        selectedIcon: _navigationIcon(Icons.devices, UiDestination.devices),
        label: Text(strings.devices),
      ),
      NavigationRailDestination(
        icon: _navigationIcon(Icons.send_outlined, UiDestination.send),
        selectedIcon: _navigationIcon(Icons.send, UiDestination.send),
        label: Text(strings.send),
      ),
      NavigationRailDestination(
        icon: _navigationIcon(
          Icons.swap_horiz_outlined,
          UiDestination.transfers,
        ),
        selectedIcon: _navigationIcon(
          Icons.swap_horiz,
          UiDestination.transfers,
        ),
        label: Text(strings.transfers),
      ),
      NavigationRailDestination(
        icon: _navigationIcon(Icons.settings_outlined, UiDestination.settings),
        selectedIcon: _navigationIcon(Icons.settings, UiDestination.settings),
        label: Text(strings.settings),
      ),
    ];
  }

  List<NavigationDestination> _barDestinations() {
    return [
      NavigationDestination(
        icon: _navigationIcon(Icons.devices, UiDestination.devices),
        label: strings.devices,
      ),
      NavigationDestination(
        icon: _navigationIcon(Icons.send, UiDestination.send),
        label: strings.send,
      ),
      NavigationDestination(
        icon: _navigationIcon(Icons.swap_horiz, UiDestination.transfers),
        label: strings.transfers,
      ),
      NavigationDestination(
        icon: _navigationIcon(Icons.settings, UiDestination.settings),
        label: strings.settings,
      ),
    ];
  }

  Widget _navigationIcon(IconData icon, UiDestination destination) {
    return NavigationBadge(
      visible: uiManager.hasNotification(destination),
      semanticsLabel: strings.newActivity,
      child: Icon(icon),
    );
  }
}
