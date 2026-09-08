import '../platform/platform_manager.dart';
import '../src/rust/api/manager.dart';
import 'controllers/devices_ui_controller.dart';
import 'controllers/activity_ui_controller.dart';
import 'controllers/file_preview_ui_controller.dart';
import 'controllers/handoff_ui_controller.dart';
import 'controllers/pairing_ui_controller.dart';
import 'controllers/settings_ui_controller.dart';
import 'controllers/transfer_ui_controller.dart';
import 'ui_notifications.dart';
import 'ui_transition.dart';

class UiController {
  UiController(
    UiBridge bridge,
    PlatformManager platform,
    void Function() onChanged,
    void Function(Object) onError,
    UiDestinationActivityDelegate onActivity,
    UiDeviceActivityDelegate onHistoryActivity,
  ) : activity = ActivityUiController(
        bridge,
        onChanged,
        onError,
        onHistoryActivity,
      ),
      devices = DevicesUiController(
        bridge,
        onChanged,
        onError,
        () => onActivity(UiDestination.devices),
      ),
      pairing = PairingUiController(
        bridge,
        onChanged,
        onError,
        () => onActivity(UiDestination.devices),
      ),
      handoff = HandoffUiController(
        bridge,
        platform,
        onChanged,
        onError,
        () => onActivity(UiDestination.send),
      ),
      transfer = TransferUiController(
        bridge,
        platform,
        onChanged,
        onError,
        () => onActivity(UiDestination.transfers),
      ),
      filePreview = FilePreviewUiController(onChanged, onError),
      settings = SettingsUiController(
        bridge,
        platform,
        onChanged,
        onError,
        () => onActivity(UiDestination.settings),
      );

  final DevicesUiController devices;
  final ActivityUiController activity;
  final PairingUiController pairing;
  final HandoffUiController handoff;
  final TransferUiController transfer;
  final FilePreviewUiController filePreview;
  final SettingsUiController settings;

  Future<void> start() async {
    await activity.start();
    await devices.start();
    await pairing.start();
    await handoff.start();
    await transfer.start();
    await settings.start();
  }

  Future<void> dispose() async {
    try {
      try {
        await activity.dispose();
      } finally {
        await settings.dispose();
      }
    } finally {
      try {
        await filePreview.dispose();
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
