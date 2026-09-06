import 'package:flutter/material.dart';

import '../src/rust/api/models.dart';
import 'screens/devices_screen.dart';
import 'screens/playback_screen.dart';
import 'screens/send_screen.dart';
import 'screens/settings_screen.dart';
import 'screens/transfers_screen.dart';
import 'ui_manager.dart';
import 'ui_strings.dart';
import 'ui_transition.dart';

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
      body: SafeArea(
        child: LayoutBuilder(
          builder: (context, constraints) {
            final content = _content();
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
    );
  }

  Widget _content() {
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
      case UiDestination.playback:
        screen = PlaybackScreen(uiManager: uiManager, strings: strings);
        break;
    }
    final error = uiManager.errorMessage;
    return Column(
      children: [
        if (error != null)
          MaterialBanner(
            content: Text(error),
            actions: [
              TextButton(
                onPressed: uiManager.clearError,
                child: Text(strings.dismiss),
              ),
            ],
          ),
        Expanded(
          child: AnimatedSwitcher(
            duration: const Duration(milliseconds: 180),
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
      case UiDestination.playback:
        return 4;
    }
  }

  void _select(int index) {
    uiManager.show(UiDestination.values[index]);
  }

  List<NavigationRailDestination> _railDestinations() {
    return [
      NavigationRailDestination(
        icon: const Icon(Icons.devices_outlined),
        selectedIcon: const Icon(Icons.devices),
        label: Text(strings.devices),
      ),
      NavigationRailDestination(
        icon: const Icon(Icons.send_outlined),
        selectedIcon: const Icon(Icons.send),
        label: Text(strings.send),
      ),
      NavigationRailDestination(
        icon: const Icon(Icons.swap_horiz_outlined),
        selectedIcon: const Icon(Icons.swap_horiz),
        label: Text(strings.transfers),
      ),
      NavigationRailDestination(
        icon: const Icon(Icons.settings_outlined),
        selectedIcon: const Icon(Icons.settings),
        label: Text(strings.settings),
      ),
      NavigationRailDestination(
        icon: const Icon(Icons.play_circle_outline),
        selectedIcon: const Icon(Icons.play_circle),
        label: Text(strings.playback),
      ),
    ];
  }

  List<NavigationDestination> _barDestinations() {
    return [
      NavigationDestination(
        icon: const Icon(Icons.devices),
        label: strings.devices,
      ),
      NavigationDestination(icon: const Icon(Icons.send), label: strings.send),
      NavigationDestination(
        icon: const Icon(Icons.swap_horiz),
        label: strings.transfers,
      ),
      NavigationDestination(
        icon: const Icon(Icons.settings),
        label: strings.settings,
      ),
      NavigationDestination(
        icon: const Icon(Icons.play_circle),
        label: strings.playback,
      ),
    ];
  }
}
