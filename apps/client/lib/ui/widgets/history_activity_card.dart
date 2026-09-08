import 'package:flutter/material.dart';

import '../../src/rust/api/activity.dart';
import '../history_presentation.dart';
import '../ui_strings.dart';

class HistoryActivityCard extends StatelessWidget {
  const HistoryActivityCard({
    required this.activity,
    required this.strings,
    this.onRetry,
    this.onOpen,
    this.onRemove,
    super.key,
  });

  final UiActivity activity;
  final UiStrings strings;
  final VoidCallback? onRetry;
  final VoidCallback? onOpen;
  final VoidCallback? onRemove;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final colors = theme.colorScheme;
    final session = activity.kind == UiActivityKind.session;
    var title = activity.title;
    var label = strings.historyReceived;
    var icon = Icons.south_west_rounded;
    if (activity.direction == UiActivityDirection.outgoing) {
      label = strings.historySent;
      icon = Icons.north_east_rounded;
    }
    if (session) {
      title = strings.historyConnection;
      label = strings.historyConnections;
      icon = Icons.link_rounded;
    }
    var statusColor = colors.onSurfaceVariant;
    if (activity.status == UiActivityStatus.completed ||
        activity.status == UiActivityStatus.delivered) {
      statusColor = const Color(0xFF8DDFB0);
    }
    if (activity.status == UiActivityStatus.failed ||
        activity.status == UiActivityStatus.interrupted) {
      statusColor = const Color(0xFFFFD16A);
    }
    return Card(
      shape: RoundedRectangleBorder(
        borderRadius: BorderRadius.circular(16),
        side: BorderSide(color: colors.outlineVariant.withValues(alpha: 0.35)),
      ),
      child: Padding(
        padding: const EdgeInsets.all(20),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Row(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Padding(
                  padding: const EdgeInsetsDirectional.only(end: 14, top: 2),
                  child: Icon(icon, size: 20, color: colors.onSurfaceVariant),
                ),
                Expanded(
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Text(
                        label,
                        style: theme.textTheme.labelSmall?.copyWith(
                          color: colors.onSurfaceVariant,
                        ),
                      ),
                      const SizedBox(height: 4),
                      Text(
                        title,
                        style: theme.textTheme.titleMedium,
                        maxLines: 3,
                        overflow: TextOverflow.ellipsis,
                      ),
                    ],
                  ),
                ),
                if (onRemove != null)
                  IconButton(
                    tooltip: strings.remove,
                    onPressed: onRemove,
                    icon: const Icon(Icons.close, size: 18),
                  ),
              ],
            ),
            const SizedBox(height: 12),
            Text(
              historyStatus(activity, strings),
              style: theme.textTheme.labelLarge?.copyWith(color: statusColor),
            ),
            if (activity.retryOf != null) ...[
              const SizedBox(height: 4),
              Text(strings.retryAttempt, style: theme.textTheme.bodySmall),
            ],
            const SizedBox(height: 16),
            _time(
              context,
              strings.activityStarted,
              activity.startedAt,
              session: session,
            ),
            if (activity.fileCompletedAt != null)
              _time(context, strings.fileCompleted, activity.fileCompletedAt!),
            if (activity.disconnectedAt != null)
              _time(context, strings.disconnectedAt, activity.disconnectedAt!),
            if (activity.endedAt != null)
              _time(context, _endLabel(session), activity.endedAt!),
            if (activity.status == UiActivityStatus.interrupted &&
                activity.endedAt == null)
              Padding(
                padding: const EdgeInsets.only(top: 8),
                child: Text(
                  strings.historyEndUnknown,
                  style: theme.textTheme.bodySmall,
                ),
              ),
            if (activity.failure != null) ...[
              const SizedBox(height: 8),
              Text(
                _failure(),
                style: theme.textTheme.bodySmall?.copyWith(
                  color: colors.onSurfaceVariant,
                ),
              ),
            ],
            if (activity.canRetry || onOpen != null) ...[
              const SizedBox(height: 12),
              Wrap(
                spacing: 8,
                runSpacing: 8,
                children: [
                  if (activity.canRetry)
                    Tooltip(
                      message: _retryHint(),
                      child: OutlinedButton.icon(
                        onPressed: onRetry,
                        icon: const Icon(Icons.refresh_rounded, size: 18),
                        label: Text(strings.retry),
                      ),
                    ),
                  if (onOpen != null)
                    TextButton.icon(
                      onPressed: onOpen,
                      icon: const Icon(Icons.open_in_new_rounded, size: 18),
                      label: Text(strings.open),
                    ),
                ],
              ),
            ],
          ],
        ),
      ),
    );
  }

  Widget _time(
    BuildContext context,
    String label,
    BigInt timestamp, {
    bool session = false,
  }) {
    if (session) {
      label = strings.sessionStarted;
    }
    return Padding(
      padding: const EdgeInsets.only(bottom: 10),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text(
            label,
            style: Theme.of(context).textTheme.labelSmall?.copyWith(
              color: Theme.of(context).colorScheme.onSurfaceVariant,
            ),
          ),
          const SizedBox(height: 3),
          Wrap(
            spacing: 12,
            runSpacing: 3,
            children: [
              Text(
                historyDate(timestamp),
                textDirection: TextDirection.ltr,
                style: Theme.of(context).textTheme.bodySmall,
              ),
              Text(
                historyTime(timestamp),
                textDirection: TextDirection.ltr,
                style: Theme.of(context).textTheme.bodySmall,
              ),
            ],
          ),
        ],
      ),
    );
  }

  String _endLabel(bool session) {
    if (session) {
      return strings.sessionEnded;
    }
    return strings.activityEnded;
  }

  String _retryHint() {
    if (onRetry == null) {
      return strings.retryConnectFirst;
    }
    return strings.retryExplanation;
  }

  String _failure() {
    return strings.historyFailure(activity.failure!);
  }
}
