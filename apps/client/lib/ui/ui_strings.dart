import '../src/rust/api/models.dart';

class UiStrings {
  const UiStrings(this.language);

  final UiLanguage language;

  String get appTitle => 'ContinueHere';
  String get devices => _text('Devices', 'دستگاه‌ها');
  String get send => _text('Send', 'ارسال');
  String get transfers => _text('Transfers', 'انتقال‌ها');
  String get settings => _text('Settings', 'تنظیمات');
  String get playback => _text('Playback', 'پخش');
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
  String get noVideo => _text('No video is open.', 'هیچ ویدیویی باز نیست.');
  String get dismiss => _text('Dismiss', 'بستن');

  String _text(String english, String persian) {
    if (language == UiLanguage.persian) {
      return persian;
    }
    return english;
  }
}
