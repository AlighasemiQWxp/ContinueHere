import 'package:flutter/material.dart';

import '../../src/rust/api/models.dart';
import '../ui_manager.dart';
import '../ui_strings.dart';
import 'screen_frame.dart';

class SettingsScreen extends StatefulWidget {
  const SettingsScreen({
    required this.uiManager,
    required this.strings,
    super.key,
  });

  final UiManager uiManager;
  final UiStrings strings;

  @override
  State<SettingsScreen> createState() => _SettingsScreenState();
}

class _SettingsScreenState extends State<SettingsScreen> {
  late final TextEditingController _displayName;

  @override
  void initState() {
    super.initState();
    _displayName = TextEditingController(
      text: widget.uiManager.settings?.displayName ?? '',
    );
  }

  @override
  void dispose() {
    _displayName.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final settings = widget.uiManager.settings;
    return ScreenFrame(
      title: widget.strings.settings,
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          TextField(
            controller: _displayName,
            decoration: InputDecoration(labelText: widget.strings.deviceName),
            onSubmitted: widget.uiManager.changeDisplayName,
          ),
          const SizedBox(height: 12),
          Align(
            alignment: AlignmentDirectional.centerStart,
            child: FilledButton(
              onPressed: () {
                widget.uiManager.changeDisplayName(_displayName.text);
              },
              child: Text(widget.strings.save),
            ),
          ),
          const SizedBox(height: 32),
          Text(
            widget.strings.destinationFolder,
            style: Theme.of(context).textTheme.titleMedium,
          ),
          const SizedBox(height: 8),
          SelectableText(settings?.defaultTransferDirectory ?? ''),
          const SizedBox(height: 12),
          Align(
            alignment: AlignmentDirectional.centerStart,
            child: OutlinedButton.icon(
              onPressed: widget.uiManager.chooseDefaultDirectory,
              icon: const Icon(Icons.folder_open),
              label: Text(widget.strings.change),
            ),
          ),
          const SizedBox(height: 32),
          Text(
            widget.strings.languageLabel,
            style: Theme.of(context).textTheme.titleMedium,
          ),
          const SizedBox(height: 8),
          SegmentedButton<UiLanguage>(
            segments: [
              ButtonSegment(
                value: UiLanguage.english,
                label: Text(widget.strings.english),
              ),
              ButtonSegment(
                value: UiLanguage.persian,
                label: Text(widget.strings.persian),
              ),
            ],
            selected: {settings?.language ?? UiLanguage.english},
            onSelectionChanged: (selection) {
              widget.uiManager.changeLanguage(selection.first);
            },
          ),
        ],
      ),
    );
  }
}
