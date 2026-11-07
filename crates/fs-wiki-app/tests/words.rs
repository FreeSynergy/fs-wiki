//! `say` fills the named places of a sentence in one pass.
//!
//! Pattern: Example-based tests of a pure function.

use fs_wiki_app::words::say;

#[test]
fn named_places_are_filled_and_unknown_ones_stay() {
    let said = say("{a} and {b} and {c}", &[("a", &1), ("b", &"two")]);
    assert_eq!(said, "1 and two and {c}");
}

#[test]
fn a_value_that_holds_a_place_is_not_filled_again() {
    let said = say("first {a}, then {b}", &[("a", &"{b}"), ("b", &"B")]);
    assert_eq!(said, "first {b}, then B");
    let said = say("{a}", &[("a", &"{a}{a}")]);
    assert_eq!(said, "{a}{a}");
}

#[test]
fn braces_that_are_not_places_are_left_alone() {
    assert_eq!(say("a { b } c {", &[("b", &"x")]), "a { b } c {");
    assert_eq!(say("{{a}}", &[("a", &"x")]), "{x}");
}
