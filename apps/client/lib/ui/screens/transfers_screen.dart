import 'package:flutter/material.dart';

import '../../src/rust/api/models.dart';
import '../ui_manager.dart';
import '../ui_strings.dart';
import 'screen_frame.dart';

class TransfersScreen extends StatelessWidget {
  const TransfersScreen({
    required this.uiManager,
    required this.strings,
    super.key,
  });

  final UiManager uiManager;
  final UiStrings strings;

  @override
  Widget build(BuildContext context) {
    return ScreenFrame(
      title: strings.transfers,
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          if (uiManager.transfers.isEmpty) Text(strings.noTransfers),
          for (final transfer in uiManager.transfers)
            Padding(
              padding: const EdgeInsets.only(bottom: 12),
              child: _TransferCard(
                transfer: transfer,
                uiManager: uiManager,
                strings: strings,
              ),
            ),
        ],
      ),
    );
  }
}

class _TransferCard extends StatelessWidget {
  const _TransferCard({
    required this.transfer,
    required this.uiManager,
    required this.strings,
  });

  final UiFileTransfer transfer;
  final UiManager uiManager;
  final UiStrings strings;

  @override
  Widget build(BuildContext context) {
    return Card(
      child: Padding(
        padding: const EdgeInsets.all(20),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            ListTile(
              contentPadding: EdgeInsets.zero,
              leading: Icon(_directionIcon()),
              title: Text(transfer.fileName),
              subtitle: Text(
                '${transfer.peerDeviceId} • ${transfer.state.name}',
              ),
              trailing: IconButton(
                onPressed: () {
                  uiManager.removeTransfer(transfer);
                },
                tooltip: strings.remove,
                icon: const Icon(Icons.close),
              ),
            ),
            const SizedBox(height: 8),
            LinearProgressIndicator(value: _progress()),
            const SizedBox(height: 8),
            Text(_sizeLabel()),
            if (_canAccept()) ...[
              const SizedBox(height: 16),
              Wrap(
                spacing: 8,
                runSpacing: 8,
                children: [
                  FilledButton(
                    onPressed: () {
                      uiManager.acceptTransfer(transfer);
                    },
                    child: Text(strings.accept),
                  ),
                  OutlinedButton(
                    onPressed: () {
                      uiManager.acceptTransfer(transfer, chooseFolder: true);
                    },
                    child: Text(strings.chooseFolder),
                  ),
                  TextButton(
                    onPressed: () {
                      uiManager.rejectTransfer(transfer);
                    },
                    child: Text(strings.decline),
                  ),
                ],
              ),
            ],
          ],
        ),
      ),
    );
  }

  bool _canAccept() {
    return transfer.direction == UiFileTransferDirection.incoming &&
        transfer.state == UiFileTransferState.offered;
  }

  double? _progress() {
    if (transfer.fileSize == BigInt.zero) {
      return null;
    }
    return transfer.transferredBytes.toDouble() / transfer.fileSize.toDouble();
  }

  String _sizeLabel() {
    return '${_formatBytes(transfer.transferredBytes)} / '
        '${_formatBytes(transfer.fileSize)}';
  }

  String _formatBytes(BigInt bytes) {
    const units = ['B', 'KB', 'MB', 'GB', 'TB'];
    var value = bytes.toDouble();
    var unit = 0;
    while (value >= 1024 && unit < units.length - 1) {
      value /= 1024;
      unit++;
    }
    var fractionDigits = 1;
    if (unit == 0) {
      fractionDigits = 0;
    }
    return '${value.toStringAsFixed(fractionDigits)} ${units[unit]}';
  }

  IconData _directionIcon() {
    if (transfer.direction == UiFileTransferDirection.incoming) {
      return Icons.download;
    }
    return Icons.upload;
  }
}
