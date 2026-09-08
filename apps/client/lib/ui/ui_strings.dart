import '../src/rust/api/models.dart';

class UiStrings {
  const UiStrings(this.language);

  final UiLanguage language;

  String get appTitle => 'ContinueHere';
  String get history => _text('History', 'تاریخچه');
  String get historyIntro => _text(
    'Your activity, organized by device.',
    'فعالیت‌های شما، به تفکیک دستگاه.',
  );
  String get noHistory => _text(
    'Your device history will appear here.',
    'تاریخچهٔ دستگاه‌های شما اینجا نمایش داده می‌شود.',
  );
  String get noDeviceHistory => _text(
    'No activity for this device.',
    'فعالیتی برای این دستگاه وجود ندارد.',
  );
  String get historyDevices => _text('All devices', 'همهٔ دستگاه‌ها');
  String get clearHistory => _text('Clear history', 'پاک کردن تاریخچه');
  String get clearHistoryMessage => _text(
    'Remove finished history entries? Your received files and active activities will stay.',
    'سوابق پایان‌یافته حذف شوند؟ فایل‌های دریافتی و فعالیت‌های جاری باقی می‌مانند.',
  );
  String get historyTimes => _text(
    'Times are shown in your local time zone, including seconds.',
    'زمان‌ها با منطقهٔ زمانی محلی شما و همراه ثانیه نمایش داده می‌شوند.',
  );
  String get historyStoredLocally => _text(
    'History is saved on this device.',
    'تاریخچه روی این دستگاه ذخیره می‌شود.',
  );
  String get historySent => _text('Sent', 'ارسال‌شده');
  String get historyReceived => _text('Received', 'دریافت‌شده');
  String get historyConnection => _text('Connection session', 'نشست اتصال');
  String get historyAll => _text('All activity', 'همهٔ فعالیت‌ها');
  String get historyFiles => _text('Files', 'فایل‌ها');
  String get historyConnections => _text('Connections', 'اتصال‌ها');
  String get activityStarted => _text('Activity started', 'شروع فعالیت');
  String get fileCompleted => _text('File completed', 'تکمیل فایل');
  String get activityEnded => _text('Activity ended', 'پایان فعالیت');
  String get sessionStarted => _text('Session started', 'شروع نشست');
  String get sessionEnded => _text('Session ended', 'پایان نشست');
  String get disconnectedAt => _text('Disconnected', 'قطع اتصال');
  String get historyEndUnknown => _text(
    'End time unknown — the app was interrupted.',
    'زمان پایان نامشخص است؛ اجرای برنامه متوقف شده بود.',
  );
  String get retry => _text('Retry', 'تلاش دوباره');
  String get retryAttempt => _text('Retry attempt', 'تلاش مجدد');
  String get retryExplanation => _text(
    'Starts a new attempt. Files are sent from the beginning and require acceptance again.',
    'تلاش تازه‌ای آغاز می‌شود. فایل از ابتدا ارسال می‌شود و به پذیرش دوباره نیاز دارد.',
  );
  String get lastActivity => _text('Last activity', 'آخرین فعالیت');
  String get historyActive => _text('In progress', 'در حال انجام');
  String get historyDelivered => _text('Delivered', 'تحویل داده شد');
  String get historyCompleted => _text('Completed', 'تکمیل شد');
  String get historyRejected => _text('Declined', 'رد شد');
  String get historyCancelled => _text('Cancelled', 'لغو شد');
  String get historyFailed => _text('Failed', 'ناموفق');
  String get historyDisconnected => _text('Disconnected', 'قطع شده');
  String get historyInterrupted => _text('Interrupted', 'متوقف شده');
  String get historyConnected => _text('Connected', 'متصل');
  String get historySaveFailed => _text(
    'History could not be saved. Recent activity may be lost when the app closes.',
    'ذخیرهٔ تاریخچه ناموفق بود. ممکن است فعالیت‌های اخیر با بستن برنامه از دست بروند.',
  );
  String get retryConnectFirst => _text(
    'Connect to this device before retrying.',
    'پیش از تلاش دوباره به این دستگاه متصل شوید.',
  );
  String get historyRemovedFile => _text(
    'This file may have been moved or removed.',
    'ممکن است این فایل جابه‌جا یا حذف شده باشد.',
  );
  String get devices => _text('Devices', 'دستگاه‌ها');
  String get send => _text('Send', 'ارسال');
  String get transfers => _text('Transfers', 'انتقال‌ها');
  String get settings => _text('Settings', 'تنظیمات');
  String get localDevice => _text('This device', 'این دستگاه');
  String get nearbyDevices => _text('Nearby devices', 'دستگاه‌های نزدیک');
  String get trustedDevices =>
      _text('Trusted devices', 'دستگاه‌های مورد اعتماد');
  String get noNearbyDevices =>
      _text('No nearby devices found.', 'هیچ دستگاه نزدیکی پیدا نشد.');
  String get noTrustedDevices =>
      _text('No trusted devices yet.', 'هنوز دستگاه مورد اعتمادی وجود ندارد.');
  String get pair => _text('Pair', 'جفت‌سازی');
  String get receivePairing =>
      _text('Receive pairing request', 'دریافت درخواست جفت‌سازی');
  String get pairingCode => _text('Verification code', 'کد تأیید');
  String get approve => _text('Approve', 'تأیید');
  String get reject => _text('Reject', 'رد کردن');
  String get cancel => _text('Cancel', 'لغو');
  String get manualEndpoint => _text('Manual endpoint', 'نشانی دستی');
  String get host => _text('Host or IP address', 'میزبان یا نشانی IP');
  String get port => _text('Port', 'درگاه');
  String get add => _text('Add', 'افزودن');
  String get connect => _text('Connect', 'اتصال');
  String get disconnect => _text('Disconnect', 'قطع اتصال');
  String get forget => _text('Forget', 'حذف اعتماد');
  String get destinationDevice => _text('Destination device', 'دستگاه مقصد');
  String get enterUrl => _text('Web address', 'نشانی وب');
  String get sendUrl => _text('Send URL', 'ارسال نشانی');
  String get sendYoutube => _text('Send YouTube video', 'ارسال ویدیوی یوتیوب');
  String get playbackPosition =>
      _text('Position (MM:SS)', 'موقعیت پخش (MM:SS)');
  String get sendVideo =>
      _text('Choose and send video', 'انتخاب و ارسال ویدیو');
  String get sendFile => _text('Choose and send file', 'انتخاب و ارسال فایل');
  String get incomingHandoffs => _text('Ready to continue', 'آماده ادامه دادن');
  String get noHandoffs => _text(
    'Nothing is waiting to be continued.',
    'موردی برای ادامه دادن وجود ندارد.',
  );
  String get open => _text('Open', 'باز کردن');
  String get remove => _text('Remove', 'حذف');
  String get accept => _text('Accept', 'پذیرفتن');
  String get chooseFolder => _text('Choose folder', 'انتخاب پوشه');
  String get decline => _text('Decline', 'رد کردن');
  String get noTransfers => _text(
    'No active or recent transfers.',
    'انتقال فعال یا اخیری وجود ندارد.',
  );
  String get deviceName => _text('Device name', 'نام دستگاه');
  String get save => _text('Save', 'ذخیره');
  String get destinationFolder =>
      _text('Received files folder', 'پوشه فایل‌های دریافتی');
  String get change => _text('Change', 'تغییر');
  String get languageLabel => _text('Language', 'زبان');
  String get english => _text('English', 'انگلیسی');
  String get persian => _text('Persian', 'فارسی');
  String get close => _text('Close', 'بستن');
  String get mediaLoadFailed =>
      _text('The image could not be displayed.', 'تصویر قابل نمایش نیست.');
  String get dismiss => _text('Dismiss', 'بستن');
  String get newActivity => _text('New activity', 'فعالیت جدید');

  String historyFailure(String failure) {
    switch (failure) {
      case 'NotConnected':
        return _text('The device was not connected.', 'دستگاه متصل نبود.');
      case 'Unsupported':
        return _text(
          'This device does not support this activity.',
          'این دستگاه از این فعالیت پشتیبانی نمی‌کند.',
        );
      case 'Invalid':
        return _text(
          'The content could not be sent.',
          'ارسال محتوا ممکن نبود.',
        );
      case 'Busy':
        return _text('The device was busy.', 'دستگاه مشغول بود.');
      case 'TimedOut':
        return _text(
          'The device did not respond in time.',
          'دستگاه به‌موقع پاسخ نداد.',
        );
      case 'Transport':
        return _text('The connection was interrupted.', 'اتصال قطع شد.');
      case 'Declined':
        return _text(
          'The recipient declined the file.',
          'گیرنده فایل را نپذیرفت.',
        );
      case 'DestinationConflict':
        return _text(
          'A file already exists at the destination.',
          'فایلی در مقصد از قبل وجود دارد.',
        );
      case 'Integrity':
        return _text(
          'The received file did not pass verification.',
          'تأیید صحت فایل دریافتی ناموفق بود.',
        );
      case 'FileSystem':
        return _text(
          'The file could not be read or saved.',
          'خواندن یا ذخیرهٔ فایل ممکن نبود.',
        );
      default:
        return _text(
          'The file transfer could not finish.',
          'انتقال فایل به پایان نرسید.',
        );
    }
  }

  String _text(String english, String persian) {
    if (language == UiLanguage.persian) {
      return persian;
    }
    return english;
  }
}
