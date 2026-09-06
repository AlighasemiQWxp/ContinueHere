import 'package:continuehere_client/src/rust/api/models.dart';
import 'package:continuehere_client/ui/ui_strings.dart';
import 'package:continuehere_client/ui/ui_transition.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  test('UI transition reports destination changes once', () {
    var changes = 0;
    final transition = UiTransition(() {
      changes++;
    });

    transition.show(UiDestination.devices);
    transition.show(UiDestination.send);

    expect(transition.destination, UiDestination.send);
    expect(changes, 1);
  });

  test('Persian strings use Persian labels', () {
    const strings = UiStrings(UiLanguage.persian);

    expect(strings.settings, 'تنظیمات');
    expect(strings.devices, 'دستگاه‌ها');
  });
}
