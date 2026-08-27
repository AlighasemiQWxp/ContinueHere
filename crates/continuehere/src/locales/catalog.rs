use super::{Language, LocalizationKey};

pub(super) fn text(language: Language, key: LocalizationKey) -> &'static str {
    let english = english_text(key);
    let requested = match language {
        Language::English => Some(english),
        Language::Persian => persian_text(key),
    };
    with_english_fallback(requested, english)
}

fn english_text(key: LocalizationKey) -> &'static str {
    match key {
        LocalizationKey::ApplicationName => "ContinueHere",
        LocalizationKey::LanguageEnglish => "English",
        LocalizationKey::LanguagePersian => "Persian",
    }
}

fn persian_text(key: LocalizationKey) -> Option<&'static str> {
    match key {
        LocalizationKey::ApplicationName => None,
        LocalizationKey::LanguageEnglish => Some("انگلیسی"),
        LocalizationKey::LanguagePersian => Some("فارسی"),
    }
}

fn with_english_fallback(requested: Option<&'static str>, english: &'static str) -> &'static str {
    requested.unwrap_or(english)
}

#[cfg(test)]
mod tests {
    use super::text;
    use crate::locales::{Language, LocalizationKey};

    #[test]
    fn catalogs_return_english_and_persian_text() {
        assert_eq!(
            text(Language::English, LocalizationKey::LanguagePersian),
            "Persian"
        );
        assert_eq!(
            text(Language::Persian, LocalizationKey::LanguagePersian),
            "فارسی"
        );
    }

    #[test]
    fn untranslated_application_name_falls_back_to_english() {
        assert_eq!(
            text(Language::Persian, LocalizationKey::ApplicationName),
            "ContinueHere"
        );
    }
}
