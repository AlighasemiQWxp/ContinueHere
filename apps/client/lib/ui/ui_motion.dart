import 'package:flutter/widgets.dart';

abstract final class UiMotion {
  static const Curve enterCurve = Curves.easeOutCubic;
  static const Curve exitCurve = Curves.easeInCubic;
  static const Curve badgeCurve = Curves.easeOutBack;

  static Duration quick(BuildContext context) {
    return _duration(context, const Duration(milliseconds: 150));
  }

  static Duration standard(BuildContext context) {
    return _duration(context, const Duration(milliseconds: 220));
  }

  static Duration media(BuildContext context) {
    return _duration(context, const Duration(milliseconds: 280));
  }

  static Duration _duration(BuildContext context, Duration duration) {
    if (MediaQuery.disableAnimationsOf(context)) {
      return Duration.zero;
    }
    return duration;
  }
}
