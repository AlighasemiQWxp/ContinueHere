import 'package:continuehere_client/src/rust/api/models.dart';
import 'package:continuehere_client/ui/ui_file_support.dart';
import 'package:continuehere_client/ui/ui_strings.dart';
import 'package:continuehere_client/ui/ui_motion.dart';
import 'package:continuehere_client/ui/ui_notifications.dart';
import 'package:continuehere_client/ui/ui_transition.dart';
import 'package:continuehere_client/ui/widgets/language_radio_group.dart';
import 'package:continuehere_client/ui/widgets/navigation_badge.dart';
import 'package:continuehere_client/ui/widgets/ui_content_transition.dart';
import 'package:continuehere_client/ui/widgets/ui_page_transition.dart';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  test('UI transition reports destination changes once', () {
    var changes = 0;
    final transition = UiTransition(() {
      changes++;
    });

    transition.show(UiDestination.devices);
    transition.show(UiDestination.send);

    expect(transition.destination, UiDestination.send);
    expect(changes, 1);
  });

  test('Persian strings use Persian labels', () {
    const strings = UiStrings(UiLanguage.persian);

    expect(strings.settings, 'تنظیمات');
    expect(strings.devices, 'دستگاه‌ها');
  });

  test('file support routes previews and blocks executable files', () {
    expect(UiFileSupport.previewKind('photo.JPEG'), UiFilePreviewKind.image);
    expect(UiFileSupport.previewKind('movie.mkv'), UiFilePreviewKind.video);
    expect(UiFileSupport.previewKind('notes.txt'), isNull);
    expect(UiFileSupport.isUnsafeToOpen('setup.exe'), isTrue);
    expect(UiFileSupport.isUnsafeToOpen('report.pdf'), isFalse);
  });

  test('UI transition owns file preview visibility', () {
    var changes = 0;
    final transition = UiTransition(() {
      changes++;
    });

    transition.showFilePreview();
    transition.showFilePreview();
    expect(transition.filePreviewVisible, isTrue);
    expect(changes, 1);

    transition.hideFilePreview();
    expect(transition.filePreviewVisible, isFalse);
    expect(changes, 2);
  });

  testWidgets('UI motion respects the reduced-motion preference', (
    tester,
  ) async {
    var duration = const Duration(milliseconds: 1);
    await tester.pumpWidget(
      MediaQuery(
        data: const MediaQueryData(disableAnimations: true),
        child: Builder(
          builder: (context) {
            duration = UiMotion.standard(context);
            return const SizedBox();
          },
        ),
      ),
    );

    expect(duration, Duration.zero);
  });

  test('notifications track only activity on inactive destinations', () {
    var changes = 0;
    final notifications = UiNotifications(() {
      changes++;
    });

    notifications.receive(UiDestination.devices, UiDestination.devices);
    notifications.receive(UiDestination.transfers, UiDestination.devices);
    notifications.receive(UiDestination.transfers, UiDestination.devices);

    expect(notifications.has(UiDestination.devices), isFalse);
    expect(notifications.has(UiDestination.transfers), isTrue);
    expect(changes, 1);

    notifications.open(UiDestination.transfers);

    expect(notifications.has(UiDestination.transfers), isFalse);
    expect(changes, 2);
  });

  testWidgets('language selection uses a radio group', (tester) async {
    UiLanguage? selected;
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: LanguageRadioGroup(
            language: UiLanguage.english,
            strings: const UiStrings(UiLanguage.english),
            onSelected: (language) {
              selected = language;
            },
          ),
        ),
      ),
    );

    expect(find.byType(RadioGroup<UiLanguage>), findsOneWidget);
    expect(find.byType(RadioListTile<UiLanguage>), findsNWidgets(2));

    await tester.tap(find.text('Persian'));

    expect(selected, UiLanguage.persian);
  });

  testWidgets('navigation badge animates unread activity', (tester) async {
    Widget app(bool visible) {
      return MaterialApp(
        home: NavigationBadge(
          visible: visible,
          semanticsLabel: 'New activity',
          child: const Icon(Icons.swap_horiz),
        ),
      );
    }

    await tester.pumpWidget(app(false));
    var badge = tester.widget<Badge>(find.byType(Badge));
    expect(badge.isLabelVisible, isFalse);

    await tester.pumpWidget(app(true));
    await tester.pump(const Duration(milliseconds: 180));
    badge = tester.widget<Badge>(find.byType(Badge));
    expect(badge.isLabelVisible, isTrue);
    expect(badge.largeSize, 10);
    expect(
      find.byWidgetPredicate(
        (widget) =>
            widget is Semantics && widget.properties.label == 'New activity',
      ),
      findsOneWidget,
    );
  });

  testWidgets('page changes use fade and directional slide transitions', (
    tester,
  ) async {
    Widget app(Key key) {
      return Directionality(
        textDirection: TextDirection.rtl,
        child: UiPageTransition(isRtl: true, child: SizedBox(key: key)),
      );
    }

    await tester.pumpWidget(app(const ValueKey('devices')));
    await tester.pumpWidget(app(const ValueKey('transfers')));
    await tester.pump(const Duration(milliseconds: 50));

    expect(find.byType(FadeTransition), findsWidgets);
    expect(find.byType(SlideTransition), findsWidgets);
  });

  testWidgets('content changes use fade and size transitions', (tester) async {
    Widget app(Object stateKey) {
      return Directionality(
        textDirection: TextDirection.ltr,
        child: UiContentTransition(stateKey: stateKey, child: const SizedBox()),
      );
    }

    await tester.pumpWidget(app('empty'));
    await tester.pumpWidget(app('populated'));
    await tester.pump(const Duration(milliseconds: 50));

    expect(find.byType(FadeTransition), findsWidgets);
    expect(find.byType(SizeTransition), findsWidgets);
  });
}
