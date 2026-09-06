import 'package:flutter/material.dart';

import '../../src/rust/api/models.dart';
import '../ui_manager.dart';
import '../ui_strings.dart';
import 'screen_frame.dart';

class DevicesScreen extends StatefulWidget {
  const DevicesScreen({
    required this.uiManager,
    required this.strings,
    super.key,
  });

  final UiManager uiManager;
  final UiStrings strings;

  @override
  State<DevicesScreen> createState() => _DevicesScreenState();
}

class _DevicesScreenState extends State<DevicesScreen> {
  final _host = TextEditingController();
  final _port = TextEditingController();

  @override
  void dispose() {
    _host.dispose();
    _port.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final devices = widget.uiManager.devices;
    final pairing = widget.uiManager.pairing;
    return ScreenFrame(
      title: widget.strings.devices,
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          if (devices != null) _localDevice(devices.localDevice),
          const SizedBox(height: 24),
          _pairing(pairing),
          const SizedBox(height: 24),
          _manualEndpoint(),
          const SizedBox(height: 24),
          Text(
            widget.strings.nearbyDevices,
            style: Theme.of(context).textTheme.titleLarge,
          ),
          const SizedBox(height: 12),
          if (devices == null || devices.candidates.isEmpty)
            Text(widget.strings.noNearbyDevices),
          if (devices != null) ...devices.candidates.map(_candidate),
          const SizedBox(height: 24),
          Text(
            widget.strings.trustedDevices,
            style: Theme.of(context).textTheme.titleLarge,
          ),
          const SizedBox(height: 12),
          if (devices == null || devices.trustedDevices.isEmpty)
            Text(widget.strings.noTrustedDevices),
          if (devices != null) ...devices.trustedDevices.map(_trustedDevice),
        ],
      ),
    );
  }

  Widget _localDevice(UiLocalDevice device) {
    return Card(
      child: ListTile(
        leading: const CircleAvatar(child: Icon(Icons.computer)),
        title: Text(device.displayName),
        subtitle: Text('${widget.strings.localDevice} • ${device.id}'),
      ),
    );
  }

  Widget _pairing(UiPairingSnapshot? snapshot) {
    final sessions = snapshot?.sessions ?? [];
    return Card(
      child: Padding(
        padding: const EdgeInsets.all(20),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            Row(
              children: [
                Expanded(
                  child: Text(
                    widget.strings.pairingCode,
                    style: Theme.of(context).textTheme.titleMedium,
                  ),
                ),
                FilledButton.tonalIcon(
                  onPressed: widget.uiManager.busy
                      ? null
                      : widget.uiManager.receivePairing,
                  icon: const Icon(Icons.call_received),
                  label: Text(widget.strings.receivePairing),
                ),
              ],
            ),
            for (final session in sessions) ...[
              const SizedBox(height: 16),
              Text(session.peerDisplayName ?? session.state.name),
              if (session.manualCode != null)
                SelectableText(
                  session.manualCode!,
                  style: Theme.of(context).textTheme.headlineMedium,
                ),
              const SizedBox(height: 8),
              Wrap(
                spacing: 8,
                children: [
                  FilledButton(
                    onPressed: widget.uiManager.approvePairing,
                    child: Text(widget.strings.approve),
                  ),
                  OutlinedButton(
                    onPressed: widget.uiManager.rejectPairing,
                    child: Text(widget.strings.reject),
                  ),
                  TextButton(
                    onPressed: widget.uiManager.cancelPairing,
                    child: Text(widget.strings.cancel),
                  ),
                ],
              ),
            ],
          ],
        ),
      ),
    );
  }

  Widget _manualEndpoint() {
    return Card(
      child: Padding(
        padding: const EdgeInsets.all(20),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            Text(
              widget.strings.manualEndpoint,
              style: Theme.of(context).textTheme.titleMedium,
            ),
            const SizedBox(height: 16),
            Row(
              children: [
                Expanded(
                  flex: 3,
                  child: TextField(
                    controller: _host,
                    decoration: InputDecoration(labelText: widget.strings.host),
                  ),
                ),
                const SizedBox(width: 12),
                Expanded(
                  child: TextField(
                    controller: _port,
                    keyboardType: TextInputType.number,
                    decoration: InputDecoration(labelText: widget.strings.port),
                  ),
                ),
                const SizedBox(width: 12),
                FilledButton(
                  onPressed: () {
                    widget.uiManager.addManualEndpoint(_host.text, _port.text);
                  },
                  child: Text(widget.strings.add),
                ),
              ],
            ),
          ],
        ),
      ),
    );
  }

  Widget _candidate(UiDiscoveryCandidate candidate) {
    var subtitle = candidate.id;
    if (candidate.endpoints.isNotEmpty) {
      final endpoint = candidate.endpoints.first;
      subtitle = '${endpoint.host}:${endpoint.port}';
    }
    return Padding(
      padding: const EdgeInsets.only(bottom: 8),
      child: Card(
        child: ListTile(
          leading: const Icon(Icons.radar),
          title: Text(candidate.id),
          subtitle: Text(subtitle),
          trailing: FilledButton.tonal(
            onPressed: () => widget.uiManager.pairWith(candidate),
            child: Text(widget.strings.pair),
          ),
        ),
      ),
    );
  }

  Widget _trustedDevice(UiTrustedDevice device) {
    final connected = widget.uiManager.isConnected(device.id);
    var connectionIcon = Icons.link_off;
    if (connected) {
      connectionIcon = Icons.link;
    }
    return Padding(
      padding: const EdgeInsets.only(bottom: 8),
      child: Card(
        child: ListTile(
          leading: Icon(connectionIcon),
          title: Text(device.displayName),
          subtitle: Text(device.platform.name),
          trailing: Wrap(
            spacing: 8,
            children: [
              if (connected)
                OutlinedButton(
                  onPressed: () => widget.uiManager.disconnect(device),
                  child: Text(widget.strings.disconnect),
                ),
              if (!connected)
                FilledButton.tonal(
                  onPressed: () => widget.uiManager.connect(device),
                  child: Text(widget.strings.connect),
                ),
              IconButton(
                onPressed: () => widget.uiManager.forget(device),
                tooltip: widget.strings.forget,
                icon: const Icon(Icons.delete_outline),
              ),
            ],
          ),
        ),
      ),
    );
  }
}
