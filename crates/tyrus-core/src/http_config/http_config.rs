use std::str::FromStr;
use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BodyLimit {
    Bytes(u64),
    Off,
}

pub const MIN_MAX_HEADER_BYTES: u64 = 8 * 1024;

pub fn parse_body_limit(value: &str) -> Result<BodyLimit, String> {
    let trimmed = value.trim();
    if trimmed.eq_ignore_ascii_case("off") {
        return Ok(BodyLimit::Off);
    }
    match bytesize::ByteSize::from_str(trimmed) {
        Ok(size) => Ok(BodyLimit::Bytes(size.as_u64())),
        Err(_) => Err(format!(
            "`{value}` is not a valid body size; use a byte size like `\"5mb\"`, `\"512kib\"`, or `\"off\"` to disable the limit"
        )),
    }
}

pub fn parse_time_window(value: &str) -> Result<Duration, String> {
    let trimmed = value.trim();
    let boundary = trimmed
        .find(|character: char| !character.is_ascii_digit())
        .ok_or_else(|| invalid_time_window(value))?;
    let (amount, unit) = trimmed.split_at(boundary);
    let amount: u64 = amount.parse().map_err(|_| invalid_time_window(value))?;
    let millis_per_unit = match unit.trim().to_ascii_lowercase().as_str() {
        "ms" | "milli" | "millis" | "millisecond" | "milliseconds" => 1,
        "s" | "sec" | "secs" | "second" | "seconds" => 1_000,
        "m" | "min" | "mins" | "minute" | "minutes" => 60_000,
        "h" | "hr" | "hrs" | "hour" | "hours" => 3_600_000,
        "d" | "day" | "days" => 86_400_000,
        _ => return Err(invalid_time_window(value)),
    };
    amount
        .checked_mul(millis_per_unit)
        .map(Duration::from_millis)
        .ok_or_else(|| format!("`{value}` overflows the supported duration range"))
}

fn invalid_time_window(value: &str) -> String {
    format!(
        "`{value}` is not a valid duration; use a value like `\"10s\"`, `\"1m\"`, `\"500ms\"`, or `\"2 hours\"`"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_body_limit_units_and_off() {
        assert_eq!(parse_body_limit("off"), Ok(BodyLimit::Off));
        assert_eq!(parse_body_limit("OFF"), Ok(BodyLimit::Off));
        assert_eq!(parse_body_limit("2mb"), Ok(BodyLimit::Bytes(2_000_000)));
        assert_eq!(parse_body_limit("2mib"), Ok(BodyLimit::Bytes(2_097_152)));
        assert_eq!(parse_body_limit("512kib"), Ok(BodyLimit::Bytes(524_288)));
        assert!(parse_body_limit("5 megabytes").is_err());
        assert!(parse_body_limit("nope").is_err());
        assert!(parse_body_limit("").is_err());
    }

    #[test]
    fn parses_time_window_compact_and_verbose() {
        assert_eq!(parse_time_window("500ms"), Ok(Duration::from_millis(500)));
        assert_eq!(parse_time_window("10s"), Ok(Duration::from_secs(10)));
        assert_eq!(parse_time_window("1m"), Ok(Duration::from_secs(60)));
        assert_eq!(parse_time_window("2h"), Ok(Duration::from_secs(7_200)));
        assert_eq!(parse_time_window("10 seconds"), Ok(Duration::from_secs(10)));
        assert_eq!(parse_time_window("1 minute"), Ok(Duration::from_secs(60)));
        assert!(parse_time_window("10").is_err());
        assert!(parse_time_window("m").is_err());
        assert!(parse_time_window("10 fortnights").is_err());
        assert!(parse_time_window("").is_err());
    }
}
