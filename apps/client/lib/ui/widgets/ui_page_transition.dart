import 'package:flutter/widgets.dart';

import '../ui_motion.dart';

class UiPageTransition extends StatelessWidget {
  const UiPageTransition({required this.isRtl, required this.child, super.key});

  final bool isRtl;
  final Widget child;

  @override
  Widget build(BuildContext context) {
    var beginning = const Offset(0.025, 0);
    if (isRtl) {
      beginning = const Offset(-0.025, 0);
    }
    return AnimatedSwitcher(
      duration: UiMotion.standard(context),
      reverseDuration: UiMotion.quick(context),
      switchInCurve: UiMotion.enterCurve,
      switchOutCurve: UiMotion.exitCurve,
      transitionBuilder: (child, animation) {
        final position = Tween(
          begin: beginning,
          end: Offset.zero,
        ).animate(animation);
        return FadeTransition(
          opacity: animation,
          child: SlideTransition(position: position, child: child),
        );
      },
      child: child,
    );
  }
}
