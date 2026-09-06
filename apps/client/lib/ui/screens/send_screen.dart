import 'package:flutter/material.dart';

import '../../src/rust/api/models.dart';
import '../ui_manager.dart';
import '../ui_strings.dart';
import '../widgets/ui_content_transition.dart';
import 'screen_frame.dart';

class SendScreen extends StatefulWidget {
  const SendScreen({required this.uiManager, required this.strings, super.key});

  final UiManager uiManager;
  final UiStrings strings;

  @override
  State<SendScreen> createState() => _SendScreenState();
}

class _SendScreenState extends State<SendScreen> {
  final _url = TextEditingController();
  final _position = TextEditingController();
  String _deviceId = '';

  @override
  void dispose() {
    _url.dispose();
    _position.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final trustedDevices = widget.uiManager.devices?.trustedDevices ?? [];
    final connectedDevices = trustedDevices
        .where((device) => widget.uiManager.isConnected(device.id))
        .toList();
    if (connectedDevices.isNotEmpty &&
        !connectedDevices.any((device) => device.id == _deviceId)) {
      _deviceId = connectedDevices.first.id;
    }
    String? selectedDeviceId;
    if (_deviceId.isNotEmpty) {
      selectedDeviceId = _deviceId;
    }

    return ScreenFrame(
      title: widget.strings.send,
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          DropdownButtonFormField<String>(
            initialValue: selectedDeviceId,
            decoration: InputDecoration(
              labelText: widget.strings.destinationDevice,
            ),
            items: connectedDevices.map(_deviceItem).toList(),
            onChanged: (value) {
              if (value == null) {
                return;
              }
              setState(() {
                _deviceId = value;
              });
            },
          ),
          const SizedBox(height: 20),
          TextField(
            controller: _url,
            keyboardType: TextInputType.url,
            decoration: InputDecoration(labelText: widget.strings.enterUrl),
          ),
          const SizedBox(height: 12),
          TextField(
            controller: _position,
            keyboardType: TextInputType.datetime,
            decoration: InputDecoration(
              labelText: widget.strings.playbackPosition,
            ),
          ),
          const SizedBox(height: 20),
          Wrap(
            spacing: 12,
            runSpacing: 12,
            children: [
              FilledButton.icon(
                onPressed: _sendUrl,
                icon: const Icon(Icons.link),
                label: Text(widget.strings.sendUrl),
              ),
              FilledButton.tonalIcon(
                onPressed: _sendYoutube,
                icon: const Icon(Icons.ondemand_video),
                label: Text(widget.strings.sendYoutube),
              ),
              OutlinedButton.icon(
                onPressed: _sendVideo,
                icon: const Icon(Icons.video_file_outlined),
                label: Text(widget.strings.sendVideo),
              ),
              OutlinedButton.icon(
                onPressed: _sendFile,
                icon: const Icon(Icons.file_present_outlined),
                label: Text(widget.strings.sendFile),
              ),
            ],
          ),
          const SizedBox(height: 32),
          _incomingHandoffs(),
        ],
      ),
    );
  }

  DropdownMenuItem<String> _deviceItem(UiTrustedDevice device) {
    return DropdownMenuItem(value: device.id, child: Text(device.displayName));
  }

  Widget _incomingHandoffs() {
    final incoming = widget.uiManager.handoffs?.incoming ?? [];
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        Text(
          widget.strings.incomingHandoffs,
          style: Theme.of(context).textTheme.titleLarge,
        ),
        const SizedBox(height: 12),
        UiContentTransition(
          stateKey: incoming.isEmpty,
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              if (incoming.isEmpty) Text(widget.strings.noHandoffs),
              for (final handoff in incoming)
                Padding(
                  padding: const EdgeInsets.only(bottom: 8),
                  child: Card(
                    child: ListTile(
                      leading: const Icon(Icons.move_to_inbox_outlined),
                      title: Text(_handoffTitle(handoff)),
                      subtitle: Text(handoff.senderDeviceId),
                      trailing: Wrap(
                        spacing: 8,
                        children: [
                          FilledButton.tonal(
                            onPressed: () {
                              widget.uiManager.openIncoming(handoff);
                            },
                            child: Text(widget.strings.open),
                          ),
                          IconButton(
                            onPressed: () {
                              widget.uiManager.removeIncoming(handoff);
                            },
                            tooltip: widget.strings.remove,
                            icon: const Icon(Icons.close),
                          ),
                        ],
                      ),
                    ),
                  ),
                ),
            ],
          ),
        ),
      ],
    );
  }

  String _handoffTitle(UiIncomingHandoff handoff) {
    final url = handoff.payload.url;
    if (url != null) {
      return url;
    }
    final filePath = handoff.payload.filePath;
    if (filePath != null) {
      return filePath;
    }
    return handoff.payload.kind.name;
  }

  void _sendUrl() {
    widget.uiManager.sendUrl(_deviceId, _url.text);
  }

  void _sendYoutube() {
    widget.uiManager.sendYoutube(_deviceId, _url.text, _position.text);
  }

  void _sendVideo() {
    widget.uiManager.sendLocalVideo(_deviceId, _position.text);
  }

  void _sendFile() {
    widget.uiManager.sendFile(_deviceId);
  }
}
