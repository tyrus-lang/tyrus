//! The string operations tested as functions, without an ABI in the way.
//!
//! These used to be reachable only through `*const c_char`, which meant a test
//! had to build a `CString`, hand over a raw pointer and free what came back,
//! so the cheap cases mostly went untested and the awkward ones (an index
//! landing inside a multi-byte character, a reversed pair, a negative bound)
//! were checked end to end or not at all. The point of the split is that the
//! logic is now ordinary Rust, and the tests can say what they mean.

use super::*;

// The character count is an argument to every character-indexed operation now,
// because the caller across the ABI already has it as a field of the value and
// walking for it again is the cost the representation exists to remove. These
// shims supply it so each test still reads as the question it is asking.
fn slice(text: &str, start: i64, end: i64) -> &str {
    super::slice(text, text.chars().count(), start, end)
}

fn substring(text: &str, start: i64, end: i64) -> &str {
    super::substring(text, text.chars().count(), start, end)
}

fn index_of(text: &str, search: &str, from_index: i64) -> i64 {
    super::index_of(text, text.chars().count(), search, from_index)
}

fn last_index_of(text: &str, search: &str, from_index: i64) -> i64 {
    super::last_index_of(text, text.chars().count(), search, from_index)
}

fn char_at(text: &str, index: i64) -> String {
    super::char_at(text, text.chars().count(), index)
}

fn char_code_at(text: &str, index: i64) -> i32 {
    super::char_code_at(text, text.chars().count(), index)
}

fn pad_start(text: &str, target_length: i64, pad: &str) -> String {
    super::pad_start(text, text.chars().count(), target_length, pad)
}

fn pad_end(text: &str, target_length: i64, pad: &str) -> String {
    super::pad_end(text, text.chars().count(), target_length, pad)
}

const ACCENTED: &str = "héllo";

#[test]
fn slices_on_character_boundaries() {
    assert_eq!(slice(ACCENTED, 1, 3), "él");
    assert_eq!(slice(ACCENTED, 0, INDEX_UNSET), ACCENTED);
    assert_eq!(slice(ACCENTED, -2, INDEX_UNSET), "lo");
    assert_eq!(slice(ACCENTED, 3, 1), "");
    assert_eq!(slice(ACCENTED, 9, 12), "");
}

#[test]
fn substring_clamps_and_swaps_where_slice_does_not() {
    assert_eq!(substring(ACCENTED, 3, 1), "él");
    assert_eq!(substring(ACCENTED, -2, 2), "hé");
    assert_eq!(slice(ACCENTED, -2, 2), "");
}

#[test]
fn finds_indices_in_characters() {
    assert_eq!(index_of(ACCENTED, "llo", 0), 2);
    assert_eq!(index_of(ACCENTED, "z", 0), -1);
    assert_eq!(index_of("abcabc", "a", -1), 0);
    assert_eq!(index_of(ACCENTED, "", 2), 2);
    assert_eq!(index_of(ACCENTED, "", 99), 5);
}

#[test]
fn last_index_of_treats_a_negative_start_as_zero() {
    assert_eq!(last_index_of("abcabc", "a", INDEX_UNSET), 3);
    assert_eq!(last_index_of("abcabc", "a", -1), 0);
    assert_eq!(last_index_of("abcabc", "a", 2), 0);
    assert_eq!(last_index_of("abcabc", "z", INDEX_UNSET), -1);
}

/// Indexing counts characters, and a negative index counts from the end the way
/// `slice` does. That is not what JavaScript's `charAt` does, it returns the
/// empty string for a negative index, and the difference is recorded here
/// rather than silently preserved, since it is the language's existing
/// behaviour and separating the core from the ABI is not the place to change it.
#[test]
fn reads_characters_by_index_counting_a_negative_from_the_end() {
    assert_eq!(char_at(ACCENTED, 1), "é");
    assert_eq!(char_at(ACCENTED, -1), "o");
    assert_eq!(char_at(ACCENTED, -4), "é");
    assert_eq!(char_at(ACCENTED, 9), "");
    assert_eq!(char_at(ACCENTED, -99), "h");

    assert_eq!(char_code_at(ACCENTED, 1), 0xe9);
    assert_eq!(char_code_at(ACCENTED, -1), 'o' as i32);
    assert_eq!(char_code_at(ACCENTED, 9), -1);
}

#[test]
fn counts_occurrences_without_overlap() {
    assert_eq!(count_occurrences("abcabc", "abc"), 2);
    assert_eq!(count_occurrences("aaa", "aa"), 1);
    assert_eq!(count_occurrences("abc", ""), 0);
}

#[test]
fn replaces_only_the_first_match() {
    assert_eq!(replace_first("abcabc", "abc", "x"), "xabc");
    assert_eq!(replace_first(ACCENTED, "é", "e"), "hello");
}

#[test]
fn repeats_and_refuses_a_negative_count() {
    assert_eq!(repeat("ab", 3), "ababab");
    assert_eq!(repeat("ab", 0), "");
    assert_eq!(repeat("ab", -5), "");
    assert_eq!(repeat("", 5), "");
}

#[test]
fn pads_to_a_character_width_on_either_side() {
    assert_eq!(pad_start(ACCENTED, 8, "."), "...héllo");
    assert_eq!(pad_end(ACCENTED, 8, "."), "héllo...");
    assert_eq!(pad_start("x", 5, "ab"), "ababx");
    assert_eq!(pad_start(ACCENTED, 2, "."), ACCENTED);
    assert_eq!(pad_start(ACCENTED, 8, ""), ACCENTED);
}

#[test]
fn concatenates() {
    assert_eq!(concat(ACCENTED, " world"), "héllo world");
    assert_eq!(concat("", ""), "");
}

#[test]
fn splits_on_a_separator_and_on_nothing() {
    assert_eq!(split("a-b-c", "-"), vec!["a", "b", "c"]);
    assert_eq!(split(ACCENTED, ""), vec!["h", "é", "l", "l", "o"]);
    assert_eq!(split("abc", "-"), vec!["abc"]);
}

#[test]
fn the_ascii_fast_path_answers_what_the_walk_answers() {
    const ASCII: &str = "hello world";
    assert_eq!(slice(ASCII, 1, 3), "el");
    assert_eq!(slice(ASCII, -5, INDEX_UNSET), "world");
    assert_eq!(slice(ASCII, 3, 1), "");
    assert_eq!(substring(ASCII, 3, 1), "el");
    assert_eq!(char_at(ASCII, 4), "o");
    assert_eq!(index_of(ASCII, "world", 0), 6);
    assert_eq!(last_index_of(ASCII, "o", INDEX_UNSET), 7);

    const MIXED: &str = "héllo world";
    assert_eq!(slice(MIXED, 1, 3), "él");
    assert_eq!(slice(MIXED, -5, INDEX_UNSET), "world");
    assert_eq!(substring(MIXED, 3, 1), "él");
    assert_eq!(char_at(MIXED, 4), "o");
    assert_eq!(index_of(MIXED, "world", 0), 6);
    assert_eq!(last_index_of(MIXED, "o", INDEX_UNSET), 7);
}

#[test]
fn to_slug_case_normalizes_accents_and_separators() {
    assert_eq!(
        to_slug_case("Como criar uma API em Tyrus!"),
        "como-criar-uma-api-em-tyrus"
    );
    assert_eq!(to_slug_case("São Paulo"), "sao-paulo");
    assert_eq!(to_slug_case("hello world example"), "hello-world-example");
    assert_eq!(to_slug_case("HelloWorldExample"), "hello-world-example");
    assert_eq!(to_slug_case("hello_world_example"), "hello-world-example");
    assert_eq!(to_slug_case("hello-world-example"), "hello-world-example");
    assert_eq!(to_slug_case("  spaces  everywhere  "), "spaces-everywhere");
    assert_eq!(to_slug_case("café"), "cafe");
    assert_eq!(to_slug_case("über"), "uber");
    assert_eq!(to_slug_case(""), "");
    assert_eq!(to_slug_case("!!!"), "");
}

#[test]
fn to_dash_case_preserves_accents() {
    assert_eq!(to_dash_case("hello world example"), "hello-world-example");
    assert_eq!(to_dash_case("HelloWorldExample"), "hello-world-example");
    assert_eq!(to_dash_case("hello_world_example"), "hello-world-example");
    assert_eq!(to_dash_case("São Paulo"), "são-paulo");
    assert_eq!(to_dash_case("café"), "café");
    assert_eq!(to_dash_case(""), "");
}

#[test]
fn to_kebab_case_is_alias_of_dash_case() {
    assert_eq!(to_dash_case("Hello World"), to_dash_case("Hello World"));
    assert_eq!(to_dash_case("foo_bar"), to_dash_case("foo_bar"));
}

#[test]
fn to_pascal_case_capitalizes_each_word() {
    assert_eq!(to_pascal_case("hello world example"), "HelloWorldExample");
    assert_eq!(to_pascal_case("hello_world_example"), "HelloWorldExample");
    assert_eq!(to_pascal_case("hello-world-example"), "HelloWorldExample");
    assert_eq!(to_pascal_case("helloWorldExample"), "HelloWorldExample");
    assert_eq!(to_pascal_case("café latte"), "CaféLatte");
    assert_eq!(to_pascal_case(""), "");
}

#[test]
fn to_camel_case_lowercases_first_word() {
    assert_eq!(to_camel_case("hello world example"), "helloWorldExample");
    assert_eq!(to_camel_case("hello_world_example"), "helloWorldExample");
    assert_eq!(to_camel_case("hello-world-example"), "helloWorldExample");
    assert_eq!(to_camel_case("HelloWorldExample"), "helloWorldExample");
    assert_eq!(to_camel_case("café latte"), "caféLatte");
    assert_eq!(to_camel_case(""), "");
}

#[test]
fn to_snake_case_uses_underscores() {
    assert_eq!(to_snake_case("hello world example"), "hello_world_example");
    assert_eq!(to_snake_case("HelloWorldExample"), "hello_world_example");
    assert_eq!(to_snake_case("hello-world-example"), "hello_world_example");
    assert_eq!(to_snake_case("café"), "café");
    assert_eq!(to_snake_case(""), "");
}

#[test]
fn truncate_takes_first_n_code_points() {
    assert_eq!(
        truncate("Texto muito longo para exibição", 10),
        "Texto muit"
    );
    assert_eq!(truncate("hello", 10), "hello");
    assert_eq!(truncate("hello", 3), "hel");
    assert_eq!(truncate("hello", 0), "");
    assert_eq!(truncate("", 5), "");
    assert_eq!(truncate("héllo", 4), "héll");
}

#[test]
fn truncate_ellipsis_adds_dots_when_needed() {
    assert_eq!(
        truncate_ellipsis("Texto muito longo para exibição", 10),
        "Texto m..."
    );
    assert_eq!(truncate_ellipsis("hello", 10), "hello");
    assert_eq!(truncate_ellipsis("hello", 5), "hello");
    assert_eq!(truncate_ellipsis("hello", 4), "h...");
    assert_eq!(truncate_ellipsis("hello", 3), "hel");
    assert_eq!(truncate_ellipsis("hello", 0), "");
    assert_eq!(truncate_ellipsis("héllo", 6), "héllo");
}

#[test]
fn strip_html_removes_tags_and_decodes_entities() {
    assert_eq!(strip_html("<p>Ola <b>TitenQ</b></p>"), "Ola TitenQ");
    assert_eq!(strip_html("<div>hello</div>"), "hello");
    assert_eq!(strip_html("no tags here"), "no tags here");
    assert_eq!(strip_html("<a href=\"url\">link</a>"), "link");
    assert_eq!(
        strip_html("&lt;script&gt;alert(&#39;hack&#39;)&lt;/script&gt;"),
        "<script>alert('hack')</script>"
    );
}

#[test]
fn escape_html_escapes_special_characters() {
    assert_eq!(
        escape_html("<script>alert('hack')</script>"),
        "&lt;script&gt;alert(&#39;hack&#39;)&lt;/script&gt;"
    );
    assert_eq!(escape_html("a & b"), "a &amp; b");
    assert_eq!(escape_html("\"quoted\""), "&quot;quoted&quot;");
    assert_eq!(escape_html("no special chars"), "no special chars");
    assert_eq!(escape_html(""), "");
}

#[test]
fn escape_debug_uses_backslash_escapes() {
    assert_eq!(
        escape_debug("quoted \"text\"\\line\nnext\tcell"),
        "quoted \\\"text\\\"\\\\line\\nnext\\tcell"
    );
    assert_eq!(escape_debug("\u{1}"), "\\u{1}");
}

#[test]
fn url_encode_decodes_roundtrip() {
    let original = "TitenQ & cia";
    let encoded = url_encode(original);
    assert_eq!(encoded, "TitenQ%20%26%20cia");
    assert_eq!(url_decode(&encoded), original);
    assert_eq!(url_encode("hello world"), "hello%20world");
    assert_eq!(url_encode("abc-123_def"), "abc-123_def");
    assert_eq!(url_encode("a/b?c=d"), "a%2Fb%3Fc%3Dd");
}

#[test]
fn url_decode_handles_invalid_input() {
    assert_eq!(url_decode("%ZZ"), "%ZZ");
    assert_eq!(url_decode("hello"), "hello");
    assert_eq!(url_decode(""), "");
}
