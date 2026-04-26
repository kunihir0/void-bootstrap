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

    // ── hostname ────────────────────────────────────────────────

    #[test]
    fn hostname_valid() {
        assert!(validate_hostname("voidlinux").is_ok());
        assert!(validate_hostname("my-host").is_ok());
        assert!(validate_hostname("a").is_ok());
    }

    #[test]
    fn hostname_rejects_empty() {
        assert!(validate_hostname("").is_err());
    }

    #[test]
    fn hostname_rejects_leading_dash() {
        assert!(validate_hostname("-bad").is_err());
    }

    #[test]
    fn hostname_rejects_trailing_dash() {
        assert!(validate_hostname("bad-").is_err());
    }

    #[test]
    fn hostname_rejects_special_chars() {
        assert!(validate_hostname("host.name").is_err());
        assert!(validate_hostname("host_name").is_err());
        assert!(validate_hostname("host name").is_err());
    }

    #[test]
    fn hostname_rejects_oversized() {
        let long = "a".repeat(254);
        assert!(validate_hostname(&long).is_err());
    }

    #[test]
    fn hostname_accepts_max_length() {
        let max = "a".repeat(253);
        assert!(validate_hostname(&max).is_ok());
    }

    // ── username ────────────────────────────────────────────────

    #[test]
    fn username_valid() {
        assert!(validate_username("baobao").is_ok());
        assert!(validate_username("_svc").is_ok());
        assert!(validate_username("user-1").is_ok());
        assert!(validate_username("a1_b2").is_ok());
    }

    #[test]
    fn username_rejects_uppercase_start() {
        assert!(validate_username("Admin").is_err());
    }

    #[test]
    fn username_rejects_digit_start() {
        assert!(validate_username("1user").is_err());
    }

    #[test]
    fn username_rejects_dash_start() {
        assert!(validate_username("-user").is_err());
    }

    #[test]
    fn username_rejects_oversized() {
        let long = "a".repeat(33);
        assert!(validate_username(&long).is_err());
    }

    #[test]
    fn username_accepts_max_length() {
        let max = "a".repeat(32);
        assert!(validate_username(&max).is_ok());
    }

    // ── locale ──────────────────────────────────────────────────

    #[test]
    fn locale_valid_standard() {
        assert!(validate_locale("en_US.UTF-8").is_ok());
        assert!(validate_locale("ja_JP.EUC-JP").is_ok());
        assert!(validate_locale("ko_KR.EUC-KR").is_ok());
    }

    #[test]
    fn locale_valid_special() {
        assert!(validate_locale("C").is_ok());
        assert!(validate_locale("POSIX").is_ok());
    }

    #[test]
    fn locale_rejects_invalid_encoding() {
        assert!(validate_locale("en_US.GARBAGE").is_err());
    }

    #[test]
    fn locale_rejects_bad_territory() {
        assert!(validate_locale("en_USXX.UTF-8").is_err());
    }

    #[test]
    fn locale_rejects_missing_territory() {
        assert!(validate_locale("en.UTF-8").is_err());
    }

    #[test]
    fn locale_rejects_empty() {
        assert!(validate_locale("").is_err());
    }

    // ── timezone ────────────────────────────────────────────────

    #[test]
    fn timezone_valid_with_real_path() {
        let tmpdir = std::env::temp_dir().join("tz_test");
        let _ = std::fs::create_dir_all(tmpdir.join("America"));
        std::fs::write(tmpdir.join("America/Phoenix"), "").unwrap();

        assert!(validate_timezone("America/Phoenix", &tmpdir).is_ok());

        let _ = std::fs::remove_dir_all(&tmpdir);
    }

    #[test]
    fn timezone_rejects_traversal() {
        let tmpdir = std::env::temp_dir().join("tz_test2");
        let _ = std::fs::create_dir_all(&tmpdir);

        assert!(validate_timezone("../../etc/passwd", &tmpdir).is_err());

        let _ = std::fs::remove_dir_all(&tmpdir);
    }

    #[test]
    fn timezone_rejects_nonexistent() {
        let tmpdir = std::env::temp_dir().join("tz_test3");
        let _ = std::fs::create_dir_all(&tmpdir);

        assert!(validate_timezone("Fake/Zone", &tmpdir).is_err());

        let _ = std::fs::remove_dir_all(&tmpdir);
    }
}
