import 'package:flutter/material.dart';

import '../../src/rust/api/models.dart';
import '../history_presentation.dart';
import '../ui_strings.dart';
import 'navigation_badge.dart';

class HistoryDeviceCard extends StatelessWidget {
  const HistoryDeviceCard({
    required this.device,
    required this.strings,
    required this.unread,
    required this.onOpen,
    super.key,
  });

  final DeviceHistory device;
  final UiStrings strings;
  final bool unread;
  final VoidCallback onOpen;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final colors = theme.colorScheme;
    return Card(
      shape: RoundedRectangleBorder(
        borderRadius: BorderRadius.circular(20),
        side: BorderSide(color: colors.outlineVariant.withValues(alpha: 0.45)),
      ),
      child: InkWell(
        onTap: onOpen,
        child: Padding(
          padding: const EdgeInsets.all(22),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Row(
                children: [
                  Container(
                    width: 48,
                    height: 48,
                    decoration: BoxDecoration(
                      color: colors.primary.withValues(alpha: 0.12),
                      borderRadius: BorderRadius.circular(14),
                    ),
                    child: Icon(_icon(), color: colors.primary),
                  ),
                  const Spacer(),
                  NavigationBadge(
                    visible: unread,
                    semanticsLabel: strings.newActivity,
                    child: const Padding(
                      padding: EdgeInsets.all(6),
                      child: Icon(Icons.chevron_right, size: 20),
                    ),
                  ),
                ],
              ),
              const SizedBox(height: 18),
              Text(
                device.name,
                maxLines: 2,
                overflow: TextOverflow.ellipsis,
                style: theme.textTheme.titleMedium,
              ),
              const SizedBox(height: 5),
              Text(
                _platformName(),
                style: theme.textTheme.bodySmall?.copyWith(
                  color: colors.onSurfaceVariant,
                ),
              ),
              const SizedBox(height: 20),
              Text(
                strings.lastActivity,
                style: theme.textTheme.labelSmall?.copyWith(
                  color: colors.onSurfaceVariant,
                ),
              ),
              const SizedBox(height: 5),
              Text(
                historyDate(device.lastActivity),
                textDirection: TextDirection.ltr,
                style: theme.textTheme.bodyMedium,
              ),
              Text(
                historyTime(device.lastActivity),
                textDirection: TextDirection.ltr,
                style: theme.textTheme.bodySmall?.copyWith(
                  color: colors.onSurfaceVariant,
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }

  IconData _icon() {
    switch (device.platform) {
      case UiPlatform.android:
      case UiPlatform.ios:
        return Icons.phone_iphone_outlined;
      default:
        return Icons.computer_outlined;
    }
  }

  String _platformName() {
    switch (device.platform) {
      case UiPlatform.windows:
        return 'Windows';
      case UiPlatform.linux:
        return 'Linux';
      case UiPlatform.macOs:
        return 'macOS';
      case UiPlatform.android:
        return 'Android';
      case UiPlatform.ios:
        return 'iOS';
      case UiPlatform.unknown:
        return 'ContinueHere';
    }
  }
}
