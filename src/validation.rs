use crate::types::VALID_ENCODINGS;
use std::path::Path;

pub(crate) fn validate_hostname(s: &str) -> std::result::Result<(), String> {
    if !s.is_empty()
        && s.len() <= 253
        && !s.starts_with('-')
        && !s.ends_with('-')
        && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
    {
        Ok(())
    } else {
        Err("Invalid hostname format. Please try again.".to_string())
    }
}

pub(crate) fn validate_username(u: &str) -> std::result::Result<(), String> {
    if u.chars()
        .next()
        .is_some_and(|c| c.is_ascii_lowercase() || c == '_')
        && u.chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '-')
        && u.len() <= 32
    {
        Ok(())
    } else {
        Err("Invalid username format. Try again.".to_string())
    }
}

pub(crate) fn validate_locale(s: &str) -> std::result::Result<(), String> {
    let valid = matches!(s, "C" | "POSIX")
        || s.split_once('_')
            .and_then(|(lang, rest)| {
                rest.split_once('.').map(|(terr, enc)| {
                    lang.len() == 2
                        && lang.chars().all(|c| c.is_ascii_lowercase())
                        && terr.len() == 2
                        && terr.chars().all(|c| c.is_ascii_uppercase())
                        && VALID_ENCODINGS.contains(&enc)
                })
            })
            .unwrap_or(false);

    if valid {
        Ok(())
    } else {
        Err("Invalid locale format (e.g., en_US.UTF-8). Please try again.".to_string())
    }
}

pub(crate) fn validate_timezone(tz: &str, zoneinfo_root: &Path) -> std::result::Result<(), String> {
    let tz_path = zoneinfo_root.join(tz);
    if tz_path.exists()
        && tz
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "/_-+".contains(c))
        && !tz.contains("..")
    {
        Ok(())
    } else {
        Err("Timezone not found or invalid format. Please try again.".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_locale_validation() {
        assert!(validate_locale("en_US.UTF-8").is_ok());
        assert!(validate_locale("C").is_ok());
        assert!(validate_locale("POSIX").is_ok());
        assert!(validate_locale("ja_JP.EUC-JP").is_ok());

        assert!(validate_locale("en_US.GARBAGE").is_err()); // Invalid encoding
        assert!(validate_locale("en_USXX.UTF-8").is_err()); // Territory too long
        assert!(validate_locale("en.UTF-8").is_err()); // Missing territory
        assert!(validate_locale("").is_err()); // Empty
    }
}
