use crate::error::AppError;
use crate::provider::Provider;
use crate::settings;
use super::types::GeminiAuthType;

pub struct GeminiAuthDetector;

impl GeminiAuthDetector {
    const PACKYCODE_SECURITY_SELECTED_TYPE: &'static str = "gemini-api-key";
    const GOOGLE_OAUTH_SECURITY_SELECTED_TYPE: &'static str = "oauth-personal";

    const PACKYCODE_PARTNER_KEY: &'static str = "packycode";
    const GOOGLE_OFFICIAL_PARTNER_KEY: &'static str = "google-official";

    const PACKYCODE_KEYWORDS: [&'static str; 3] = ["packycode", "packyapi", "packy"];

    /// Detect Gemini provider authentication type
    pub fn detect_gemini_auth_type(provider: &Provider) -> GeminiAuthType {
        if let Some(key) = provider
            .meta
            .as_ref()
            .and_then(|meta| meta.partner_promotion_key.as_deref())
        {
            if key.eq_ignore_ascii_case(Self::GOOGLE_OFFICIAL_PARTNER_KEY) {
                return GeminiAuthType::GoogleOfficial;
            }
            if key.eq_ignore_ascii_case(Self::PACKYCODE_PARTNER_KEY) {
                return GeminiAuthType::Packycode;
            }
        }

        let name_lower = provider.name.to_ascii_lowercase();
        if name_lower == "google" || name_lower.starts_with("google ") {
            return GeminiAuthType::GoogleOfficial;
        }

        if Self::contains_packycode_keyword(&provider.name) {
            return GeminiAuthType::Packycode;
        }

        if let Some(site) = provider.website_url.as_deref() {
            if Self::contains_packycode_keyword(site) {
                return GeminiAuthType::Packycode;
            }
        }

        if let Some(base_url) = provider
            .settings_config
            .pointer("/env/GOOGLE_GEMINI_BASE_URL")
            .and_then(|v| v.as_str())
        {
            if Self::contains_packycode_keyword(base_url) {
                return GeminiAuthType::Packycode;
            }
        }

        GeminiAuthType::Generic
    }

    fn contains_packycode_keyword(value: &str) -> bool {
        let lower = value.to_ascii_lowercase();
        Self::PACKYCODE_KEYWORDS
            .iter()
            .any(|keyword| lower.contains(keyword))
    }

    fn is_packycode_gemini(provider: &Provider) -> bool {
        if provider
            .meta
            .as_ref()
            .and_then(|meta| meta.partner_promotion_key.as_deref())
            .is_some_and(|key| key.eq_ignore_ascii_case(Self::PACKYCODE_PARTNER_KEY))
        {
            return true;
        }

        if Self::contains_packycode_keyword(&provider.name) {
            return true;
        }

        if let Some(site) = provider.website_url.as_deref() {
            if Self::contains_packycode_keyword(site) {
                return true;
            }
        }

        if let Some(base_url) = provider
            .settings_config
            .pointer("/env/GOOGLE_GEMINI_BASE_URL")
            .and_then(|v| v.as_str())
        {
            if Self::contains_packycode_keyword(base_url) {
                return true;
            }
        }

        false
    }

    fn is_google_official_gemini(provider: &Provider) -> bool {
        if provider
            .meta
            .as_ref()
            .and_then(|meta| meta.partner_promotion_key.as_deref())
            .is_some_and(|key| key.eq_ignore_ascii_case(Self::GOOGLE_OFFICIAL_PARTNER_KEY))
        {
            return true;
        }

        let name_lower = provider.name.to_ascii_lowercase();
        name_lower == "google" || name_lower.starts_with("google ")
    }

    /// Ensure PackyCode Gemini provider security flag is set correctly
    pub fn ensure_packycode_security_flag(provider: &Provider) -> Result<(), AppError> {
        if !Self::is_packycode_gemini(provider) {
            return Ok(());
        }

        settings::ensure_security_auth_selected_type(Self::PACKYCODE_SECURITY_SELECTED_TYPE)?;

        use crate::gemini_config::write_packycode_settings;
        write_packycode_settings()?;

        Ok(())
    }

    /// Ensure Google Official Gemini provider security flag is set correctly
    pub fn ensure_google_oauth_security_flag(provider: &Provider) -> Result<(), AppError> {
        if !Self::is_google_official_gemini(provider) {
            return Ok(());
        }

        settings::ensure_security_auth_selected_type(Self::GOOGLE_OAUTH_SECURITY_SELECTED_TYPE)?;

        use crate::gemini_config::write_google_oauth_settings;
        write_google_oauth_settings()?;

        Ok(())
    }
}
