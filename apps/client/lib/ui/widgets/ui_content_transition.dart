import 'package:flutter/widgets.dart';

import '../ui_motion.dart';

class UiContentTransition extends StatelessWidget {
  const UiContentTransition({
    required this.stateKey,
    required this.child,
    super.key,
  });

  final Object stateKey;
  final Widget child;

  @override
  Widget build(BuildContext context) {
    return AnimatedSwitcher(
      duration: UiMotion.standard(context),
      switchInCurve: UiMotion.enterCurve,
      switchOutCurve: UiMotion.exitCurve,
      transitionBuilder: (child, animation) {
        return FadeTransition(
          opacity: animation,
          child: SizeTransition(
            sizeFactor: animation,
            alignment: AlignmentDirectional.topStart,
            child: child,
          ),
        );
      },
      child: KeyedSubtree(key: ValueKey(stateKey), child: child),
    );
  }
}
