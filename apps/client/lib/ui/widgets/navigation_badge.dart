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
          backgroundColor: Colors.transparent,
          padding: EdgeInsets.zero,
          largeSize: 10,
          label: Opacity(
            opacity: opacity,
            child: Transform.scale(
              scale: value,
              child: Container(
                width: 10,
                height: 10,
                decoration: BoxDecoration(
                  shape: BoxShape.circle,
                  gradient: const RadialGradient(
                    center: Alignment(-0.35, -0.45),
                    colors: [
                      Color(0xFFFFF3B0),
                      Color(0xFFFFD45A),
                      Color(0xFFE9A923),
                    ],
                    stops: [0, 0.5, 1],
                  ),
                  border: Border.all(
                    color: const Color(0xFFFFE69A),
                    width: 0.6,
                  ),
                  boxShadow: const [
                    BoxShadow(
                      color: Color(0x66FFBD35),
                      blurRadius: 9,
                      spreadRadius: 1,
                    ),
                    BoxShadow(color: Color(0x33FFDF76), blurRadius: 3),
                  ],
                ),
              ),
            ),
          ),
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
