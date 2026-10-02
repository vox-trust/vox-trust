//! The trust-policy decision table (spec section 8).

use core::fmt;

/// What the verifier found in a piece of audio.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum SealCheck {
    /// A seal was found and it verifies under a key the verifier trusts.
    Valid,
    /// A seal was found but it is broken, or the audio does not match it.
    Invalid,
    /// A well-formed seal exists, but under a key the verifier does not trust.
    UnknownKey,
    /// No seal could be extracted.
    Absent,
}

/// What the verifier knows about the claimed speaker.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ContactState {
    /// The verifier has pinned this contact as someone who always seals.
    pub always_seals: bool,
    /// The verifier wants a missing seal from this contact to be an alert.
    pub strict: bool,
}

/// The outcome shown to the user.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Verdict {
    /// A valid seal from a trusted key.
    Verified,
    /// No trusted seal, from someone who never used the protocol. Neutral, never "fake".
    Unsealed,
    /// A missing seal from a contact who always seals.
    Warning,
    /// A broken seal, a seal from the wrong key for a pinned contact, or a missing seal
    /// in strict mode for a pinned contact.
    Alert,
}

impl SealCheck {
    /// Stable lower-case name, used in JSON reports and by the command-line tool.
    pub fn as_str(self) -> &'static str {
        match self {
            SealCheck::Valid => "valid",
            SealCheck::Invalid => "invalid",
            SealCheck::UnknownKey => "unknown_key",
            SealCheck::Absent => "absent",
        }
    }
}

impl fmt::Display for SealCheck {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl fmt::Display for Verdict {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl Verdict {
    /// Stable lower-case name, used in JSON output and by the command-line tool.
    pub fn as_str(self) -> &'static str {
        match self {
            Verdict::Verified => "verified",
            Verdict::Unsealed => "unsealed",
            Verdict::Warning => "warning",
            Verdict::Alert => "alert",
        }
    }

    /// A stable numeric code, used by the WebAssembly interface.
    pub fn code(self) -> u32 {
        match self {
            Verdict::Verified => 0,
            Verdict::Unsealed => 1,
            Verdict::Warning => 2,
            Verdict::Alert => 3,
        }
    }
}

/// Applies the trust-policy table of the spec (section 8).
///
/// `contact` is `None` when the verifier has no record of the claimed speaker.
pub fn decide(check: SealCheck, contact: Option<ContactState>) -> Verdict {
    let pinned = contact.is_some_and(|c| c.always_seals);
    let strict = contact.is_some_and(|c| c.always_seals && c.strict);
    match check {
        SealCheck::Valid => Verdict::Verified,
        SealCheck::Invalid => Verdict::Alert,
        SealCheck::UnknownKey if pinned => Verdict::Alert,
        SealCheck::UnknownKey => Verdict::Unsealed,
        SealCheck::Absent if strict => Verdict::Alert,
        SealCheck::Absent if pinned => Verdict::Warning,
        SealCheck::Absent => Verdict::Unsealed,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PINNED: ContactState = ContactState {
        always_seals: true,
        strict: false,
    };
    const STRICT: ContactState = ContactState {
        always_seals: true,
        strict: true,
    };
    const NEVER: ContactState = ContactState {
        always_seals: false,
        strict: false,
    };
    // `strict` without `always_seals` is meaningless: it must not change anything.
    const STRICT_ONLY: ContactState = ContactState {
        always_seals: false,
        strict: true,
    };

    #[test]
    fn valid_and_invalid_do_not_depend_on_the_contact() {
        for contact in [
            None,
            Some(NEVER),
            Some(STRICT_ONLY),
            Some(PINNED),
            Some(STRICT),
        ] {
            assert_eq!(decide(SealCheck::Valid, contact), Verdict::Verified);
            assert_eq!(decide(SealCheck::Invalid, contact), Verdict::Alert);
        }
    }

    #[test]
    fn absent_seal() {
        assert_eq!(decide(SealCheck::Absent, None), Verdict::Unsealed);
        assert_eq!(decide(SealCheck::Absent, Some(NEVER)), Verdict::Unsealed);
        assert_eq!(
            decide(SealCheck::Absent, Some(STRICT_ONLY)),
            Verdict::Unsealed
        );
        assert_eq!(decide(SealCheck::Absent, Some(PINNED)), Verdict::Warning);
        assert_eq!(decide(SealCheck::Absent, Some(STRICT)), Verdict::Alert);
    }

    #[test]
    fn seal_under_an_unknown_key() {
        assert_eq!(decide(SealCheck::UnknownKey, None), Verdict::Unsealed);
        assert_eq!(
            decide(SealCheck::UnknownKey, Some(NEVER)),
            Verdict::Unsealed
        );
        assert_eq!(decide(SealCheck::UnknownKey, Some(PINNED)), Verdict::Alert);
        assert_eq!(decide(SealCheck::UnknownKey, Some(STRICT)), Verdict::Alert);
    }

    #[test]
    fn names_are_stable_and_display_matches() {
        for (check, name) in [
            (SealCheck::Valid, "valid"),
            (SealCheck::Invalid, "invalid"),
            (SealCheck::UnknownKey, "unknown_key"),
            (SealCheck::Absent, "absent"),
        ] {
            assert_eq!(check.as_str(), name);
            assert_eq!(check.to_string(), name);
        }
        for (verdict, name) in [
            (Verdict::Verified, "verified"),
            (Verdict::Unsealed, "unsealed"),
            (Verdict::Warning, "warning"),
            (Verdict::Alert, "alert"),
        ] {
            assert_eq!(verdict.as_str(), name);
            assert_eq!(verdict.to_string(), name);
        }
    }

    #[test]
    fn verdict_codes_are_stable() {
        assert_eq!(Verdict::Verified.code(), 0);
        assert_eq!(Verdict::Unsealed.code(), 1);
        assert_eq!(Verdict::Warning.code(), 2);
        assert_eq!(Verdict::Alert.code(), 3);
    }
}
