import 'package:flutter/material.dart';

import '../../src/rust/api/activity.dart';
import '../history_presentation.dart';
import '../ui_manager.dart';
import '../ui_strings.dart';
import '../widgets/history_activity_card.dart';
import '../widgets/history_device_card.dart';

enum _HistoryFilter { all, files, connections }

class HistoryScreen extends StatefulWidget {
  const HistoryScreen({
    required this.uiManager,
    required this.strings,
    super.key,
  });
  final UiManager uiManager;
  final UiStrings strings;

  @override
  State<HistoryScreen> createState() => _HistoryScreenState();
}

class _HistoryScreenState extends State<HistoryScreen> {
  _HistoryFilter _filter = _HistoryFilter.all;

  @override
  Widget build(BuildContext context) {
    final manager = widget.uiManager;
    final strings = widget.strings;
    final devices = DeviceHistory.group(manager.history);
    final deviceId = manager.historyDevice;
    DeviceHistory? selected;
    for (final device in devices) {
      if (device.deviceId == deviceId) {
        selected = device;
        break;
      }
    }
    var title = strings.history;
    if (selected != null) {
      title = selected.name;
    }
    return PopScope(
      canPop: deviceId == null,
      onPopInvokedWithResult: (didPop, result) {
        if (!didPop) {
          manager.showHistoryDevices();
        }
      },
      child: CustomScrollView(
        key: ValueKey(deviceId),
        slivers: [
          SliverAppBar.large(
            title: Text(title, maxLines: 1, overflow: TextOverflow.ellipsis),
            pinned: true,
            leading: _backButton(deviceId),
            actions: [
              if (deviceId == null && devices.isNotEmpty)
                IconButton(
                  tooltip: strings.clearHistory,
                  onPressed: _clearAction(),
                  icon: const Icon(Icons.delete_outline),
                ),
              const SizedBox(width: 12),
            ],
          ),
          SliverPadding(
            padding: const EdgeInsets.fromLTRB(24, 4, 24, 20),
            sliver: SliverToBoxAdapter(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Text(
                    _subtitle(deviceId),
                    style: Theme.of(context).textTheme.bodyMedium?.copyWith(
                      color: Theme.of(context).colorScheme.onSurfaceVariant,
                    ),
                  ),
                  if (manager.historyStorageError != null) ...[
                    const SizedBox(height: 16),
                    Row(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      children: [
                        const Icon(
                          Icons.warning_amber_rounded,
                          color: Color(0xFFFFD16A),
                        ),
                        const SizedBox(width: 10),
                        Expanded(child: Text(strings.historySaveFailed)),
                      ],
                    ),
                  ],
                  if (deviceId != null) ...[
                    const SizedBox(height: 18),
                    Wrap(
                      spacing: 8,
                      runSpacing: 6,
                      children: [
                        _filterChip(_HistoryFilter.all, strings.historyAll),
                        _filterChip(_HistoryFilter.files, strings.historyFiles),
                        _filterChip(
                          _HistoryFilter.connections,
                          strings.historyConnections,
                        ),
                      ],
                    ),
                  ],
                ],
              ),
            ),
          ),
          if (deviceId == null) _devices(devices) else _timeline(selected),
          const SliverToBoxAdapter(child: SizedBox(height: 32)),
        ],
      ),
    );
  }

  Widget? _backButton(String? deviceId) {
    if (deviceId == null) {
      return null;
    }
    return BackButton(onPressed: widget.uiManager.showHistoryDevices);
  }

  VoidCallback? _clearAction() {
    if (widget.uiManager.historyBusy) {
      return null;
    }
    return _clear;
  }

  String _subtitle(String? deviceId) {
    if (deviceId == null) {
      return widget.strings.historyIntro;
    }
    return widget.strings.historyTimes;
  }

  Widget _filterChip(_HistoryFilter filter, String label) {
    return ChoiceChip(
      label: Text(label),
      selected: _filter == filter,
      onSelected: (_) => setState(() => _filter = filter),
    );
  }

  Widget _devices(List<DeviceHistory> devices) {
    if (devices.isEmpty) {
      return _empty(widget.strings.noHistory);
    }
    return SliverPadding(
      padding: const EdgeInsets.symmetric(horizontal: 24),
      sliver: SliverLayoutBuilder(
        builder: (context, constraints) {
          var columns = 1;
          if (constraints.crossAxisExtent >= 1000) {
            columns = 3;
          } else if (constraints.crossAxisExtent >= 620) {
            columns = 2;
          }
          final rows = (devices.length / columns).ceil();
          return SliverList.builder(
            itemCount: rows,
            itemBuilder: (context, row) {
              final cards = <Widget>[];
              for (var column = 0; column < columns; column++) {
                if (column > 0) {
                  cards.add(const SizedBox(width: 16));
                }
                final index = row * columns + column;
                if (index >= devices.length) {
                  cards.add(const Expanded(child: SizedBox.shrink()));
                  continue;
                }
                final device = devices[index];
                cards.add(
                  Expanded(
                    child: HistoryDeviceCard(
                      device: device,
                      strings: widget.strings,
                      unread: widget.uiManager.hasDeviceNotification(
                        device.deviceId,
                      ),
                      onOpen: () {
                        _filter = _HistoryFilter.all;
                        widget.uiManager.showDeviceHistory(device.deviceId);
                      },
                    ),
                  ),
                );
              }
              return Padding(
                padding: const EdgeInsets.only(bottom: 16),
                child: Row(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: cards,
                ),
              );
            },
          );
        },
      ),
    );
  }

  Widget _timeline(DeviceHistory? device) {
    final entries = device?.entries.where(_matches).toList() ?? <UiActivity>[];
    if (entries.isEmpty) {
      return _empty(widget.strings.noDeviceHistory);
    }
    return SliverPadding(
      padding: const EdgeInsets.symmetric(horizontal: 24),
      sliver: SliverList.builder(
        itemCount: entries.length,
        itemBuilder: (context, index) {
          final entry = entries[index];
          final day = historyDate(activityTime(entry));
          final newDay =
              index == 0 ||
              historyDate(activityTime(entries[index - 1])) != day;
          VoidCallback? retry;
          VoidCallback? open;
          VoidCallback? remove;
          if (!widget.uiManager.historyBusy) {
            if (entry.canRetry &&
                widget.uiManager.isConnected(entry.deviceId)) {
              retry = () => widget.uiManager.retryActivity(entry);
            }
            if (widget.uiManager.canOpenActivity(entry)) {
              open = () => widget.uiManager.openActivity(entry);
            }
            if (entry.status != UiActivityStatus.active) {
              remove = () => widget.uiManager.removeActivity(entry);
            }
          }
          return Align(
            alignment: AlignmentDirectional.centerStart,
            child: ConstrainedBox(
              constraints: const BoxConstraints(maxWidth: 920),
              child: Padding(
                padding: const EdgeInsets.only(bottom: 12),
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    if (newDay)
                      Padding(
                        padding: const EdgeInsets.only(top: 10, bottom: 12),
                        child: Text(
                          day,
                          textDirection: TextDirection.ltr,
                          style: Theme.of(context).textTheme.titleSmall,
                        ),
                      ),
                    HistoryActivityCard(
                      activity: entry,
                      strings: widget.strings,
                      onRetry: retry,
                      onOpen: open,
                      onRemove: remove,
                    ),
                  ],
                ),
              ),
            ),
          );
        },
      ),
    );
  }

  bool _matches(UiActivity entry) {
    switch (_filter) {
      case _HistoryFilter.all:
        return true;
      case _HistoryFilter.files:
        return entry.kind == UiActivityKind.file ||
            entry.kind == UiActivityKind.localVideo;
      case _HistoryFilter.connections:
        return entry.kind == UiActivityKind.session;
    }
  }

  Widget _empty(String message) {
    return SliverFillRemaining(
      hasScrollBody: false,
      child: Padding(
        padding: const EdgeInsets.all(32),
        child: Column(
          mainAxisAlignment: MainAxisAlignment.center,
          children: [
            Icon(
              Icons.history_rounded,
              size: 48,
              color: Theme.of(context).colorScheme.outline,
            ),
            const SizedBox(height: 18),
            Text(
              message,
              textAlign: TextAlign.center,
              style: Theme.of(context).textTheme.titleMedium,
            ),
            const SizedBox(height: 8),
            Text(
              widget.strings.historyStoredLocally,
              textAlign: TextAlign.center,
              style: Theme.of(context).textTheme.bodySmall,
            ),
          ],
        ),
      ),
    );
  }

  Future<void> _clear() async {
    final strings = widget.strings;
    final confirmed = await showDialog<bool>(
      context: context,
      builder: (context) => AlertDialog(
        title: Text(strings.clearHistory),
        content: Text(strings.clearHistoryMessage),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(context, false),
            child: Text(strings.cancel),
          ),
          FilledButton(
            onPressed: () => Navigator.pop(context, true),
            child: Text(strings.clearHistory),
          ),
        ],
      ),
    );
    if (confirmed == true && mounted) {
      await widget.uiManager.clearHistory();
    }
  }
}
