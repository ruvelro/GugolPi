//! Utilidades de tiempo sin dependencias: marca UTC en RFC 3339.

use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Devuelve el instante actual en UTC con formato RFC 3339 (`2026-09-24T18:03:11Z`).
pub fn now_utc_rfc3339() -> String {
    let since_epoch = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::ZERO);
    utc_rfc3339_from_unix(since_epoch.as_secs())
}

/// Convierte segundos desde la época Unix a RFC 3339 en UTC.
///
/// Algoritmo de días civiles de Howard Hinnant, válido para cualquier fecha del calendario
/// gregoriano proléptico.
pub fn utc_rfc3339_from_unix(unix_seconds: u64) -> String {
    let days = unix_seconds / 86_400;
    let secs_of_day = unix_seconds % 86_400;
    let (year, month, day) = civil_from_days(i64::try_from(days).unwrap_or(i64::MAX));
    let hour = secs_of_day / 3_600;
    let minute = (secs_of_day % 3_600) / 60;
    let second = secs_of_day % 60;
    format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}Z")
}

fn civil_from_days(days_since_epoch: i64) -> (i64, u32, u32) {
    let z = days_since_epoch + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let year = if m <= 2 { y + 1 } else { y };
    (year, m, d)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn epoch_is_1970() {
        assert_eq!(utc_rfc3339_from_unix(0), "1970-01-01T00:00:00Z");
    }

    #[test]
    fn known_instant() {
        // 2026-09-24T12:34:56Z
        assert_eq!(utc_rfc3339_from_unix(1_790_253_296), "2026-09-24T12:34:56Z");
    }

    #[test]
    fn leap_day() {
        // 2024-02-29T00:00:00Z
        assert_eq!(utc_rfc3339_from_unix(1_709_164_800), "2024-02-29T00:00:00Z");
    }
}
