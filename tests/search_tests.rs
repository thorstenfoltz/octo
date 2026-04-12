use octa::data::search::RowMatcher;
use octa::data::*;

// --- wildcard_to_regex ---

#[test]
fn wildcard_star_matches_any() {
    let re = regex::Regex::new(&wildcard_to_regex("foo*bar")).unwrap();
    assert!(re.is_match("fooXYZbar"));
    assert!(re.is_match("foobar"));
    assert!(!re.is_match("foXbar"));
}

#[test]
fn wildcard_question_mark_matches_single_char() {
    let re = regex::Regex::new(&wildcard_to_regex("item?")).unwrap();
    assert!(re.is_match("itemA"));
    assert!(re.is_match("item1"));
    // ? matches exactly one char inside the string
    assert!(!re.is_match("item"));
}

#[test]
fn wildcard_escaped_star_is_literal() {
    let re = regex::Regex::new(&wildcard_to_regex("2\\*3")).unwrap();
    assert!(re.is_match("2*3"));
    assert!(!re.is_match("2X3"));
    assert!(!re.is_match("23"));
}

#[test]
fn wildcard_escaped_question_is_literal() {
    let re = regex::Regex::new(&wildcard_to_regex("what\\?")).unwrap();
    assert!(re.is_match("what?"));
    assert!(!re.is_match("whatX"));
}

#[test]
fn wildcard_case_insensitive() {
    let re = regex::Regex::new(&wildcard_to_regex("hello*")).unwrap();
    assert!(re.is_match("HELLO world"));
    assert!(re.is_match("Hello"));
}

#[test]
fn wildcard_special_regex_chars_escaped() {
    let re = regex::Regex::new(&wildcard_to_regex("price ($10.00)")).unwrap();
    assert!(re.is_match("price ($10.00)"));
    assert!(!re.is_match("price X$10Y00Z"));
}

#[test]
fn wildcard_combined_star_and_question() {
    let re = regex::Regex::new(&wildcard_to_regex("a?c*z")).unwrap();
    assert!(re.is_match("abcXYZz"));
    assert!(re.is_match("axcz"));
    assert!(!re.is_match("az"));
}

// --- SearchMode ---

#[test]
fn search_mode_labels() {
    assert_eq!(SearchMode::Plain.label(), "Plain");
    assert_eq!(SearchMode::Wildcard.label(), "Wildcard");
    assert_eq!(SearchMode::Regex.label(), "Regex");
}

#[test]
fn search_mode_default_is_plain() {
    assert_eq!(SearchMode::default(), SearchMode::Plain);
}

// --- Regex replace via wildcard ---

#[test]
fn wildcard_regex_replace() {
    let re = regex::Regex::new(&wildcard_to_regex("foo*bar")).unwrap();
    let result = re.replace("fooXYZbar", "replaced");
    assert_eq!(result, "replaced");
}

#[test]
fn wildcard_regex_replace_preserves_surrounding() {
    let re = regex::Regex::new(&wildcard_to_regex("world")).unwrap();
    let result = re.replace("hello world!", "earth");
    assert_eq!(result, "hello earth!");
}

#[test]
fn regex_replace_with_capture_groups() {
    let re = regex::Regex::new(r"(\d+)\.(\d+)").unwrap();
    let result = re.replace("price 12.50", "$1,$2");
    assert_eq!(result, "price 12,50");
}

// --- Unicode / Umlaut tests ---

#[test]
fn plain_search_matches_german_umlauts() {
    let m = RowMatcher::new("über", SearchMode::Plain);
    assert!(m.matches("Übersicht"));
    assert!(m.matches("über"));
    assert!(!m.matches("uber"));
}

#[test]
fn plain_search_case_insensitive_umlauts() {
    let m = RowMatcher::new("ä", SearchMode::Plain);
    assert!(m.matches("Ä"));
    assert!(m.matches("ä"));
    assert!(m.matches("Bär"));
    assert!(m.matches("BÄR"));
}

#[test]
fn plain_search_eszett() {
    let m = RowMatcher::new("straße", SearchMode::Plain);
    assert!(m.matches("Straße"));
    assert!(m.matches("straße"));
    // Note: ß and SS are NOT equivalent via to_lowercase(); this is expected.
    assert!(!m.matches("STRASSE"));
}

#[test]
fn plain_replace_with_umlauts_in_query() {
    let m = RowMatcher::new("ä", SearchMode::Plain);
    assert_eq!(m.replace("Bär", "ae"), "Baer");
    assert_eq!(m.replace("BÄR", "aeR"), "BaeRR");
}

#[test]
fn plain_replace_preserves_surrounding_unicode() {
    let m = RowMatcher::new("world", SearchMode::Plain);
    assert_eq!(m.replace("Ünö world Ünö", "welt"), "Ünö welt Ünö");
}

#[test]
fn plain_search_mixed_ascii_umlaut() {
    let m = RowMatcher::new("münchen", SearchMode::Plain);
    assert!(m.matches("München"));
    assert!(m.matches("MÜNCHEN"));
    assert!(!m.matches("Munchen"));
}

#[test]
fn wildcard_search_with_umlauts() {
    let re = regex::Regex::new(&wildcard_to_regex("ü*ung")).unwrap();
    assert!(re.is_match("Übung"));
    assert!(re.is_match("überraschung"));
    assert!(!re.is_match("ubung"));
}

#[test]
fn wildcard_case_insensitive_umlauts() {
    let re = regex::Regex::new(&wildcard_to_regex("*ö*")).unwrap();
    assert!(re.is_match("schön"));
    assert!(re.is_match("SCHÖN"));
    assert!(re.is_match("Öl"));
}

#[test]
fn plain_replace_turkish_i_no_panic() {
    // Turkish İ (U+0130) lowercases to i + combining dot (3 bytes vs 2 bytes).
    // The old byte-offset approach would panic here; regex-based replace is safe.
    let m = RowMatcher::new("test", SearchMode::Plain);
    let result = m.replace("İ test end", "X");
    assert!(result.contains('X'));
    assert!(!result.contains("test"));
}

#[test]
fn plain_search_accented_characters() {
    let m = RowMatcher::new("café", SearchMode::Plain);
    assert!(m.matches("Café"));
    assert!(m.matches("CAFÉ"));
    assert!(!m.matches("cafe"));
}
