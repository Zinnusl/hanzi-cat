// Additional integration tests for dataset invariants.
// These tests are native-friendly and avoid wasm/browser APIs.

use std::collections::HashSet;

#[test]
fn single_hanzi_entries_are_unique_and_valid() {
    let mut seen = HashSet::new();
    for (h, p) in hanzi_cat::SINGLE_HANZI {
        assert!(seen.insert(*h), "duplicate hanzi '{}' in SINGLE_HANZI", h);
        let s = *p;
        assert!(!s.is_empty(), "empty pinyin for hanzi '{}'", h);
        // pinyin should end with tone digit 1..5
        let last = s.chars().last().unwrap();
        assert!(('1'..='5').contains(&last), "pinyin '{}' for '{}' does not end with tone digit", s, h);
        // ensure exactly one digit present in single-hanzi pinyin
        let digit_count = s.chars().filter(|c| ('1'..='5').contains(c)).count();
        assert_eq!(digit_count, 1, "single hanzi pinyin '{}' for '{}' should contain exactly one tone digit", s, h);
        for c in s.chars() {
            assert!(c.is_ascii_lowercase() || ('1'..='5').contains(&c), "invalid char '{}' in pinyin '{}' for '{}'", c, s, h);
        }
    }
}

#[test]
fn multi_hanzi_entries_are_unique_and_valid() {
    let mut seen_hanzi = HashSet::new();
    let mut seen_pinyin = HashSet::new();
    for (h, p) in hanzi_cat::MULTI_HANZI {
        assert!(seen_hanzi.insert(*h), "duplicate hanzi '{}' in MULTI_HANZI", h);
        let s = *p;
        assert!(!s.is_empty(), "empty pinyin for '{}'", h);
        let last = s.chars().last().unwrap();
        assert!(('1'..='5').contains(&last), "pinyin '{}' for '{}' does not end with tone digit", s, h);
        // ensure at least 1 digit present
        let digit_count = s.chars().filter(|c| ('1'..='5').contains(c)).count();
        assert!(digit_count >= 1, "multi hanzi pinyin '{}' for '{}' should contain at least one tone digit", s, h);
        for c in s.chars() {
            assert!(c.is_ascii_lowercase() || ('1'..='5').contains(&c), "invalid char '{}' in pinyin '{}' for '{}'", c, s, h);
        }
        assert!(seen_pinyin.insert(s), "duplicate pinyin '{}' in MULTI_HANZI for '{}'", s, h);
    }
}

#[test]
fn pinyin_sets_do_not_duplicate_exactly() {
    use hanzi_cat::{SINGLE_HANZI, MULTI_HANZI};
    let single_p: HashSet<&str> = SINGLE_HANZI.iter().map(|(_,p)| *p).collect();
    for (_h,p) in MULTI_HANZI.iter() {
        assert!(!single_p.contains(*p), "pinyin '{}' appears in both SINGLE_HANZI and MULTI_HANZI", p);
    }
}

