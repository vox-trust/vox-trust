//! Kotlin, Swift and Python bindings for `vox-trust-core`, generated with UniFFI.
//!
//! The interface mirrors the JavaScript package: [`seal_wav`], [`verify_wav`] and [`decide`],
//! plus key helpers. Byte strings are copied across the boundary; secrets are not kept.
//! There is no hand-written `unsafe` code here; the scaffolding UniFFI generates has some,
//! at the language boundary.

use vox_trust_core::file::{self, SealParams, Signer, Trust as CoreTrust};
use vox_trust_core::{circle, policy, ContactState};

uniffi::setup_scaffolding!();

/// Why a call failed.
#[derive(Debug, thiserror::Error, uniffi::Error)]
pub enum VoxTrustError {
    /// A key, seed or public key was not 32 bytes, or another argument was invalid.
    #[error("invalid argument: {reason}")]
    InvalidArgument {
        /// What was wrong.
        reason: String,
    },
    /// The WAV file could not be sealed or read (not 16-bit PCM, malformed, too large...).
    #[error("{reason}")]
    File {
        /// What was wrong.
        reason: String,
    },
}

/// How a file is sealed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum SealMode {
    /// A secret shared by a circle (family, team): HMAC-SHA-256.
    Circle,
    /// The speaker's own Ed25519 key; verifiers pin the public key.
    Public,
}

/// What the verifier found, the input to [`decide`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum SealCheck {
    /// A seal that verifies under a trusted key.
    Valid,
    /// A broken seal, or audio that no longer matches it.
    Invalid,
    /// A well-formed seal under a key the verifier does not trust.
    UnknownKey,
    /// No seal.
    Absent,
}

/// What to show the user.
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum Verdict {
    /// A valid seal from a trusted key.
    Verified,
    /// No seal you can check, from someone who never seals. Neutral, never "fake".
    Unsealed,
    /// No seal from a contact who always seals.
    Warning,
    /// A broken seal, a different key for a pinned contact, or a missing seal in strict mode.
    Alert,
}

/// What the verifier knows about the claimed speaker.
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Record)]
pub struct Contact {
    /// This contact always seals, so a missing seal is suspicious.
    pub always_seals: bool,
    /// A missing seal from this contact is an alert, not a warning.
    pub strict: bool,
}

/// Keys the verifier trusts.
#[derive(Debug, Clone, PartialEq, Eq, Default, uniffi::Record)]
pub struct Trust {
    /// The circle secret (32 bytes), if any.
    #[uniffi(default = None)]
    pub circle_key: Option<Vec<u8>>,
    /// Identifier of the circle key (see [`circle_key_id`]).
    #[uniffi(default = 0)]
    pub circle_key_id: u32,
    /// The Ed25519 public key (32 bytes) pinned for this contact, if any.
    #[uniffi(default = None)]
    pub pinned_public_key: Option<Vec<u8>>,
}

/// The outcome of verifying a file. Field meanings follow the specification, section 6.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct Report {
    /// The seal check, input to [`decide`].
    pub check: SealCheck,
    /// Why, as a stable lower-case name (`"none"`, `"modified"`, `"untrusted_key"`...).
    pub reason: String,
    /// `"circle"` or `"public"`, if a manifest was readable.
    pub mode: Option<String>,
    /// Key identifier declared by the manifest.
    pub key_id: Option<u32>,
    /// Creation time declared by the manifest (authenticated only if `authenticated`).
    pub created_unix: Option<u64>,
    /// Counter declared by the manifest (authenticated only if `authenticated`).
    pub counter: Option<u32>,
    /// Sample rate of the file.
    pub sample_rate: u32,
    /// Channel count of the file.
    pub channels: u16,
    /// Frame count of the file.
    pub n_frames: u64,
    /// Frames per chunk declared by the manifest.
    pub chunk_frames: Option<u32>,
    /// Chunk count declared by the manifest.
    pub n_chunks: Option<u32>,
    /// The authenticator verified under a key the verifier trusts.
    pub authenticated: bool,
    /// The authenticator is genuine, whether or not its key is trusted.
    pub authenticator_valid: bool,
    /// Format and every chunk match the manifest (integrity, not authorship).
    pub content_matches: bool,
    /// Indexes of the chunks that changed after sealing.
    pub modified_chunks: Vec<u32>,
    /// The public key carried by a public-mode manifest. Untrusted unless
    /// `authenticator_valid`, and attributable to a person only if `authenticated`.
    pub embedded_public_key: Option<Vec<u8>>,
}

fn key32(bytes: &[u8], name: &str) -> Result<[u8; 32], VoxTrustError> {
    bytes
        .try_into()
        .map_err(|_| VoxTrustError::InvalidArgument {
            reason: format!("{name} must be 32 bytes, got {}", bytes.len()),
        })
}

fn file_error(e: file::FileError) -> VoxTrustError {
    VoxTrustError::File {
        reason: e.to_string(),
    }
}

fn check_out(c: vox_trust_core::SealCheck) -> SealCheck {
    match c {
        vox_trust_core::SealCheck::Valid => SealCheck::Valid,
        vox_trust_core::SealCheck::Invalid => SealCheck::Invalid,
        vox_trust_core::SealCheck::UnknownKey => SealCheck::UnknownKey,
        _ => SealCheck::Absent,
    }
}

fn check_in(c: SealCheck) -> vox_trust_core::SealCheck {
    match c {
        SealCheck::Valid => vox_trust_core::SealCheck::Valid,
        SealCheck::Invalid => vox_trust_core::SealCheck::Invalid,
        SealCheck::UnknownKey => vox_trust_core::SealCheck::UnknownKey,
        SealCheck::Absent => vox_trust_core::SealCheck::Absent,
    }
}

/// Seals a 16-bit PCM WAV file and returns the sealed bytes.
///
/// `key` is the 32-byte circle secret or Ed25519 seed; `key_id` is written into circle-mode
/// manifests (ignored in public mode). `chunk_frames` is the unit of tamper localization,
/// for example the sample rate for one-second chunks.
#[uniffi::export]
pub fn seal_wav(
    wav: Vec<u8>,
    mode: SealMode,
    key: Vec<u8>,
    key_id: u32,
    created_unix: u64,
    counter: u32,
    chunk_frames: u32,
) -> Result<Vec<u8>, VoxTrustError> {
    let key = key32(&key, "key")?;
    let signer = match mode {
        SealMode::Circle => Signer::Circle { key: &key, key_id },
        SealMode::Public => Signer::Public { seed: &key },
    };
    let params = SealParams {
        created_unix,
        counter,
        chunk_frames,
    };
    file::seal_wav(&wav, signer, params).map_err(file_error)
}

/// Verifies a WAV file against the keys the verifier trusts.
#[uniffi::export]
pub fn verify_wav(wav: Vec<u8>, trust: Trust) -> Result<Report, VoxTrustError> {
    let circle = trust
        .circle_key
        .as_deref()
        .map(|k| key32(k, "circle_key"))
        .transpose()?;
    let pinned = trust
        .pinned_public_key
        .as_deref()
        .map(|k| key32(k, "pinned_public_key"))
        .transpose()?;
    let core_trust = CoreTrust {
        circle: circle.as_ref().map(|k| (trust.circle_key_id, k)),
        pinned_public: pinned.as_ref(),
    };
    let r = file::verify_wav(&wav, core_trust).map_err(file_error)?;
    Ok(Report {
        check: check_out(r.check),
        reason: r.reason.as_str().to_string(),
        mode: r.mode.map(|m| m.as_str().to_string()),
        key_id: r.key_id,
        created_unix: r.created_unix,
        counter: r.counter,
        sample_rate: r.sample_rate,
        channels: r.channels,
        n_frames: r.n_frames,
        chunk_frames: r.chunk_frames,
        n_chunks: r.n_chunks,
        authenticated: r.authenticated,
        authenticator_valid: r.authenticator_valid,
        content_matches: r.content_matches,
        modified_chunks: r.modified_chunks,
        embedded_public_key: r.embedded_public_key.map(|k| k.to_vec()),
    })
}

/// Applies the trust policy (specification, section 8). Pass no contact when the speaker
/// is not a known contact.
#[uniffi::export(default(contact = None))]
pub fn decide(check: SealCheck, contact: Option<Contact>) -> Verdict {
    let contact = contact.map(|c| ContactState {
        always_seals: c.always_seals,
        strict: c.strict,
    });
    match policy::decide(check_in(check), contact) {
        vox_trust_core::Verdict::Verified => Verdict::Verified,
        vox_trust_core::Verdict::Unsealed => Verdict::Unsealed,
        vox_trust_core::Verdict::Warning => Verdict::Warning,
        _ => Verdict::Alert,
    }
}

/// The Ed25519 public key (32 bytes) for a 32-byte seed. Share it with your contacts.
#[uniffi::export]
pub fn public_key(seed: Vec<u8>) -> Result<Vec<u8>, VoxTrustError> {
    Ok(file::public_key(&key32(&seed, "seed")?).to_vec())
}

/// The key identifier a public-mode manifest carries for this public key.
#[uniffi::export]
pub fn public_key_id(public_key: Vec<u8>) -> Result<u32, VoxTrustError> {
    Ok(file::public_key_id(&key32(&public_key, "public_key")?))
}

/// The recommended identifier of a 32-byte circle key.
#[uniffi::export]
pub fn circle_key_id(key: Vec<u8>) -> Result<u32, VoxTrustError> {
    Ok(circle::key_id(&key32(&key, "key")?))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tone() -> Vec<u8> {
        let pcm: Vec<u8> = (0..16_000i32)
            .flat_map(|i| (((i % 80) - 40) as i16 * 200).to_le_bytes())
            .collect();
        vox_trust_core::wav::encode_pcm16(1, 16_000, &pcm).unwrap()
    }

    #[test]
    fn public_mode_round_trip_and_tamper() {
        let seed = vec![9u8; 32];
        let pk = public_key(seed.clone()).unwrap();
        let sealed = seal_wav(tone(), SealMode::Public, seed, 0, 1, 0, 4_000).unwrap();
        let trust = Trust {
            pinned_public_key: Some(pk.clone()),
            ..Trust::default()
        };
        let contact = Some(Contact {
            always_seals: true,
            strict: false,
        });
        let r = verify_wav(sealed.clone(), trust.clone()).unwrap();
        assert_eq!(decide(r.check, contact), Verdict::Verified);
        assert_eq!(r.key_id, Some(public_key_id(pk.clone()).unwrap()));
        assert_eq!(r.embedded_public_key, Some(pk));
        let mut bad = sealed;
        bad[44 + 2 * 10_000] ^= 0x40;
        let r = verify_wav(bad, trust.clone()).unwrap();
        assert_eq!(decide(r.check, contact), Verdict::Alert);
        assert_eq!(r.modified_chunks, vec![2]);
        let r = verify_wav(tone(), trust).unwrap();
        assert_eq!(r.check, SealCheck::Absent);
        assert_eq!(decide(r.check, contact), Verdict::Warning);
        assert_eq!(decide(r.check, None), Verdict::Unsealed);
    }

    #[test]
    fn circle_mode_and_errors() {
        let key = vec![3u8; 32];
        let id = circle_key_id(key.clone()).unwrap();
        let sealed = seal_wav(tone(), SealMode::Circle, key.clone(), id, 1, 0, 16_000).unwrap();
        let trust = Trust {
            circle_key: Some(key),
            circle_key_id: id,
            pinned_public_key: None,
        };
        assert_eq!(verify_wav(sealed, trust).unwrap().check, SealCheck::Valid);
        assert!(matches!(
            seal_wav(tone(), SealMode::Circle, vec![1; 31], 0, 1, 0, 1),
            Err(VoxTrustError::InvalidArgument { .. })
        ));
        assert!(matches!(
            verify_wav(b"not a wav".to_vec(), Trust::default()),
            Err(VoxTrustError::File { .. })
        ));
        assert!(matches!(
            seal_wav(tone(), SealMode::Public, vec![1; 32], 0, 1, 0, 0),
            Err(VoxTrustError::File { .. })
        ));
    }
}
