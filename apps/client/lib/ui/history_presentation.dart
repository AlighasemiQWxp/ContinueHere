import '../src/rust/api/activity.dart';
import '../src/rust/api/models.dart';
import 'ui_strings.dart';

class DeviceHistory {
  DeviceHistory(List<UiActivity> entries)
    : _entries = List.unmodifiable(entries);

  final List<UiActivity> _entries;
  List<UiActivity> get entries => _entries;
  String get deviceId => _entries.first.deviceId;
  String get name => _entries.first.deviceName;
  UiPlatform get platform => _entries.first.platform;
  BigInt get lastActivity => activityTime(_entries.first);

  static List<DeviceHistory> group(List<UiActivity> entries) {
    final grouped = <String, List<UiActivity>>{};
    for (final entry in entries) {
      grouped.putIfAbsent(entry.deviceId, () => []).add(entry);
    }
    final devices = grouped.values.map((entries) {
      entries.sort((a, b) {
        final byTime = activityTime(b).compareTo(activityTime(a));
        if (byTime != 0) {
          return byTime;
        }
        return b.revision.compareTo(a.revision);
      });
      return DeviceHistory(entries);
    }).toList();
    devices.sort((a, b) => b.lastActivity.compareTo(a.lastActivity));
    return devices;
  }
}

BigInt activityTime(UiActivity activity) {
  var time = activity.startedAt;
  for (final candidate in [
    activity.endedAt,
    activity.completedAt,
    activity.fileCompletedAt,
    activity.disconnectedAt,
  ]) {
    if (candidate != null && candidate > time) {
      time = candidate;
    }
  }
  return time;
}

String historyDate(BigInt timestamp) {
  final time = DateTime.fromMillisecondsSinceEpoch(timestamp.toInt());
  return '${time.year}-${_two(time.month)}-${_two(time.day)}';
}

String historyTime(BigInt timestamp) {
  final time = DateTime.fromMillisecondsSinceEpoch(timestamp.toInt());
  final offset = time.timeZoneOffset;
  var sign = '+';
  if (offset.isNegative) {
    sign = '-';
  }
  final minutes = offset.inMinutes.abs();
  return '${_two(time.hour)}:${_two(time.minute)}:${_two(time.second)}.${time.millisecond.toString().padLeft(3, '0')} '
      'UTC$sign${_two(minutes ~/ 60)}:${_two(minutes % 60)}';
}

String _two(int value) => value.toString().padLeft(2, '0');

String historyStatus(UiActivity activity, UiStrings strings) {
  switch (activity.status) {
    case UiActivityStatus.active:
      if (activity.kind == UiActivityKind.session) {
        return strings.historyConnected;
      }
      return strings.historyActive;
    case UiActivityStatus.delivered:
      return strings.historyDelivered;
    case UiActivityStatus.completed:
      return strings.historyCompleted;
    case UiActivityStatus.rejected:
      return strings.historyRejected;
    case UiActivityStatus.cancelled:
      return strings.historyCancelled;
    case UiActivityStatus.failed:
      return strings.historyFailed;
    case UiActivityStatus.disconnected:
      return strings.historyDisconnected;
    case UiActivityStatus.interrupted:
      return strings.historyInterrupted;
  }
}
