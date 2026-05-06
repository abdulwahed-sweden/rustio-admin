//! 5-tier role hierarchy.
//!
//! Roles form a strict ladder; a higher role implicitly has every
//! capability of every lower role:
//!
//! ```text
//!   Developer (6) > Administrator (5) > Supervisor (4) > Staff (3) > User (2)
//! ```
//!
//! Two semantic dimensions every caller cares about:
//!
//! - [`Role::can_access_panel`] — gates the `/admin` URL space.
//!   Anything Staff or higher can enter; lower bounces to login or
//!   the forbidden page.
//! - [`Role::bypasses_group_checks`] — gates per-model permissions.
//!   Administrator and Developer skip the `view_X`/`add_X`/… group
//!   table lookup; Supervisor and below get checked.
//!
//! Roles are locked at 5 — by design. New project-specific tiers go
//! into the M2M groups table, not into this enum.

use crate::error::{Error, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Role {
    User,
    Staff,
    Supervisor,
    Administrator,
    Developer,
}

impl Role {
    /// Numeric rank used for `includes` comparisons. The actual values
    /// only matter relative to each other.
    pub fn rank(self) -> u8 {
        match self {
            Role::User => 2,
            Role::Staff => 3,
            Role::Supervisor => 4,
            Role::Administrator => 5,
            Role::Developer => 6,
        }
    }

    /// `self` is allowed to do anything `other` can do.
    pub fn includes(self, other: Role) -> bool {
        self.rank() >= other.rank()
    }

    /// Stable, lowercase identifier. Matches the SQL `role` column.
    pub fn as_str(self) -> &'static str {
        match self {
            Role::User => "user",
            Role::Staff => "staff",
            Role::Supervisor => "supervisor",
            Role::Administrator => "administrator",
            Role::Developer => "developer",
        }
    }

    /// Human-readable label for templates.
    pub fn label(self) -> &'static str {
        match self {
            Role::User => "User",
            Role::Staff => "Staff",
            Role::Supervisor => "Supervisor",
            Role::Administrator => "Administrator",
            Role::Developer => "Developer",
        }
    }

    /// Parse from the SQL string. Errors map to `Error::BadRequest`
    /// so HTTP handlers can surface them directly.
    pub fn parse(s: &str) -> Result<Self> {
        match s {
            "user" => Ok(Role::User),
            "staff" => Ok(Role::Staff),
            "supervisor" => Ok(Role::Supervisor),
            "administrator" => Ok(Role::Administrator),
            "developer" => Ok(Role::Developer),
            other => Err(Error::BadRequest(format!("unknown role: {other}"))),
        }
    }

    /// Is this role allowed into the admin panel at all?
    /// Staff and higher pass.
    pub fn can_access_panel(self) -> bool {
        self.rank() >= Role::Staff.rank()
    }

    /// Bypasses per-model group permission checks. Administrator and
    /// Developer can do anything the framework knows how to do; lower
    /// tiers must hold the matching `<admin>.<action>_<model>` permission.
    pub fn bypasses_group_checks(self) -> bool {
        matches!(self, Role::Administrator | Role::Developer)
    }
}

impl serde::Serialize for Role {
    fn serialize<S: serde::Serializer>(&self, s: S) -> std::result::Result<S::Ok, S::Error> {
        s.serialize_str(self.as_str())
    }
}

impl<'de> serde::Deserialize<'de> for Role {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> std::result::Result<Self, D::Error> {
        let s = <String as serde::Deserialize>::deserialize(d)?;
        Role::parse(&s).map_err(serde::de::Error::custom)
    }
}

impl std::fmt::Display for Role {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 25-case `Role::includes` matrix. Every (self, other) pair.
    #[test]
    fn includes_matrix_is_strict_ladder() {
        let tiers = [
            Role::User,
            Role::Staff,
            Role::Supervisor,
            Role::Administrator,
            Role::Developer,
        ];
        for (i, &a) in tiers.iter().enumerate() {
            for (j, &b) in tiers.iter().enumerate() {
                assert_eq!(
                    a.includes(b),
                    i >= j,
                    "{a:?}.includes({b:?}) should be {}",
                    i >= j
                );
            }
        }
    }

    #[test]
    fn parse_round_trips_for_every_variant() {
        for &r in &[
            Role::User,
            Role::Staff,
            Role::Supervisor,
            Role::Administrator,
            Role::Developer,
        ] {
            assert_eq!(Role::parse(r.as_str()).unwrap(), r);
        }
    }

    #[test]
    fn parse_rejects_unknown() {
        assert!(Role::parse("admin").is_err());
        assert!(Role::parse("root").is_err());
        assert!(Role::parse("").is_err());
    }

    #[test]
    fn can_access_panel_gates_at_staff() {
        assert!(!Role::User.can_access_panel());
        assert!(Role::Staff.can_access_panel());
        assert!(Role::Supervisor.can_access_panel());
        assert!(Role::Administrator.can_access_panel());
        assert!(Role::Developer.can_access_panel());
    }

    #[test]
    fn bypasses_group_checks_only_admin_and_dev() {
        assert!(!Role::User.bypasses_group_checks());
        assert!(!Role::Staff.bypasses_group_checks());
        assert!(!Role::Supervisor.bypasses_group_checks());
        assert!(Role::Administrator.bypasses_group_checks());
        assert!(Role::Developer.bypasses_group_checks());
    }

    #[test]
    fn label_is_capitalized_human_form() {
        assert_eq!(Role::Administrator.label(), "Administrator");
        assert_eq!(Role::Developer.label(), "Developer");
        assert_eq!(Role::User.label(), "User");
    }
}
