use crate::error::AppError;
use base64::prelude::*;
use base64::Engine;
use url::Url;

/// Validate that a string is a valid HTTP(S) URL
pub fn validate_url(url_str: &str, field_name: &str) -> Result<(), AppError> {
    let url = Url::parse(url_str)
        .map_err(|e| AppError::InvalidInput(format!("Invalid URL for '{field_name}': {e}")))?;

    let scheme = url.scheme();
    if scheme != "http" && scheme != "https" {
        return Err(AppError::InvalidInput(format!(
            "Invalid URL scheme for '{field_name}': must be http or https, got '{scheme}'"
        )));
    }

    Ok(())
}

/// Infer homepage URL from API endpoint
///
/// Examples:
/// - https://api.anthropic.com/v1 → https://anthropic.com
/// - https://api.openai.com/v1 → https://openai.com
/// - https://api-test.company.com/v1 → https://company.com
pub fn infer_homepage_from_endpoint(endpoint: &str) -> Option<String> {
    let url = Url::parse(endpoint).ok()?;
    let host = url.host_str()?;

    // Remove common API prefixes
    let clean_host = host
        .strip_prefix("api.")
        .or_else(|| host.strip_prefix("api-"))
        .unwrap_or(host);

    Some(format!("https://{clean_host}"))
}

/// Decode Base64 parameter from deeplink URL with tolerance for common issues
pub fn decode_base64_param(field: &str, raw: &str) -> Result<Vec<u8>, AppError> {
    let mut candidates: Vec<String> = Vec::new();
    // Remove line breaks while preserving spaces (for restoring '+')
    let trimmed = raw.trim_matches(|c| c == '\r' || c == '\n');

    // Try replacing spaces with '+' first (URL decoding may convert '+' to space)
    if trimmed.contains(' ') {
        let replaced = trimmed.replace(' ', "+");
        if !replaced.is_empty() && !candidates.contains(&replaced) {
            candidates.push(replaced);
        }
    }

    // Original value (after replaced version)
    if !trimmed.is_empty() && !candidates.contains(&trimmed.to_string()) {
        candidates.push(trimmed.to_string());
    }

    // Add padding if missing
    let existing = candidates.clone();
    for candidate in existing {
        let mut padded = candidate.clone();
        let remainder = padded.len() % 4;
        if remainder != 0 {
            padded.extend(std::iter::repeat_n('=', 4 - remainder));
        }
        if !candidates.contains(&padded) {
            candidates.push(padded);
        }
    }

    let mut last_error: Option<String> = None;
    for candidate in candidates {
        for engine in [
            &BASE64_STANDARD,
            &BASE64_STANDARD_NO_PAD,
            &BASE64_URL_SAFE,
            &BASE64_URL_SAFE_NO_PAD,
        ] {
            match engine.decode(&candidate) {
                Ok(bytes) => return Ok(bytes),
                Err(err) => last_error = Some(err.to_string()),
            }
        }
    }

    Err(AppError::InvalidInput(format!(
        "{field} 参数 Base64 解码失败：{}。请确认链接参数已用 Base64 编码并经过 URL 转义（尤其是将 '+' 编码为 %2B，或使用 URL-safe Base64）。",
        last_error.unwrap_or_else(|| "未知错误".to_string())
    )))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_invalid_url() {
        let result = validate_url("not-a-url", "test");
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_invalid_scheme() {
        let result = validate_url("ftp://example.com", "test");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("must be http or https"));
    }

    #[test]
    fn test_infer_homepage() {
        assert_eq!(
            infer_homepage_from_endpoint("https://api.anthropic.com/v1"),
            Some("https://anthropic.com".to_string())
        );
        assert_eq!(
            infer_homepage_from_endpoint("https://api-test.company.com/v1"),
            Some("https://test.company.com".to_string())
        );
        assert_eq!(
            infer_homepage_from_endpoint("https://example.com"),
            Some("https://example.com".to_string())
        );
    }
}
