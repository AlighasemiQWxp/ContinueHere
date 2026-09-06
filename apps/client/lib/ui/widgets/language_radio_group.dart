import 'package:flutter/material.dart';

import '../../src/rust/api/models.dart';
import '../ui_strings.dart';

typedef UiLanguageSelectedDelegate = void Function(UiLanguage language);

class LanguageRadioGroup extends StatelessWidget {
  const LanguageRadioGroup({
    required this.language,
    required this.strings,
    required this.onSelected,
    super.key,
  });

  final UiLanguage language;
  final UiStrings strings;
  final UiLanguageSelectedDelegate onSelected;

  @override
  Widget build(BuildContext context) {
    return RadioGroup<UiLanguage>(
      groupValue: language,
      onChanged: (value) {
        if (value == null) {
          return;
        }
        onSelected(value);
      },
      child: Column(
        children: [
          RadioListTile<UiLanguage>(
            value: UiLanguage.english,
            title: Text(strings.english),
            selected: language == UiLanguage.english,
            contentPadding: EdgeInsets.zero,
          ),
          RadioListTile<UiLanguage>(
            value: UiLanguage.persian,
            title: Text(strings.persian),
            selected: language == UiLanguage.persian,
            contentPadding: EdgeInsets.zero,
          ),
        ],
      ),
    );
  }
}
