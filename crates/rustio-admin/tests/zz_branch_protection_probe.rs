//! THROWAWAY — deliberately failing test used to prove that branch
//! protection on `main` refuses a merge when a required check is red.
//!
//! This file must never reach `main`. It exists only on the
//! `test/protection-probe` branch and is deleted with it.

#[test]
fn this_test_fails_on_purpose() {
    assert_eq!(
        1, 2,
        "intentional failure — proving `build / test / lint` blocks the merge"
    );
}
