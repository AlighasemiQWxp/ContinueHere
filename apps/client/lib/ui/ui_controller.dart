import '../platform/platform_manager.dart';
import '../src/rust/api/manager.dart';
import 'controllers/devices_ui_controller.dart';
import 'controllers/handoff_ui_controller.dart';
import 'controllers/pairing_ui_controller.dart';
import 'controllers/playback_ui_controller.dart';
import 'controllers/settings_ui_controller.dart';
import 'controllers/transfer_ui_controller.dart';

class UiController {
  UiController(
    UiBridge bridge,
    PlatformManager platform,
    void Function() onChanged,
    void Function(Object) onError,
  ) : devices = DevicesUiController(bridge, onChanged, onError),
      pairing = PairingUiController(bridge, onChanged, onError),
      handoff = HandoffUiController(bridge, platform, onChanged, onError),
      transfer = TransferUiController(bridge, platform, onChanged, onError),
      playback = PlaybackUiController(onChanged, onError),
      settings = SettingsUiController(bridge, platform, onChanged, onError);

  final DevicesUiController devices;
  final PairingUiController pairing;
  final HandoffUiController handoff;
  final TransferUiController transfer;
  final PlaybackUiController playback;
  final SettingsUiController settings;

  Future<void> start() async {
    await devices.start();
    await pairing.start();
    await handoff.start();
    await transfer.start();
    await settings.start();
  }

  Future<void> dispose() async {
    try {
      await settings.dispose();
    } finally {
      try {
        await playback.dispose();
      } finally {
        try {
          await transfer.dispose();
        } finally {
          try {
            await handoff.dispose();
          } finally {
            try {
              await pairing.dispose();
            } finally {
              await devices.dispose();
            }
          }
        }
      }
    }
  }
}
