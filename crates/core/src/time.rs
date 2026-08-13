use std::time::{SystemTime, UNIX_EPOCH};

/// 現在時刻を ISO 8601 (UTC) で返す
pub fn now_iso8601() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format_unix_secs(secs)
}

/// UNIX 秒 → ISO 8601 (UTC) 文字列
pub fn format_unix_secs(secs: u64) -> String {
    let days = secs / 86_400;
    let rem = secs % 86_400;
    let hours = rem / 3600;
    let mins = (rem % 3600) / 60;
    let sec = rem % 60;
    let (year, month, day) = days_to_ymd(days);
    format!("{year:04}-{month:02}-{day:02}T{hours:02}:{mins:02}:{sec:02}Z")
}

/// UNIX epoch からの日数 → (year, month, day)
/// Howard Hinnant の civil_from_days アルゴリズム
pub fn days_to_ymd(days: u64) -> (u64, u64, u64) {
    let z = days as i64 + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y as u64, m, d)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn epoch_is_1970_01_01() {
        assert_eq!(days_to_ymd(0), (1970, 1, 1));
    }

    #[test]
    fn day_one_is_1970_01_02() {
        assert_eq!(days_to_ymd(1), (1970, 1, 2));
    }

    #[test]
    fn year_2000() {
        // 1970-01-01 から 2000-01-01 まで 10957 日
        assert_eq!(days_to_ymd(10_957), (2000, 1, 1));
    }

    #[test]
    fn leap_year_feb_29() {
        // 2024-02-29 は 1970-01-01 から 19782 日
        assert_eq!(days_to_ymd(19_782), (2024, 2, 29));
    }

    #[test]
    fn roundtrip_format() {
        assert_eq!(format_unix_secs(0), "1970-01-01T00:00:00Z");
        assert_eq!(format_unix_secs(86_400), "1970-01-02T00:00:00Z");
    }
}
