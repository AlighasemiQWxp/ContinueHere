import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:continuehere_client/src/rust/api/activity.dart';
import 'package:continuehere_client/src/rust/api/models.dart';
import 'package:continuehere_client/ui/history_presentation.dart';
import 'package:continuehere_client/ui/ui_notifications.dart';
import 'package:continuehere_client/ui/ui_strings.dart';
import 'package:continuehere_client/ui/ui_transition.dart';
import 'package:continuehere_client/ui/widgets/history_activity_card.dart';
import 'package:continuehere_client/ui/widgets/history_device_card.dart';
import 'package:continuehere_client/ui/widgets/navigation_badge.dart';

UiActivity entry(
  String id,
  String deviceId,
  int time, {
  UiActivityKind kind = UiActivityKind.file,
  UiActivityStatus status = UiActivityStatus.completed,
}) {
  final timestamp = BigInt.from(time);
  return UiActivity(
    id: id,
    deviceId: deviceId,
    deviceName: 'A device with a long readable name',
    platform: UiPlatform.windows,
    kind: kind,
    direction: UiActivityDirection.incoming,
    status: status,
    title: 'holiday-video.mp4',
    startedAt: timestamp,
    endedAt: timestamp + BigInt.from(2000),
    completedAt: timestamp + BigInt.from(2000),
    disconnectedAt: null,
    fileCompletedAt: timestamp + BigInt.from(1000),
    failure: null,
    url: null,
    positionMillis: BigInt.zero,
    retryOf: null,
    sessionId: null,
    revision: timestamp,
    filePath: 'C:\\Received\\holiday-video.mp4',
    canRetry: false,
  );
}

void main() {
  test(
    'opening History clears navigation unread but preserves device unread',
    () {
      final notifications = UiNotifications(() {});
      notifications.receiveHistory('peer-a', UiDestination.transfers, null);
      expect(notifications.has(UiDestination.history), isTrue);
      expect(notifications.hasDevice('peer-a'), isTrue);
      notifications.open(UiDestination.history);
      expect(notifications.has(UiDestination.history), isFalse);
      expect(notifications.hasDevice('peer-a'), isTrue);
      notifications.openDevice('peer-a');
      expect(notifications.hasDevice('peer-a'), isFalse);
    },
  );

  test('reading one device does not consume another device notification', () {
    final notifications = UiNotifications(() {});
    notifications.receiveHistory('peer-a', UiDestination.history, 'peer-a');
    expect(notifications.hasDevice('peer-a'), isFalse);
    notifications.receiveHistory('peer-b', UiDestination.history, 'peer-a');
    expect(notifications.hasDevice('peer-b'), isTrue);
    expect(notifications.has(UiDestination.history), isFalse);
    notifications.openDevice('peer-a');
    expect(notifications.hasDevice('peer-b'), isTrue);
    notifications.receiveHistory('peer-b', UiDestination.transfers, null);
    expect(notifications.has(UiDestination.history), isTrue);
  });

  test('cleared devices cannot leave stale navigation notifications', () {
    final notifications = UiNotifications(() {});
    notifications.receiveHistory('peer-a', UiDestination.send, null);
    notifications.retainDevices({});
    expect(notifications.has(UiDestination.history), isFalse);
    expect(notifications.hasDevice('peer-a'), isFalse);
  });

  test(
    'history groups stable device identifiers and orders latest activity first',
    () {
      final devices = DeviceHistory.group([
        entry('a', 'peer-a', 100),
        entry('b', 'peer-b', 500),
        entry('c', 'peer-a', 300),
      ]);
      expect(devices.length, 2);
      expect(devices.first.deviceId, 'peer-b');
      expect(devices.last.entries.map((entry) => entry.id), ['c', 'a']);
    },
  );

  test('timestamp formatting includes seconds milliseconds and UTC offset', () {
    final timestamp = BigInt.from(
      DateTime(2026, 9, 8, 14, 32, 7, 125).millisecondsSinceEpoch,
    );
    expect(historyDate(timestamp), '2026-09-08');
    expect(historyTime(timestamp), startsWith('14:32:07.125 UTC'));
  });

  testWidgets(
    'Persian device card supports narrow width large text and unread badge',
    (tester) async {
      tester.view.physicalSize = const Size(360, 900);
      tester.view.devicePixelRatio = 1;
      addTearDown(tester.view.resetPhysicalSize);
      addTearDown(tester.view.resetDevicePixelRatio);
      var opened = false;
      final device = DeviceHistory.group([entry('a', 'peer-a', 1788877927125)])
          .first;
      await tester.pumpWidget(
        MaterialApp(
          home: MediaQuery(
            data: const MediaQueryData(textScaler: TextScaler.linear(1.6)),
            child: Directionality(
              textDirection: TextDirection.rtl,
              child: Scaffold(
                body: SingleChildScrollView(
                  child: Padding(
                    padding: const EdgeInsets.all(24),
                    child: HistoryDeviceCard(
                      device: device,
                      strings: const UiStrings(UiLanguage.persian),
                      unread: true,
                      onOpen: () => opened = true,
                    ),
                  ),
                ),
              ),
            ),
          ),
        ),
      );
      await tester.pumpAndSettle();
      expect(tester.takeException(), isNull);
      expect(find.byType(NavigationBadge), findsOneWidget);
      await tester.tap(find.text(device.name));
      expect(opened, isTrue);
    },
  );

  testWidgets(
    'timeline labels file completion and activity ending separately',
    (tester) async {
      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: SingleChildScrollView(
              child: HistoryActivityCard(
                activity: entry('a', 'peer-a', 1788877927125),
                strings: const UiStrings(UiLanguage.english),
              ),
            ),
          ),
        ),
      );
      expect(find.text('Received'), findsOneWidget);
      expect(find.text('File completed'), findsOneWidget);
      expect(find.text('Activity ended'), findsOneWidget);
      expect(find.text('holiday-video.mp4'), findsOneWidget);
      expect(tester.takeException(), isNull);
    },
  );
}
