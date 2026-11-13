use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

#[cfg(kani)]
#[path = "kani_proofs/kani_proofs.rs"]
mod kani_proofs;

pub fn is_email(value: &str) -> bool {
    if value.is_empty()
        || value
            .bytes()
            .any(|byte| byte.is_ascii_whitespace() || byte.is_ascii_control())
        || value.matches('@').count() != 1
    {
        return false;
    }
    let (local, domain) = value.split_once('@').unwrap();
    if local.is_empty()
        || domain.is_empty()
        || local.starts_with('.')
        || local.ends_with('.')
        || domain.starts_with('.')
        || domain.ends_with('.')
        || local.contains("..")
        || domain.contains("..")
        || !domain.contains('.')
    {
        return false;
    }
    if !local.bytes().all(|byte| {
        byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'%' | b'+' | b'-')
    }) {
        return false;
    }
    let mut labels = domain.split('.');
    let Some(tld) = labels.next_back() else {
        return false;
    };
    if tld.len() < 2 {
        return false;
    }
    domain.split('.').all(|label| {
        !label.is_empty()
            && !label.starts_with('-')
            && !label.ends_with('-')
            && label
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
    })
}

pub fn is_url(value: &str) -> bool {
    let Some((scheme, rest)) = value.split_once("://") else {
        return false;
    };
    let valid_scheme = matches!(scheme, "http" | "https");
    let host = rest.split('/').next().unwrap_or_default();
    valid_scheme && !host.is_empty() && !host.chars().any(char::is_whitespace)
}

pub fn is_uuid(value: &str, version: Option<i64>) -> bool {
    let bytes = value.as_bytes();
    if bytes.len() != 36 {
        return false;
    }
    for index in [8, 13, 18, 23] {
        if bytes[index] != b'-' {
            return false;
        }
    }
    let hex_positions = (0..bytes.len()).filter(|index| ![8, 13, 18, 23].contains(index));
    if !hex_positions
        .into_iter()
        .all(|index| bytes[index].is_ascii_hexdigit())
    {
        return false;
    }
    match version {
        Some(version) => (1..=8).contains(&version) && bytes[14] == b'0' + version as u8,
        None => true,
    }
}

pub fn is_ip(value: &str) -> bool {
    value.parse::<IpAddr>().is_ok()
}

pub fn is_ipv4(value: &str) -> bool {
    value.parse::<Ipv4Addr>().is_ok()
}

pub fn is_ipv6(value: &str) -> bool {
    value.parse::<Ipv6Addr>().is_ok()
}

pub fn is_e164(value: &str) -> bool {
    let Some(number) = value.strip_prefix('+') else {
        return false;
    };
    (1..=15).contains(&number.len()) && number.bytes().all(|byte| byte.is_ascii_digit())
}

pub fn is_cep(value: &str) -> bool {
    has_ascii_digits(value, 8)
        || (value.len() == 9
            && has_ascii_digits(&value[..5], 5)
            && &value[5..6] == "-"
            && has_ascii_digits(&value[6..], 3))
}

pub fn is_cpf(value: &str) -> bool {
    let Some(digits) = normalize_cpf(value) else {
        return false;
    };
    if digits.windows(2).all(|window| window[0] == window[1]) {
        return false;
    }
    let first = cpf_check_digit(&digits[..9], 10);
    let second = cpf_check_digit(&digits[..10], 11);
    digits[9] == first && digits[10] == second
}

pub fn is_cnpj(value: &str) -> bool {
    let Some(chars) = normalize_cnpj(value) else {
        return false;
    };
    let first = cnpj_check_digit(&chars[..12], &[5, 4, 3, 2, 9, 8, 7, 6, 5, 4, 3, 2]);
    let mut second_input = chars[..12].to_vec();
    second_input.push((first + b'0') as char);
    let second = cnpj_check_digit(&second_input, &[6, 5, 4, 3, 2, 9, 8, 7, 6, 5, 4, 3, 2]);
    cnpj_digit_value(chars[12]) == Some(first) && cnpj_digit_value(chars[13]) == Some(second)
}

fn has_ascii_digits(value: &str, count: usize) -> bool {
    value.len() == count && value.bytes().all(|byte| byte.is_ascii_digit())
}

fn normalize_cpf(value: &str) -> Option<Vec<u8>> {
    let bytes = value.as_bytes();
    if bytes.len() == 11 && bytes.iter().all(u8::is_ascii_digit) {
        return Some(bytes.iter().map(|byte| byte - b'0').collect());
    }
    if bytes.len() == 14
        && bytes[3] == b'.'
        && bytes[7] == b'.'
        && bytes[11] == b'-'
        && bytes
            .iter()
            .enumerate()
            .all(|(index, byte)| matches!(index, 3 | 7 | 11) || byte.is_ascii_digit())
    {
        return Some(
            bytes
                .iter()
                .filter(|byte| byte.is_ascii_digit())
                .map(|byte| byte - b'0')
                .collect(),
        );
    }
    None
}

fn cpf_check_digit(digits: &[u8], first_weight: u8) -> u8 {
    let sum: u32 = digits
        .iter()
        .enumerate()
        .map(|(index, digit)| u32::from(*digit) * u32::from(first_weight - index as u8))
        .sum();
    let remainder = sum % 11;
    if remainder < 2 {
        0
    } else {
        (11 - remainder) as u8
    }
}

fn normalize_cnpj(value: &str) -> Option<Vec<char>> {
    let bytes = value.as_bytes();
    let raw = if bytes.len() == 14 {
        if !bytes[..12].iter().all(u8::is_ascii_alphanumeric)
            || !bytes[12..].iter().all(u8::is_ascii_digit)
        {
            return None;
        }
        value.to_string()
    } else if bytes.len() == 18
        && bytes[2] == b'.'
        && bytes[6] == b'.'
        && bytes[10] == b'/'
        && bytes[15] == b'-'
    {
        let mut raw = String::with_capacity(14);
        for (index, byte) in bytes.iter().enumerate() {
            if matches!(index, 2 | 6 | 10 | 15) {
                continue;
            }
            if raw.len() < 12 {
                if !byte.is_ascii_alphanumeric() {
                    return None;
                }
            } else if !byte.is_ascii_digit() {
                return None;
            }
            raw.push(*byte as char);
        }
        raw
    } else {
        return None;
    };
    Some(
        raw.chars()
            .map(|character| character.to_ascii_uppercase())
            .collect(),
    )
}

fn cnpj_check_digit(chars: &[char], weights: &[u8]) -> u8 {
    let sum: u32 = chars
        .iter()
        .zip(weights)
        .map(|(character, weight)| u32::from(cnpj_base_value(*character)) * u32::from(*weight))
        .sum();
    let remainder = sum % 11;
    if remainder < 2 {
        0
    } else {
        (11 - remainder) as u8
    }
}

fn cnpj_base_value(character: char) -> u8 {
    character as u8 - b'0'
}

fn cnpj_digit_value(character: char) -> Option<u8> {
    character.to_digit(10).map(|value| value as u8)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EnvRefError {
    Undefined(String),
    Cyclic(String),
}

pub fn env_strip_export(key: &str) -> &str {
    let trimmed = key.trim_start();
    if let Some(rest) = trimmed.strip_prefix("export")
        && (rest.is_empty() || rest.starts_with([' ', '\t']))
    {
        return rest.trim_start();
    }
    trimmed
}

pub fn env_unquote(raw: &str) -> (String, bool) {
    let bytes = raw.as_bytes();
    if bytes.len() >= 2 {
        let (first, last) = (bytes[0], bytes[bytes.len() - 1]);
        if first == b'"' && last == b'"' {
            return (env_unescape(&raw[1..raw.len() - 1]), true);
        }
        if first == b'\'' && last == b'\'' {
            return (raw[1..raw.len() - 1].to_string(), false);
        }
    }
    (raw.to_string(), true)
}

fn env_unescape(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    let mut chars = value.chars();
    while let Some(character) = chars.next() {
        if character != '\\' {
            out.push(character);
            continue;
        }
        match chars.next() {
            Some('n') => out.push('\n'),
            Some('t') => out.push('\t'),
            Some('r') => out.push('\r'),
            Some('\\') => out.push('\\'),
            Some('"') => out.push('"'),
            Some(other) => {
                out.push('\\');
                out.push(other);
            }
            None => out.push('\\'),
        }
    }
    out
}

pub fn env_resolve(entries: &[(String, String, bool)]) -> Vec<Result<String, EnvRefError>> {
    let index: std::collections::HashMap<&str, usize> = entries
        .iter()
        .enumerate()
        .map(|(position, (name, _, _))| (name.as_str(), position))
        .collect();
    let mut cache: Vec<Option<Result<String, EnvRefError>>> = vec![None; entries.len()];
    (0..entries.len())
        .map(|position| env_resolve_one(position, entries, &index, &mut cache, &mut Vec::new()))
        .collect()
}

fn env_resolve_one(
    position: usize,
    entries: &[(String, String, bool)],
    index: &std::collections::HashMap<&str, usize>,
    cache: &mut Vec<Option<Result<String, EnvRefError>>>,
    stack: &mut Vec<usize>,
) -> Result<String, EnvRefError> {
    if let Some(cached) = &cache[position] {
        return cached.clone();
    }
    if stack.contains(&position) {
        return Err(EnvRefError::Cyclic(entries[position].0.clone()));
    }
    let (_, value, interpolatable) = &entries[position];
    if !*interpolatable || !value.contains("${") {
        let resolved = Ok(value.clone());
        cache[position] = Some(resolved.clone());
        return resolved;
    }
    stack.push(position);
    let mut out = String::new();
    let mut error = None;
    let bytes = value.as_bytes();
    let mut cursor = 0;
    while cursor < value.len() {
        if bytes[cursor] == b'$'
            && bytes.get(cursor + 1) == Some(&b'{')
            && let Some(relative) = value[cursor + 2..].find('}')
        {
            let name = value[cursor + 2..cursor + 2 + relative].to_string();
            match index.get(name.as_str()) {
                Some(&referenced) => {
                    match env_resolve_one(referenced, entries, index, cache, stack) {
                        Ok(resolved) => out.push_str(&resolved),
                        Err(reference_error) => {
                            error = Some(reference_error);
                            break;
                        }
                    }
                }
                None => {
                    error = Some(EnvRefError::Undefined(name));
                    break;
                }
            }
            cursor += 2 + relative + 1;
            continue;
        }
        let character = value[cursor..].chars().next().unwrap();
        out.push(character);
        cursor += character.len_utf8();
    }
    stack.pop();
    let resolved = match error {
        Some(reference_error) => Err(reference_error),
        None => Ok(out),
    };
    cache[position] = Some(resolved.clone());
    resolved
}

pub fn char_count(value: &str) -> usize {
    char_count_bytes(value.as_bytes())
}

pub fn char_count_bytes(bytes: &[u8]) -> usize {
    let (chunks, remainder) = bytes.as_chunks::<8>();
    let mut total = 0usize;
    for chunk in chunks {
        let word = u64::from_ne_bytes(*chunk);
        let high = word & 0x8080_8080_8080_8080;
        let next = word & 0x4040_4040_4040_4040;
        total += 8 - (high & !(next << 1)).count_ones() as usize;
    }
    total
        + remainder
            .iter()
            .filter(|&&byte| (byte as i8) >= -0x40)
            .count()
}

pub fn byte_offset_of_char(value: &str, index: usize) -> usize {
    value
        .char_indices()
        .nth(index)
        .map_or(value.len(), |(offset, _)| offset)
}

pub fn char_index_of_byte(value: &str, offset: usize) -> usize {
    value[..offset].chars().count()
}

pub fn char_byte_offset(text: &str, index: usize) -> usize {
    text.char_indices()
        .nth(index)
        .map_or(text.len(), |(offset, _)| offset)
}

pub fn normalize_char_index(index: i64, count: usize) -> usize {
    let count = count as i64;
    let resolved = if index < 0 {
        (count + index).max(0)
    } else {
        index.min(count)
    };
    resolved as usize
}

pub fn char_index_at_byte(text: &str, offset: usize) -> i64 {
    char_count(&text[..offset.min(text.len())]) as i64
}

pub fn is_not_empty(value: &str) -> bool {
    !value.is_empty()
}

pub fn is_not_blank(value: &str) -> bool {
    !value.trim().is_empty()
}

pub fn is_trimmed(value: &str) -> bool {
    value.trim() == value
}

pub fn is_lowercase(value: &str) -> bool {
    value.to_lowercase() == value
}

pub fn is_uppercase(value: &str) -> bool {
    value.to_uppercase() == value
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_string_is_measured_in_characters_and_not_bytes() {
        assert_eq!(char_count("ção"), 3);
        assert_eq!("ção".len(), 5, "the byte length is the wrong answer");
        assert_eq!(char_count(""), 0);
    }

    #[test]
    fn blankness_is_about_whitespace_and_emptiness_is_about_length() {
        assert!(is_not_empty(" "), "a space is a character");
        assert!(!is_not_blank(" "), "but it is still blank");
        assert!(!is_not_empty(""));
        assert!(!is_not_blank(""));
        assert!(is_not_blank("a"));
        assert!(!is_not_blank("\u{00a0}\t\n"));
    }

    #[test]
    fn trimming_looks_at_unicode_whitespace() {
        assert!(is_trimmed("a b"));
        assert!(!is_trimmed(" a"));
        assert!(!is_trimmed("a\u{00a0}"));
    }

    #[test]
    fn case_is_a_round_trip_and_not_a_property_of_each_character() {
        assert!(is_lowercase("abc"));
        assert!(!is_lowercase("Abc"));
        assert!(is_uppercase("ABC"));
        assert!(!is_uppercase("Abc"));

        assert!(is_lowercase("ß"));
        assert!(!is_uppercase("ß"));
        assert!(is_uppercase("SS"));

        assert!(!is_lowercase("İ"));
        assert!(is_uppercase("İ"));

        assert!(is_lowercase("123"));
        assert!(is_uppercase("123"));
    }

    #[test]
    fn validates_email_addresses() {
        assert!(is_email("hello@example.com"));
        assert!(is_email("hello.user+tag@example.co"));
        assert!(!is_email("hello@example"));
        assert!(!is_email("hello@@example.com"));
        assert!(!is_email(".hello@example.com"));
        assert!(!is_email("hello@example..com"));
    }

    #[test]
    fn validates_brazilian_postal_codes() {
        assert!(is_cep("01001000"));
        assert!(is_cep("01001-000"));
        assert!(!is_cep("0100-1000"));
        assert!(!is_cep("01001-00A"));
    }

    #[test]
    fn validates_common_string_formats() {
        assert!(is_url("https://example.com/docs"));
        assert!(is_url("http://localhost:3000"));
        assert!(!is_url("mailto:ada@example.com"));
        assert!(!is_url("https://"));

        assert!(is_uuid("01890f3d-7d2b-7cc8-98f1-97a7c8f1a111", None));
        assert!(is_uuid("01890f3d-7d2b-7cc8-98f1-97a7c8f1a111", Some(7)));
        assert!(!is_uuid("01890f3d-7d2b-7cc8-98f1-97a7c8f1a111", Some(4)));
        assert!(!is_uuid("not-a-uuid", None));

        assert!(is_ip("127.0.0.1"));
        assert!(is_ip("::1"));
        assert!(is_ipv4("127.0.0.1"));
        assert!(!is_ipv4("::1"));
        assert!(is_ipv6("::1"));
        assert!(!is_ipv6("127.0.0.1"));

        assert!(is_e164("+15551234567"));
        assert!(!is_e164("15551234567"));
        assert!(!is_e164("+1555123456789012"));
    }

    #[test]
    fn validates_brazilian_cpf_values() {
        assert!(is_cpf("529.982.247-25"));
        assert!(is_cpf("52998224725"));
        assert!(!is_cpf("529.982.247-26"));
        assert!(!is_cpf("111.111.111-11"));
    }

    #[test]
    fn validates_brazilian_cnpj_values() {
        assert!(is_cnpj("11.222.333/0001-81"));
        assert!(is_cnpj("11222333000181"));
        assert!(is_cnpj("AB.CDE.F12/3456-80"));
        assert!(is_cnpj("ABCDEF12345680"));
        assert!(!is_cnpj("AB.CDE.F12/3456-81"));
        assert!(!is_cnpj("AB.CDE.F12/3456-AA"));
    }

    #[test]
    fn env_unquote_strips_matching_quotes() {
        assert_eq!(env_unquote("plain"), ("plain".to_string(), true));
        assert_eq!(
            env_unquote("\"hello world\""),
            ("hello world".to_string(), true)
        );
        assert_eq!(
            env_unquote("'literal ${A}'"),
            ("literal ${A}".to_string(), false)
        );
        assert_eq!(env_unquote("\"a\\nb\""), ("a\nb".to_string(), true));
        assert_eq!(
            env_unquote("\"unterminated"),
            ("\"unterminated".to_string(), true)
        );
    }

    #[test]
    fn env_resolve_expands_references_and_chains() {
        let entries = vec![
            ("HOST".to_string(), "localhost".to_string(), true),
            ("PORT".to_string(), "3000".to_string(), true),
            (
                "URL".to_string(),
                "http://${HOST}:${PORT}".to_string(),
                true,
            ),
            ("ALIAS".to_string(), "${URL}/api".to_string(), true),
        ];
        let resolved = env_resolve(&entries);
        assert_eq!(resolved[2], Ok("http://localhost:3000".to_string()));
        assert_eq!(resolved[3], Ok("http://localhost:3000/api".to_string()));
    }

    #[test]
    fn env_resolve_reports_undefined_and_cycles() {
        let undefined = vec![("A".to_string(), "${MISSING}".to_string(), true)];
        assert_eq!(
            env_resolve(&undefined)[0],
            Err(EnvRefError::Undefined("MISSING".to_string()))
        );

        let cyclic = vec![
            ("A".to_string(), "${B}".to_string(), true),
            ("B".to_string(), "${A}".to_string(), true),
        ];
        let resolved = env_resolve(&cyclic);
        assert!(matches!(resolved[0], Err(EnvRefError::Cyclic(_))));
        assert!(matches!(resolved[1], Err(EnvRefError::Cyclic(_))));
    }

    #[test]
    fn env_resolve_does_not_interpolate_single_quoted() {
        let entries = vec![
            ("A".to_string(), "x".to_string(), true),
            ("B".to_string(), "${A}".to_string(), false),
        ];
        assert_eq!(env_resolve(&entries)[1], Ok("${A}".to_string()));
    }
}
