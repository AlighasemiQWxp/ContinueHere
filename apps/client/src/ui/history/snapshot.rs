use continuehere::{Activity, ActivityDirection, ActivityKind, ActivityStatus};

use crate::ui::{ContentRow, shared::text};

use super::HistoryAccess;

pub(super) fn activity_time(item: &Activity) -> u64 {
    [
        Some(item.started_at()),
        item.ended_at(),
        item.completed_at(),
        item.file_completed_at(),
        item.disconnected_at(),
    ]
    .into_iter()
    .flatten()
    .max()
    .unwrap_or(item.started_at())
}

pub(super) fn timestamp(value: u64) -> String {
    i64::try_from(value)
        .ok()
        .and_then(chrono::DateTime::from_timestamp_millis)
        .map(|time| {
            time.with_timezone(&chrono::Local)
                .format("%Y-%m-%d  %H:%M:%S%.3f UTC%:z")
                .to_string()
        })
        .unwrap_or_else(|| value.to_string())
}

pub(super) fn can_open(item: &Activity) -> bool {
    matches!(
        item.status(),
        ActivityStatus::Completed | ActivityStatus::Delivered
    ) && (item.file_path().is_some() || item.url().is_some())
}

pub(super) fn activity_row(item: &Activity, access: &HistoryAccess, rtl: bool) -> ContentRow {
    let session = item.kind() == ActivityKind::Session;
    let direction = match item.direction() {
        ActivityDirection::Outgoing => text(rtl, "Sent", "ارسالی"),
        ActivityDirection::Incoming => text(rtl, "Received", "دریافتی"),
        ActivityDirection::Connection => text(rtl, "Connection", "اتصال"),
    };
    let mut detail = format!(
        "{}\n{}: {}",
        direction,
        text(rtl, "Started", "شروع"),
        timestamp(item.started_at())
    );
    for (label, value) in [
        (
            text(rtl, "File completed", "تکمیل فایل"),
            item.file_completed_at(),
        ),
        (
            text(rtl, "Disconnected", "قطع اتصال"),
            item.disconnected_at(),
        ),
        (text(rtl, "Ended", "پایان"), item.ended_at()),
    ] {
        if let Some(value) = value {
            detail.push_str(&format!("\n{label}: {}", timestamp(value)));
        }
    }
    if item.status() == ActivityStatus::Interrupted && item.ended_at().is_none() {
        detail.push_str(text(
            rtl,
            "\nEnd time unknown after unexpected exit",
            "\nزمان پایان پس از خروج غیرمنتظره نامشخص است",
        ));
    }
    if item.retry_of().is_some() {
        detail.push_str(text(rtl, "\nRetry attempt", "\nتلاش دوباره"));
    }
    if let Some(continuation) = item.document_continuation() {
        let (label, position) = match continuation {
            continuehere::DocumentContinuation::PdfPage(page) => (text(rtl, "Page", "صفحه"), page),
            continuehere::DocumentContinuation::PowerPointSlide(slide) => {
                (text(rtl, "Slide", "اسلاید"), slide)
            }
        };
        detail.push_str(&format!("\n{label}: {position}"));
    }
    if let Some(failure) = item.failure() {
        detail.push_str(&format!("\n{failure}"));
    }
    let status = match item.status() {
        ActivityStatus::Active => text(rtl, "Active", "فعال"),
        ActivityStatus::Delivered => text(rtl, "Delivered", "تحویل داده شد"),
        ActivityStatus::Completed => text(rtl, "Completed", "تکمیل شد"),
        ActivityStatus::Rejected => text(rtl, "Rejected", "رد شد"),
        ActivityStatus::Cancelled => text(rtl, "Cancelled", "لغو شد"),
        ActivityStatus::Failed => text(rtl, "Failed", "ناموفق"),
        ActivityStatus::Disconnected => text(rtl, "Disconnected", "قطع شده"),
        ActivityStatus::Interrupted => text(rtl, "Interrupted", "متوقف شده"),
    };
    ContentRow {
        id: item.id().into(),
        title: if session {
            text(rtl, "Connection session", "نشست اتصال").into()
        } else {
            item.title().into()
        },
        detail: detail.into(),
        status: status.into(),
        can_open: can_open(item),
        retryable: item.can_retry(),
        can_retry: item.can_retry()
            && access
                .transport()
                .connections()
                .iter()
                .any(|connection| connection.device_id().as_str() == item.device_id()),
        active: item.status() == ActivityStatus::Active,
        ..Default::default()
    }
}
