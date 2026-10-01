//! File mode: a signed manifest inside a WAV file that shows which chunks were altered.
//!
//! The manifest commits to the audio format and to a SHA-256 digest of every chunk of
//! PCM samples. It is authenticated either with a shared secret (HMAC-SHA-256, circle mode)
//! or with an Ed25519 signature (public mode).
//!
//! **Limits, in plain words.** File mode only survives *bit-exact* copies. Re-encoding to
//! MP3 or AAC, resampling, or re-recording changes every sample, so the seal reports the
//! audio as modified. Surviving that is the job of the (not yet built) watermark carrier.
//! Only the `fmt ` parameters and the PCM samples are authenticated; other chunks are not.
//! `Valid` covers exactly: format tag, channels, sample rate, bits per sample, and the PCM
//! bytes. It does not cover `byte_rate`, extra `fmt ` bytes, other chunks or chunk order.
//!
//! Manifest layout (all integers big-endian):
//!
//! ```text
//!  0   4  magic "VOXT"
//!  4   1  version (0)
//!  5   1  mode (0 circle, 1 public)
//!  6   4  key_id (public mode: first 4 bytes of SHA-256(public key))
//! 10   8  created_unix
//! 18   4  counter
//! 22   4  sample_rate
//! 26   2  channels
//! 28   2  bits_per_sample (16)
//! 30   8  n_frames
//! 38   4  chunk_frames
//! 42   4  n_chunks
//! 46  32*n_chunks  chunk digests: SHA-256(0x01 || index (u32 BE) || PCM bytes of the chunk)
//! then the authenticator:
//!   circle: 32 bytes  HMAC-SHA-256(K, "vox-trust/0/file-circle\0" || everything above)
//!   public: 32-byte public key || 64-byte Ed25519 signature over
//!           "vox-trust/0/file-public\0" || everything above
//! ```

use core::fmt;

use crate::crypto;
use crate::policy::SealCheck;
use crate::seal::Mode;
use crate::to_hex;
use crate::wav::{self, Wav, WavError};

/// Manifest magic bytes (also the RIFF chunk id).
pub const MAGIC: [u8; 4] = *b"VOXT";
/// Manifest version produced and understood by this crate.
pub const FILE_VERSION: u8 = 0;
/// Upper bound on the number of chunks, to keep verification memory bounded.
pub const MAX_CHUNKS: u32 = 1 << 20;

const HEADER_LEN: usize = 46;
const DIGEST_LEN: usize = 32;
const CIRCLE_AUTH_LEN: usize = 32;
const PUBLIC_AUTH_LEN: usize = 32 + 64;
const DOMAIN_CIRCLE: &[u8] = b"vox-trust/0/file-circle\0";
const DOMAIN_PUBLIC: &[u8] = b"vox-trust/0/file-public\0";
const CHUNK_PREFIX: u8 = 0x01;

/// Who seals a file.
///
/// `Debug` never prints the secret.
#[derive(Clone, Copy)]
pub enum Signer<'a> {
    /// Shared-secret mode.
    Circle {
        /// 32-byte secret key.
        key: &'a [u8; 32],
        /// Identifier of that key, written into the manifest.
        key_id: u32,
    },
    /// Public-key mode.
    Public {
        /// 32-byte Ed25519 seed (the private key).
        seed: &'a [u8; 32],
    },
}

impl fmt::Debug for Signer<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Signer::Circle { key_id, .. } => f
                .debug_struct("Circle")
                .field("key", &"[REDACTED]")
                .field("key_id", key_id)
                .finish(),
            Signer::Public { .. } => f
                .debug_struct("Public")
                .field("seed", &"[REDACTED]")
                .finish(),
        }
    }
}

/// What a verifier already trusts.
///
/// `Debug` never prints the circle key.
#[derive(Clone, Copy, Default)]
pub struct Trust<'a> {
    /// A circle key and its identifier, if the verifier has one.
    pub circle: Option<(u32, &'a [u8; 32])>,
    /// The Ed25519 public key the verifier has pinned for this contact, if any.
    pub pinned_public: Option<&'a [u8; 32]>,
}

impl fmt::Debug for Trust<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Trust")
            .field("circle", &self.circle.map(|(id, _)| (id, "[REDACTED]")))
            .field("pinned_public", &self.pinned_public.map(|k| to_hex(k)))
            .finish()
    }
}

/// Parameters of a sealing operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SealParams {
    /// Creation time as Unix seconds (the caller supplies the clock).
    pub created_unix: u64,
    /// Informational counter, authenticated with the rest of the manifest.
    pub counter: u32,
    /// Number of audio frames per chunk. Must be at least 1.
    pub chunk_frames: u32,
}

/// Errors that stop sealing or verifying altogether.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum FileError {
    /// The WAV file was rejected.
    Wav(WavError),
    /// `chunk_frames` was zero.
    ZeroChunkFrames,
    /// The audio has no frames.
    EmptyAudio,
    /// More than [`MAX_CHUNKS`] chunks would be needed.
    TooManyChunks,
}

impl fmt::Display for FileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FileError::Wav(e) => write!(f, "{e}"),
            FileError::ZeroChunkFrames => f.write_str("chunk size must be at least one frame"),
            FileError::EmptyAudio => f.write_str("the audio is empty"),
            FileError::TooManyChunks => f.write_str("too many chunks; use a larger chunk size"),
        }
    }
}

impl std::error::Error for FileError {}

impl From<WavError> for FileError {
    fn from(e: WavError) -> Self {
        FileError::Wav(e)
    }
}

/// Why a check came out the way it did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Reason {
    /// Everything verified.
    None,
    /// The file carries no manifest.
    NoManifest,
    /// The manifest is malformed.
    Malformed,
    /// The manifest has a version this crate does not understand.
    UnsupportedVersion,
    /// The circle authenticator does not match.
    BadAuthenticator,
    /// The public-key signature does not verify.
    BadSignature,
    /// The seal is well-formed but under a key the verifier does not trust.
    UntrustedKey,
    /// The audio format or length differs from what was sealed.
    FormatChanged,
    /// The format matches but some chunks differ.
    Modified,
}

impl fmt::Display for Reason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl Reason {
    /// Stable lower-case name.
    pub fn as_str(self) -> &'static str {
        match self {
            Reason::None => "none",
            Reason::NoManifest => "no_manifest",
            Reason::Malformed => "malformed",
            Reason::UnsupportedVersion => "unsupported_version",
            Reason::BadAuthenticator => "bad_authenticator",
            Reason::BadSignature => "bad_signature",
            Reason::UntrustedKey => "untrusted_key",
            Reason::FormatChanged => "format_changed",
            Reason::Modified => "modified",
        }
    }
}

/// The outcome of verifying a file.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Report {
    /// The seal check, input to the trust policy.
    pub check: SealCheck,
    /// Why.
    pub reason: Reason,
    /// Mode declared by the manifest, if one was readable.
    pub mode: Option<Mode>,
    /// Key identifier declared by the manifest.
    pub key_id: Option<u32>,
    /// Creation time declared by the manifest (authenticated only if `authenticated`).
    pub created_unix: Option<u64>,
    /// Counter declared by the manifest (authenticated only if `authenticated`).
    pub counter: Option<u32>,
    /// Sample rate of the file being verified.
    pub sample_rate: u32,
    /// Channel count of the file being verified.
    pub channels: u16,
    /// Frame count of the file being verified.
    pub n_frames: u64,
    /// Chunk size declared by the manifest.
    pub chunk_frames: Option<u32>,
    /// Chunk count declared by the manifest.
    pub n_chunks: Option<u32>,
    /// The manifest's authenticator verified under a trusted key.
    pub authenticated: bool,
    /// The manifest's authenticator is genuine (HMAC under the matching key, or a valid
    /// Ed25519 signature under the embedded key), whether or not that key is trusted.
    ///
    /// `content_matches` and `modified_chunks` are only meaningful when this is true.
    pub authenticator_valid: bool,
    /// The audio format and every chunk digest match the manifest. Always `false` unless
    /// `authenticator_valid`. It says nothing about *who* sealed the file: an
    /// `UnknownKey` result with `content_matches` means "intact since sealed by an unpinned
    /// key", not "genuine".
    pub content_matches: bool,
    /// Indices of chunks whose audio no longer matches the manifest. Filled only when
    /// `authenticator_valid` and the format matches; empty otherwise.
    pub modified_chunks: Vec<u32>,
    /// For public mode: the public key embedded in the manifest.
    ///
    /// **Untrusted unless `authenticator_valid`**: before that it is just bytes read from the
    /// file, and anyone can embed any key. Even when `authenticator_valid` is true it only
    /// proves the file was signed by *whoever holds that key*; attribute the file to a person
    /// only if `authenticated` is true (the key is the one the verifier pinned).
    pub embedded_public_key: Option<[u8; 32]>,
}

impl Report {
    fn new(wav: &Wav<'_>) -> Self {
        Report {
            check: SealCheck::Absent,
            reason: Reason::NoManifest,
            mode: None,
            key_id: None,
            created_unix: None,
            counter: None,
            sample_rate: wav.sample_rate,
            channels: wav.channels,
            n_frames: wav.frames(),
            chunk_frames: None,
            n_chunks: None,
            authenticated: false,
            authenticator_valid: false,
            content_matches: false,
            modified_chunks: Vec::new(),
            embedded_public_key: None,
        }
    }

    /// A compact JSON rendering. All strings come from fixed vocabularies, so no escaping
    /// is needed.
    pub fn to_json(&self) -> String {
        fn opt<T: fmt::Display>(v: Option<T>) -> String {
            v.map_or_else(|| "null".to_string(), |v| v.to_string())
        }
        fn opt_str(v: Option<String>) -> String {
            v.map_or_else(|| "null".to_string(), |v| format!("\"{v}\""))
        }
        let modified = self
            .modified_chunks
            .iter()
            .map(u32::to_string)
            .collect::<Vec<_>>()
            .join(",");
        format!(
            concat!(
                "{{\"check\":\"{}\",\"reason\":\"{}\",\"mode\":{},\"key_id\":{},",
                "\"created_unix\":{},\"counter\":{},\"sample_rate\":{},\"channels\":{},",
                "\"n_frames\":{},\"chunk_frames\":{},\"n_chunks\":{},\"authenticated\":{},",
                "\"authenticator_valid\":{},\"content_matches\":{},",
                "\"modified_chunks\":[{}],\"embedded_public_key\":{}}}"
            ),
            self.check,
            self.reason,
            opt_str(self.mode.map(|m| m.to_string())),
            opt_str(self.key_id.map(|k| to_hex(&k.to_be_bytes()))),
            opt(self.created_unix),
            opt(self.counter),
            self.sample_rate,
            self.channels,
            self.n_frames,
            opt(self.chunk_frames),
            opt(self.n_chunks),
            self.authenticated,
            self.authenticator_valid,
            self.content_matches,
            modified,
            opt_str(self.embedded_public_key.map(|k| to_hex(&k))),
        )
    }
}

/// The key identifier of an Ed25519 public key: the first 4 bytes of its SHA-256.
pub fn public_key_id(public: &[u8; 32]) -> u32 {
    let digest = crypto::sha256(&[public]);
    u32::from_be_bytes([digest[0], digest[1], digest[2], digest[3]])
}

/// The Ed25519 public key for a seed.
pub fn public_key(seed: &[u8; 32]) -> [u8; 32] {
    crypto::ed25519_public(seed)
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Header {
    mode: Mode,
    key_id: u32,
    created_unix: u64,
    counter: u32,
    sample_rate: u32,
    channels: u16,
    bits: u16,
    n_frames: u64,
    chunk_frames: u32,
    n_chunks: u32,
}

impl Header {
    fn encode(&self) -> [u8; HEADER_LEN] {
        let mut out = [0u8; HEADER_LEN];
        out[0..4].copy_from_slice(&MAGIC);
        out[4] = FILE_VERSION;
        out[5] = self.mode.bits();
        out[6..10].copy_from_slice(&self.key_id.to_be_bytes());
        out[10..18].copy_from_slice(&self.created_unix.to_be_bytes());
        out[18..22].copy_from_slice(&self.counter.to_be_bytes());
        out[22..26].copy_from_slice(&self.sample_rate.to_be_bytes());
        out[26..28].copy_from_slice(&self.channels.to_be_bytes());
        out[28..30].copy_from_slice(&self.bits.to_be_bytes());
        out[30..38].copy_from_slice(&self.n_frames.to_be_bytes());
        out[38..42].copy_from_slice(&self.chunk_frames.to_be_bytes());
        out[42..46].copy_from_slice(&self.n_chunks.to_be_bytes());
        out
    }

    fn decode(b: &[u8]) -> Result<Header, Reason> {
        if b.len() < HEADER_LEN || b[0..4] != MAGIC {
            return Err(Reason::Malformed);
        }
        if b[4] != FILE_VERSION {
            return Err(Reason::UnsupportedVersion);
        }
        let mode = Mode::from_bits(b[5]).map_err(|_| Reason::Malformed)?;
        let u32_at = |i: usize| u32::from_be_bytes([b[i], b[i + 1], b[i + 2], b[i + 3]]);
        let mut u64_bytes = [0u8; 8];
        u64_bytes.copy_from_slice(&b[10..18]);
        let created_unix = u64::from_be_bytes(u64_bytes);
        u64_bytes.copy_from_slice(&b[30..38]);
        let n_frames = u64::from_be_bytes(u64_bytes);
        let header = Header {
            mode,
            key_id: u32_at(6),
            created_unix,
            counter: u32_at(18),
            sample_rate: u32_at(22),
            channels: u16::from_be_bytes([b[26], b[27]]),
            bits: u16::from_be_bytes([b[28], b[29]]),
            n_frames,
            chunk_frames: u32_at(38),
            n_chunks: u32_at(42),
        };
        let expected_chunks = if header.chunk_frames == 0 {
            return Err(Reason::Malformed);
        } else {
            header.n_frames.div_ceil(u64::from(header.chunk_frames))
        };
        if header.n_chunks > MAX_CHUNKS
            || u64::from(header.n_chunks) != expected_chunks
            || header.n_chunks == 0
        {
            return Err(Reason::Malformed);
        }
        Ok(header)
    }
}

enum Auth {
    Circle([u8; 32]),
    Public { key: [u8; 32], signature: [u8; 64] },
}

struct Manifest {
    header: Header,
    digests: Vec<[u8; 32]>,
    auth: Auth,
    /// Length of the authenticated prefix (header + digests).
    signed_len: usize,
}

impl Manifest {
    fn decode(bytes: &[u8]) -> Result<Manifest, Reason> {
        let header = Header::decode(bytes)?;
        let digests_len = header.n_chunks as usize * DIGEST_LEN;
        let signed_len = HEADER_LEN + digests_len;
        let auth_len = match header.mode {
            Mode::Circle => CIRCLE_AUTH_LEN,
            Mode::Public => PUBLIC_AUTH_LEN,
        };
        if bytes.len() != signed_len + auth_len {
            return Err(Reason::Malformed);
        }
        let digests = bytes[HEADER_LEN..signed_len]
            .as_chunks::<DIGEST_LEN>()
            .0
            .to_vec();
        let tail = &bytes[signed_len..];
        let auth = match header.mode {
            Mode::Circle => {
                let mut tag = [0u8; 32];
                tag.copy_from_slice(tail);
                Auth::Circle(tag)
            }
            Mode::Public => {
                let mut key = [0u8; 32];
                let mut signature = [0u8; 64];
                key.copy_from_slice(&tail[..32]);
                signature.copy_from_slice(&tail[32..]);
                if header.key_id != public_key_id(&key) {
                    return Err(Reason::Malformed);
                }
                Auth::Public { key, signature }
            }
        };
        Ok(Manifest {
            header,
            digests,
            auth,
            signed_len,
        })
    }
}

fn chunk_digest(index: u32, pcm: &[u8]) -> [u8; 32] {
    crypto::sha256(&[&[CHUNK_PREFIX], &index.to_be_bytes(), pcm])
}

fn digests(pcm: &[u8], channels: u16, chunk_frames: u32) -> Vec<[u8; 32]> {
    let chunk_bytes =
        usize::try_from(u64::from(chunk_frames) * u64::from(channels) * 2).unwrap_or(usize::MAX);
    pcm.chunks(chunk_bytes.max(1))
        .enumerate()
        .map(|(i, c)| chunk_digest(i as u32, c))
        .collect()
}

/// Seals a WAV file: appends (or replaces) the `VOXT` manifest chunk.
pub fn seal_wav(
    wav_bytes: &[u8],
    signer: Signer<'_>,
    params: SealParams,
) -> Result<Vec<u8>, FileError> {
    if params.chunk_frames == 0 {
        return Err(FileError::ZeroChunkFrames);
    }
    let wav = wav::parse(wav_bytes)?;
    let n_frames = wav.frames();
    if n_frames == 0 {
        return Err(FileError::EmptyAudio);
    }
    let n_chunks = n_frames.div_ceil(u64::from(params.chunk_frames));
    if n_chunks > u64::from(MAX_CHUNKS) {
        return Err(FileError::TooManyChunks);
    }
    let (mode, key_id) = match signer {
        Signer::Circle { key_id, .. } => (Mode::Circle, key_id),
        Signer::Public { seed } => (Mode::Public, public_key_id(&crypto::ed25519_public(seed))),
    };
    let header = Header {
        mode,
        key_id,
        created_unix: params.created_unix,
        counter: params.counter,
        sample_rate: wav.sample_rate,
        channels: wav.channels,
        bits: wav.bits_per_sample,
        n_frames,
        chunk_frames: params.chunk_frames,
        n_chunks: n_chunks as u32,
    };
    let mut signed = Vec::with_capacity(HEADER_LEN + DIGEST_LEN * n_chunks as usize);
    signed.extend_from_slice(&header.encode());
    for digest in digests(wav.pcm, wav.channels, params.chunk_frames) {
        signed.extend_from_slice(&digest);
    }
    let mut manifest = signed.clone();
    match signer {
        Signer::Circle { key, .. } => {
            manifest.extend_from_slice(&crypto::hmac_sha256(key, &[DOMAIN_CIRCLE, &signed]));
        }
        Signer::Public { seed } => {
            let signature = crypto::ed25519_sign(seed, &[DOMAIN_PUBLIC, &signed].concat());
            manifest.extend_from_slice(&crypto::ed25519_public(seed));
            manifest.extend_from_slice(&signature);
        }
    }
    Ok(wav::with_manifest(wav_bytes, &manifest)?)
}

/// Verifies a WAV file against what the verifier trusts.
///
/// Errors only when the WAV file itself cannot be read. Everything about the seal is in
/// the [`Report`].
pub fn verify_wav(wav_bytes: &[u8], trust: Trust<'_>) -> Result<Report, FileError> {
    let wav = wav::parse(wav_bytes)?;
    let mut report = Report::new(&wav);
    let Some(manifest_bytes) = wav.manifest else {
        return Ok(report);
    };
    report.check = SealCheck::Invalid;
    let manifest = match Manifest::decode(manifest_bytes) {
        Ok(m) => m,
        Err(reason) => {
            report.reason = reason;
            return Ok(report);
        }
    };
    let h = &manifest.header;
    report.mode = Some(h.mode);
    report.key_id = Some(h.key_id);
    report.created_unix = Some(h.created_unix);
    report.counter = Some(h.counter);
    report.chunk_frames = Some(h.chunk_frames);
    report.n_chunks = Some(h.n_chunks);

    let signed = &manifest_bytes[..manifest.signed_len];
    // `trusted` is false only for a valid public-key signature under an unpinned key: the
    // authenticator is genuine, but the verifier has no reason to believe its holder.
    let mut trusted = true;
    match &manifest.auth {
        Auth::Circle(tag) => match trust.circle {
            Some((id, key)) if id == h.key_id => {
                if !crypto::hmac_verify(key, &[DOMAIN_CIRCLE, signed], tag) {
                    report.reason = Reason::BadAuthenticator;
                    return Ok(report);
                }
            }
            _ => {
                // Without the key nothing in the manifest can be checked: it could be forged.
                report.check = SealCheck::UnknownKey;
                report.reason = Reason::UntrustedKey;
                return Ok(report);
            }
        },
        Auth::Public { key, signature } => {
            report.embedded_public_key = Some(*key);
            if !crypto::ed25519_verify(key, &[DOMAIN_PUBLIC, signed].concat(), signature) {
                report.reason = Reason::BadSignature;
                return Ok(report);
            }
            if trust.pinned_public != Some(key) {
                trusted = false;
            }
        }
    }
    report.authenticator_valid = true;
    report.authenticated = trusted;

    // The authenticator is genuine, so the declared format and digests are too; compare
    // them with the file even when the key is not trusted.
    let format_ok = h.sample_rate == wav.sample_rate
        && h.channels == wav.channels
        && h.bits == wav.bits_per_sample
        && h.n_frames == wav.frames();
    let mut digests_ok = false;
    if format_ok {
        let actual = digests(wav.pcm, wav.channels, h.chunk_frames);
        report.modified_chunks = actual
            .iter()
            .zip(&manifest.digests)
            .enumerate()
            .filter(|(_, (a, b))| a != b)
            .map(|(i, _)| i as u32)
            .collect();
        digests_ok = report.modified_chunks.is_empty() && actual.len() == manifest.digests.len();
    }
    report.content_matches = format_ok && digests_ok;

    if !trusted {
        report.check = SealCheck::UnknownKey;
        report.reason = Reason::UntrustedKey;
    } else if !format_ok {
        report.reason = Reason::FormatChanged;
    } else if digests_ok {
        report.check = SealCheck::Valid;
        report.reason = Reason::None;
    } else {
        report.reason = Reason::Modified;
    }
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::policy::{decide, ContactState, Verdict};

    const KEY: [u8; 32] = [9u8; 32];
    const OTHER_KEY: [u8; 32] = [10u8; 32];
    const SEED: [u8; 32] = [3u8; 32];
    const ATTACKER_SEED: [u8; 32] = [4u8; 32];

    fn pcm(frames: usize, channels: usize) -> Vec<u8> {
        (0..frames * channels)
            .flat_map(|i| ((i as i16).wrapping_mul(731) ^ 0x1234).to_le_bytes())
            .collect()
    }

    fn sample(frames: usize, channels: u16) -> Vec<u8> {
        wav::encode_pcm16(channels, 8000, &pcm(frames, usize::from(channels))).unwrap()
    }

    fn params(chunk_frames: u32) -> SealParams {
        SealParams {
            created_unix: 1_700_000_000,
            counter: 7,
            chunk_frames,
        }
    }

    fn circle_signer() -> Signer<'static> {
        Signer::Circle {
            key: &KEY,
            key_id: 0x0102_0304,
        }
    }

    fn circle_trust() -> Trust<'static> {
        Trust {
            circle: Some((0x0102_0304, &KEY)),
            pinned_public: None,
        }
    }

    /// Byte offset of the PCM data inside a WAV file.
    fn pcm_offset(bytes: &[u8]) -> usize {
        let wav = wav::parse(bytes).unwrap();
        wav.pcm.as_ptr() as usize - bytes.as_ptr() as usize
    }

    fn verify(bytes: &[u8], trust: Trust<'_>) -> Report {
        verify_wav(bytes, trust).unwrap()
    }

    #[test]
    fn circle_seal_round_trip() {
        let sealed = seal_wav(&sample(100, 1), circle_signer(), params(32)).unwrap();
        let r = verify(&sealed, circle_trust());
        assert_eq!(r.check, SealCheck::Valid);
        assert_eq!(r.reason, Reason::None);
        assert!(r.authenticated && r.modified_chunks.is_empty());
        assert_eq!(r.mode, Some(Mode::Circle));
        assert_eq!(r.key_id, Some(0x0102_0304));
        assert_eq!(r.created_unix, Some(1_700_000_000));
        assert_eq!(r.counter, Some(7));
        assert_eq!(r.n_chunks, Some(4)); // 100 frames / 32 = 3 full + 1 partial
    }

    #[test]
    fn public_seal_round_trip_needs_a_pinned_key() {
        let public = public_key(&SEED);
        let sealed = seal_wav(&sample(64, 2), Signer::Public { seed: &SEED }, params(16)).unwrap();

        let pinned = Trust {
            circle: None,
            pinned_public: Some(&public),
        };
        assert_eq!(verify(&sealed, pinned).check, SealCheck::Valid);

        // Valid signature, but the verifier has not pinned any key: not trusted.
        let r = verify(&sealed, Trust::default());
        assert_eq!(r.check, SealCheck::UnknownKey);
        assert_eq!(r.reason, Reason::UntrustedKey);
        assert_eq!(r.embedded_public_key, Some(public));
        assert!(!r.authenticated);
        assert!(r.authenticator_valid && r.content_matches);
    }

    #[test]
    fn unpinned_key_still_reports_content_integrity() {
        let sealed = seal_wav(&sample(100, 1), Signer::Public { seed: &SEED }, params(25)).unwrap();
        let none = Trust::default();

        let r = verify(&sealed, none);
        assert_eq!(r.check, SealCheck::UnknownKey);
        assert!(r.content_matches && r.modified_chunks.is_empty());

        // Tampered audio under a genuine signature: still UnknownKey, but flagged.
        let mut tampered = sealed.clone();
        let at = pcm_offset(&tampered);
        tampered[at + 2 * 25 * 2 + 4] ^= 0x01;
        let r = verify(&tampered, none);
        assert_eq!(
            (r.check, r.reason),
            (SealCheck::UnknownKey, Reason::UntrustedKey)
        );
        assert!(r.authenticator_valid && !r.authenticated && !r.content_matches);
        assert_eq!(r.modified_chunks, vec![2]);

        // Changed format: not matching, and no chunk list.
        let w = wav::parse(&sealed).unwrap();
        let faster = wav::encode_pcm16(1, 16000, w.pcm).unwrap();
        let faster = wav::with_manifest(&faster, w.manifest.unwrap()).unwrap();
        let r = verify(&faster, none);
        assert_eq!(r.check, SealCheck::UnknownKey);
        assert!(!r.content_matches && r.modified_chunks.is_empty());

        // Forged signature: nothing is reported as fact.
        let mut manifest = w.manifest.unwrap().to_vec();
        let last = manifest.len() - 1;
        manifest[last] ^= 1;
        let forged = wav::with_manifest(&tampered, &manifest).unwrap();
        let r = verify(&forged, none);
        assert_eq!(r.check, SealCheck::Invalid);
        assert!(!r.authenticator_valid && !r.content_matches && r.modified_chunks.is_empty());
    }

    #[test]
    fn circle_seal_without_the_key_reports_nothing_about_content() {
        let sealed = seal_wav(&sample(100, 1), circle_signer(), params(25)).unwrap();
        let mut tampered = sealed;
        let at = pcm_offset(&tampered);
        tampered[at + 4] ^= 1;
        let r = verify(&tampered, Trust::default());
        assert_eq!(r.check, SealCheck::UnknownKey);
        assert!(!r.authenticator_valid && !r.content_matches && r.modified_chunks.is_empty());
    }

    #[test]
    fn an_attacker_resealing_with_their_own_key_is_not_trusted() {
        let public = public_key(&SEED);
        let original = sample(64, 1);
        let mut forged = original.clone();
        let at = pcm_offset(&forged);
        forged[at + 10] ^= 0xFF;
        let resealed = seal_wav(
            &forged,
            Signer::Public {
                seed: &ATTACKER_SEED,
            },
            params(16),
        )
        .unwrap();
        let victim = Trust {
            circle: None,
            pinned_public: Some(&public),
        };
        let r = verify(&resealed, victim);
        assert_eq!(r.check, SealCheck::UnknownKey);
        let pinned_contact = Some(ContactState {
            always_seals: true,
            strict: false,
        });
        assert_eq!(decide(r.check, pinned_contact), Verdict::Alert);
    }

    #[test]
    fn modified_chunk_is_localised() {
        let sealed = seal_wav(&sample(100, 1), circle_signer(), params(25)).unwrap();
        let mut tampered = sealed.clone();
        let at = pcm_offset(&tampered);
        tampered[at + 2 * 25 * 2 + 4] ^= 0x01; // one bit in chunk 2
        let r = verify(&tampered, circle_trust());
        assert_eq!(r.check, SealCheck::Invalid);
        assert_eq!(r.reason, Reason::Modified);
        assert!(r.authenticated);
        assert_eq!(r.modified_chunks, vec![2]);
    }

    #[test]
    fn replacing_several_chunks_lists_them_all() {
        let sealed = seal_wav(&sample(100, 1), circle_signer(), params(25)).unwrap();
        let mut tampered = sealed.clone();
        let at = pcm_offset(&tampered);
        for b in &mut tampered[at..at + 25 * 2] {
            *b = 0;
        }
        for b in &mut tampered[at + 75 * 2..at + 100 * 2] {
            *b = 0;
        }
        let r = verify(&tampered, circle_trust());
        assert_eq!(r.modified_chunks, vec![0, 3]);
    }

    #[test]
    fn swapping_two_chunks_is_detected_because_the_index_is_bound() {
        let sealed = seal_wav(&sample(100, 1), circle_signer(), params(25)).unwrap();
        let mut swapped = sealed.clone();
        let at = pcm_offset(&swapped);
        let (a, b) = (at, at + 25 * 2);
        for i in 0..25 * 2 {
            swapped.swap(a + i, b + i);
        }
        let r = verify(&swapped, circle_trust());
        assert_eq!(r.check, SealCheck::Invalid);
        assert_eq!(r.modified_chunks, vec![0, 1]);
    }

    #[test]
    fn truncating_or_extending_audio_is_a_format_change() {
        let sealed = seal_wav(&sample(100, 1), circle_signer(), params(25)).unwrap();
        let wav = wav::parse(&sealed).unwrap();
        // Rebuild with fewer frames but the same manifest.
        let shorter = wav::encode_pcm16(1, 8000, &wav.pcm[..wav.pcm.len() - 50]).unwrap();
        let shorter = wav::with_manifest(&shorter, wav.manifest.unwrap()).unwrap();
        let r = verify(&shorter, circle_trust());
        assert_eq!(r.check, SealCheck::Invalid);
        assert_eq!(r.reason, Reason::FormatChanged);
        assert!(r.authenticated);
    }

    #[test]
    fn changing_the_sample_rate_is_a_format_change() {
        let sealed = seal_wav(&sample(100, 1), circle_signer(), params(25)).unwrap();
        let wav = wav::parse(&sealed).unwrap();
        let faster = wav::encode_pcm16(1, 16000, wav.pcm).unwrap();
        let faster = wav::with_manifest(&faster, wav.manifest.unwrap()).unwrap();
        assert_eq!(
            verify(&faster, circle_trust()).reason,
            Reason::FormatChanged
        );
    }

    #[test]
    fn stripping_the_manifest_gives_absent() {
        let sealed = seal_wav(&sample(100, 1), circle_signer(), params(25)).unwrap();
        let wav = wav::parse(&sealed).unwrap();
        let stripped = wav::encode_pcm16(1, 8000, wav.pcm).unwrap();
        let r = verify(&stripped, circle_trust());
        assert_eq!(r.check, SealCheck::Absent);
        assert_eq!(r.reason, Reason::NoManifest);
        let pinned_contact = Some(ContactState {
            always_seals: true,
            strict: false,
        });
        assert_eq!(decide(r.check, pinned_contact), Verdict::Warning);
    }

    #[test]
    fn wrong_key_or_unknown_key_id() {
        let sealed = seal_wav(&sample(100, 1), circle_signer(), params(25)).unwrap();
        let wrong = Trust {
            circle: Some((0x0102_0304, &OTHER_KEY)),
            pinned_public: None,
        };
        let r = verify(&sealed, wrong);
        assert_eq!(
            (r.check, r.reason),
            (SealCheck::Invalid, Reason::BadAuthenticator)
        );

        let other_id = Trust {
            circle: Some((0x0A0B_0C0D, &KEY)),
            pinned_public: None,
        };
        let r = verify(&sealed, other_id);
        assert_eq!(
            (r.check, r.reason),
            (SealCheck::UnknownKey, Reason::UntrustedKey)
        );
        assert_eq!(
            verify(&sealed, Trust::default()).check,
            SealCheck::UnknownKey
        );
    }

    #[test]
    fn tampering_with_the_manifest_breaks_the_authenticator() {
        let sealed = seal_wav(&sample(100, 1), circle_signer(), params(25)).unwrap();
        let wav = wav::parse(&sealed).unwrap();
        let manifest = wav.manifest.unwrap().to_vec();
        // Flip a bit in: counter, a digest, and the authenticator itself.
        for position in [20usize, HEADER_LEN + 5, manifest.len() - 1] {
            let mut bad = manifest.clone();
            bad[position] ^= 0x01;
            let tampered = wav::with_manifest(&sealed, &bad).unwrap();
            let r = verify(&tampered, circle_trust());
            assert_eq!(
                (r.check, r.reason),
                (SealCheck::Invalid, Reason::BadAuthenticator),
                "position {position}"
            );
            assert!(!r.authenticated);
        }
    }

    #[test]
    fn tampering_with_a_public_signature_is_detected() {
        let public = public_key(&SEED);
        let sealed = seal_wav(&sample(64, 1), Signer::Public { seed: &SEED }, params(16)).unwrap();
        let wav = wav::parse(&sealed).unwrap();
        let mut manifest = wav.manifest.unwrap().to_vec();
        let last = manifest.len() - 1;
        manifest[last] ^= 1;
        let tampered = wav::with_manifest(&sealed, &manifest).unwrap();
        let trust = Trust {
            circle: None,
            pinned_public: Some(&public),
        };
        let r = verify(&tampered, trust);
        assert_eq!(
            (r.check, r.reason),
            (SealCheck::Invalid, Reason::BadSignature)
        );
    }

    #[test]
    fn a_public_manifest_whose_key_id_does_not_match_its_key_is_malformed() {
        let sealed = seal_wav(&sample(64, 1), Signer::Public { seed: &SEED }, params(16)).unwrap();
        let wav = wav::parse(&sealed).unwrap();
        let mut manifest = wav.manifest.unwrap().to_vec();
        manifest[6] ^= 0xFF; // key_id
        let tampered = wav::with_manifest(&sealed, &manifest).unwrap();
        let r = verify(&tampered, Trust::default());
        assert_eq!((r.check, r.reason), (SealCheck::Invalid, Reason::Malformed));
    }

    #[test]
    fn malformed_manifests_are_invalid_not_panics() {
        let base = sample(50, 1);
        for junk in [
            &b""[..],
            &b"VOXT"[..],
            &[0u8; HEADER_LEN][..],
            &[0xFFu8; 200][..],
        ] {
            let with_junk = wav::with_manifest(&base, junk).unwrap();
            let r = verify(&with_junk, circle_trust());
            assert_eq!(r.check, SealCheck::Invalid);
            assert_eq!(r.reason, Reason::Malformed);
        }
        // Unsupported version
        let sealed = seal_wav(&base, circle_signer(), params(25)).unwrap();
        let mut manifest = wav::parse(&sealed).unwrap().manifest.unwrap().to_vec();
        manifest[4] = 9;
        let bumped = wav::with_manifest(&base, &manifest).unwrap();
        assert_eq!(
            verify(&bumped, circle_trust()).reason,
            Reason::UnsupportedVersion
        );
    }

    #[test]
    fn a_huge_chunk_count_in_a_manifest_does_not_allocate() {
        let base = sample(50, 1);
        let sealed = seal_wav(&base, circle_signer(), params(25)).unwrap();
        let mut manifest = wav::parse(&sealed).unwrap().manifest.unwrap().to_vec();
        manifest[42..46].copy_from_slice(&u32::MAX.to_be_bytes());
        let bad = wav::with_manifest(&base, &manifest).unwrap();
        assert_eq!(verify(&bad, circle_trust()).reason, Reason::Malformed);
    }

    #[test]
    fn sealing_again_replaces_the_previous_seal() {
        let once = seal_wav(&sample(100, 1), circle_signer(), params(25)).unwrap();
        let twice = seal_wav(
            &once,
            Signer::Circle {
                key: &OTHER_KEY,
                key_id: 5,
            },
            params(50),
        )
        .unwrap();
        let r = verify(
            &twice,
            Trust {
                circle: Some((5, &OTHER_KEY)),
                pinned_public: None,
            },
        );
        assert_eq!(r.check, SealCheck::Valid);
        assert_eq!(r.n_chunks, Some(2));
        // The first key no longer matches this file.
        assert_eq!(verify(&twice, circle_trust()).check, SealCheck::UnknownKey);
    }

    #[test]
    fn chunk_sizes_edge_cases() {
        // Chunk larger than the audio: a single chunk.
        let r = verify(
            &seal_wav(&sample(10, 1), circle_signer(), params(1_000_000)).unwrap(),
            circle_trust(),
        );
        assert_eq!((r.check, r.n_chunks), (SealCheck::Valid, Some(1)));
        // One frame per chunk.
        let r = verify(
            &seal_wav(&sample(10, 2), circle_signer(), params(1)).unwrap(),
            circle_trust(),
        );
        assert_eq!((r.check, r.n_chunks), (SealCheck::Valid, Some(10)));
    }

    #[test]
    fn sealing_errors() {
        assert_eq!(
            seal_wav(&sample(10, 1), circle_signer(), params(0)).unwrap_err(),
            FileError::ZeroChunkFrames
        );
        assert_eq!(
            seal_wav(
                &wav::encode_pcm16(1, 8000, &[]).unwrap(),
                circle_signer(),
                params(4)
            )
            .unwrap_err(),
            FileError::EmptyAudio
        );
        assert_eq!(
            seal_wav(b"not a wav", circle_signer(), params(4)).unwrap_err(),
            FileError::Wav(WavError::TooShort)
        );
        let too_many = (MAX_CHUNKS as usize + 1) * 2;
        let big = wav::encode_pcm16(1, 8000, &vec![0u8; too_many]).unwrap();
        assert_eq!(
            seal_wav(&big, circle_signer(), params(1)).unwrap_err(),
            FileError::TooManyChunks
        );
    }

    #[test]
    fn report_json_shape() {
        let sealed = seal_wav(&sample(100, 1), circle_signer(), params(25)).unwrap();
        let mut tampered = sealed.clone();
        let at = pcm_offset(&tampered);
        tampered[at] ^= 1;
        let json = verify(&tampered, circle_trust()).to_json();
        assert!(json.starts_with("{\"check\":\"invalid\",\"reason\":\"modified\""));
        assert!(json.contains("\"modified_chunks\":[0]"));
        assert!(json.contains("\"key_id\":\"01020304\""));
        let absent = verify(&sample(10, 1), circle_trust()).to_json();
        assert!(absent.contains("\"check\":\"absent\""));
        assert!(absent.contains("\"mode\":null"));
    }
}
