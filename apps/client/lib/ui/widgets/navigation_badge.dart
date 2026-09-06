import 'package:flutter/material.dart';

import '../ui_motion.dart';

class NavigationBadge extends StatelessWidget {
  const NavigationBadge({
    required this.visible,
    required this.semanticsLabel,
    required this.child,
    super.key,
  });

  final bool visible;
  final String semanticsLabel;
  final Widget child;

  @override
  Widget build(BuildContext context) {
    var target = 0.0;
    if (visible) {
      target = 1.0;
    }
    final badge = TweenAnimationBuilder<double>(
      tween: Tween(end: target),
      duration: UiMotion.quick(context),
      curve: UiMotion.badgeCurve,
      child: child,
      builder: (context, value, child) {
        var opacity = value;
        if (opacity < 0) {
          opacity = 0;
        }
        if (opacity > 1) {
          opacity = 1;
        }
        return Badge(
          isLabelVisible: value > 0,
          smallSize: 7 * value,
          backgroundColor: Theme.of(context).colorScheme.tertiary
              .withValues(alpha: opacity),
          child: child,
        );
      },
    );
    if (!visible) {
      return badge;
    }
    return Semantics(label: semanticsLabel, child: badge);
  }
}
