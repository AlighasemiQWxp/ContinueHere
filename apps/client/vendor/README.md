# Slint Material components

`material/` contains the official Slint Material component sources from
[Slint v1.17.1](https://github.com/slint-ui/slint/tree/v1.17.1/ui-libraries/material/src).
They are imported through the `@material` library path in the client build script.
The MIT license and source copyright notices are retained.

Local adaptations:

- `ui/components/slider.slint`: enabled by default, explicit fractional `step`,
  correct minimum-offset conversion and stepping, zero-range protection, disabled
  input guards, and accessible step reporting. `value_changed` is emitted by user
  interaction only; programmatic media-position updates do not send another seek.
- `ui/components/text_field.slint`: expose the underlying input's horizontal
  alignment for English/Persian input.
- `ui/components/filled_button.slint` and `floating_action_button.slint`: forward
  focus to the interactive base; guard disabled button accessibility activation.
- `ui/components/radio_button.slint`: forward enabled state to the interactive
  area and guard disabled accessibility activation.

Application colors, wrappers, layout, icons, and feature callbacks live outside
the vendor directory. The ContinueHere PNG is the supplied application artwork;
the ICO contains the same artwork in Windows icon sizes, without a redesign.
