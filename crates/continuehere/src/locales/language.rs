use super::TextDirection;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum Language {
    #[default]
    English,
    Persian,
}

impl Language {
    pub const fn code(self) -> &'static str {
        match self {
            Self::English => "en",
            Self::Persian => "fa",
        }
    }

    pub const fn text_direction(self) -> TextDirection {
        match self {
            Self::English => TextDirection::LeftToRight,
            Self::Persian => TextDirection::RightToLeft,
        }
    }

    pub const fn is_rtl(self) -> bool {
        self.text_direction().is_rtl()
    }
}

#[cfg(test)]
mod tests {
    use super::Language;
    use crate::locales::TextDirection;

    #[test]
    fn languages_expose_stable_codes_and_directions() {
        assert_eq!(Language::English.code(), "en");
        assert_eq!(
            Language::English.text_direction(),
            TextDirection::LeftToRight
        );
        assert!(!Language::English.is_rtl());
        assert_eq!(Language::Persian.code(), "fa");
        assert_eq!(
            Language::Persian.text_direction(),
            TextDirection::RightToLeft
        );
        assert!(Language::Persian.is_rtl());
    }
}
