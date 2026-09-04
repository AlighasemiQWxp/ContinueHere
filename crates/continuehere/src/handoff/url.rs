use url::Url;

use crate::transport::MAX_URL_SIZE;

use super::HandoffError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UrlHandoff {
    url: String,
}

impl UrlHandoff {
    pub fn new(url: &str) -> Result<Self, HandoffError> {
        Ok(Self {
            url: validate_url(url)?,
        })
    }

    pub fn url(&self) -> &str {
        &self.url
    }
}

pub(crate) fn validate_url(value: &str) -> Result<String, HandoffError> {
    if value.is_empty() {
        return Err(HandoffError::EmptyUrl);
    }
    if value.trim() != value {
        return Err(HandoffError::InvalidUrl);
    }
    if value.len() > MAX_URL_SIZE {
        return Err(HandoffError::UrlTooLarge);
    }
    let parsed = Url::parse(value).map_err(|_| HandoffError::InvalidUrl)?;
    if parsed.scheme() != "http" && parsed.scheme() != "https" {
        return Err(HandoffError::UnsupportedUrlScheme);
    }
    if parsed.host().is_none() {
        return Err(HandoffError::InvalidUrl);
    }
    if !parsed.username().is_empty() || parsed.password().is_some() {
        return Err(HandoffError::UrlContainsCredentials);
    }
    let normalized = parsed.to_string();
    if normalized.len() > MAX_URL_SIZE {
        return Err(HandoffError::UrlTooLarge);
    }
    Ok(normalized)
}

#[cfg(test)]
mod tests {
    use super::{HandoffError, validate_url};

    #[test]
    fn accepts_and_normalizes_http_urls() {
        assert_eq!(
            validate_url("https://example.com").expect("URL should be valid"),
            "https://example.com/"
        );
    }

    #[test]
    fn rejects_dangerous_schemes_and_credentials() {
        assert_eq!(
            validate_url("javascript:alert(1)"),
            Err(HandoffError::UnsupportedUrlScheme)
        );
        assert_eq!(
            validate_url("https://user:secret@example.com"),
            Err(HandoffError::UrlContainsCredentials)
        );
    }
}
