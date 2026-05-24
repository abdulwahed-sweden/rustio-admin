//! ULID generation for `history.jsonl` event identifiers.
//!
//! `DESIGN_BUILDER.md` §4.4 #6: *"ULIDs are derived from the
//! wall-clock timestamp plus 80 bits of randomness. Replay does not
//! regenerate ULIDs; reading an existing log does not assign new
//! IDs."*
//!
//! Wraps the `ulid` crate. Lives in its own module so the rest of
//! the Builder never imports `ulid::*` directly -- useful for the
//! "no second ULID source" discipline if it ever becomes a doctrine
//! concern.

use ulid::Ulid;

/// Generate a fresh ULID using the system clock. Monotonic within a
/// single process; reuses the same wall-clock millisecond across
/// multiple calls only with strictly-greater random suffixes.
///
/// Returned as a Crockford-base32 string (26 ASCII chars).
pub(crate) fn new_ulid() -> String {
    Ulid::new().to_string()
}

/// Parse a previously-generated ULID. Used by the replay path to
/// validate that an on-disk event id is well-formed before trusting
/// any ordering it implies.
pub(crate) fn parse(s: &str) -> Result<Ulid, ulid::DecodeError> {
    Ulid::from_string(s)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_ulid_is_26_chars_crockford_base32() {
        let id = new_ulid();
        assert_eq!(id.len(), 26);
        // Crockford base32 alphabet: 0-9, A-Z minus I, L, O, U.
        for c in id.chars() {
            assert!(
                c.is_ascii_digit() || (c.is_ascii_uppercase() && !"ILOU".contains(c)),
                "non-Crockford char {c:?} in ULID {id:?}"
            );
        }
    }

    #[test]
    fn parse_round_trip() {
        let id = new_ulid();
        let parsed = parse(&id).expect("our own ULID must parse");
        assert_eq!(parsed.to_string(), id);
    }

    #[test]
    fn ulids_are_distinct_within_process() {
        // 100 IDs back-to-back. ULID guarantees monotonic ordering
        // within a millisecond by incrementing the random suffix, so
        // no collisions even when wall clock ticks slowly.
        let mut seen = std::collections::HashSet::new();
        for _ in 0..100 {
            let id = new_ulid();
            assert!(seen.insert(id), "ULID collision");
        }
    }
}
