pub fn parse_symbols(output: &str) -> Vec<String> {
    let mut symbols = std::collections::BTreeSet::new();
    for line in output.lines() {
        let mut fields = line.split_whitespace();
        let (Some(symbol), Some(kind)) = (fields.next(), fields.next()) else {
            continue;
        };
        if kind.len() == 1
            && kind.as_bytes()[0].is_ascii_alphabetic()
            && symbol.trim_start_matches('_').starts_with("tyrus_")
        {
            symbols.insert(symbol.to_string());
        }
    }
    symbols.into_iter().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn archive_member_headers_are_not_exported_as_symbols() {
        let output = "tyrus_runtime-hash.cgu.o:\ntyrus_probe T 0 10\ntyrus_decimal-hash.cgu.o:\ntyrus_data D 10 8\n";
        assert_eq!(parse_symbols(output), ["tyrus_data", "tyrus_probe"]);
    }

    #[test]
    fn macho_prefixes_and_duplicate_definitions_are_preserved_and_deduplicated() {
        let output = "_tyrus_probe T 0 10\n_ordinary T 20 10\n_tyrus_probe T 0 10\n";
        assert_eq!(parse_symbols(output), ["_tyrus_probe"]);
    }
}
