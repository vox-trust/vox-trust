//! A strict, minimal RIFF/WAVE reader and writer for 16-bit PCM, plus the `VOXT` chunk
//! that carries a file-mode manifest.
//!
//! Strictness is deliberate: ambiguity is where attacks hide. Duplicate `fmt `/`data`/`VOXT`
//! chunks, truncated chunks, unsupported formats, trailing bytes after the RIFF container
//! and a RIFF size that disagrees with the chunks are all errors.
//!
//! What a seal covers is narrower than what is parsed here: only the format tag, channel
//! count, sample rate, bits per sample and the PCM bytes of `data` are authenticated.
//! `byte_rate`, extra `fmt ` bytes, other chunks and chunk order are not.

use core::fmt;

/// RIFF chunk id of the file-mode manifest.
pub const MANIFEST_CHUNK_ID: [u8; 4] = *b"VOXT";

/// Why a WAV file was rejected.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum WavError {
    /// Fewer than 12 bytes.
    TooShort,
    /// Not a RIFF/WAVE file.
    NotRiffWave,
    /// A chunk, or the RIFF size, runs past the end of the file.
    TruncatedChunk,
    /// No `fmt ` chunk.
    MissingFmt,
    /// No `data` chunk.
    MissingData,
    /// More than one `fmt ` or `data` chunk.
    DuplicateChunk,
    /// More than one `VOXT` chunk.
    MultipleManifests,
    /// Only 16-bit integer PCM is supported.
    UnsupportedFormat,
    /// `block_align` does not match `channels * 2`, or channels is zero.
    BadBlockAlign,
    /// The data length is not a whole number of frames.
    BadDataLength,
    /// Bytes follow the end of the RIFF container.
    TrailingBytes,
    /// The RIFF size and the chunk sizes do not add up (leftover bytes inside the
    /// container, or a missing pad byte).
    SizeMismatch,
    /// A chunk or the whole file would exceed the 4 GiB limit of RIFF.
    TooLarge,
}

impl fmt::Display for WavError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            WavError::TooShort => "file is too short to be a WAV file",
            WavError::NotRiffWave => "not a RIFF/WAVE file",
            WavError::TruncatedChunk => "a chunk runs past the end of the file",
            WavError::MissingFmt => "no fmt chunk",
            WavError::MissingData => "no data chunk",
            WavError::DuplicateChunk => "more than one fmt or data chunk",
            WavError::MultipleManifests => "more than one VOXT manifest chunk",
            WavError::UnsupportedFormat => "only 16-bit integer PCM WAV is supported",
            WavError::BadBlockAlign => "inconsistent block alignment",
            WavError::BadDataLength => "data length is not a whole number of frames",
            WavError::TrailingBytes => "bytes after the end of the RIFF container",
            WavError::SizeMismatch => "RIFF size does not match the chunks",
            WavError::TooLarge => "chunk or file larger than 4 GiB",
        })
    }
}

impl std::error::Error for WavError {}

/// One RIFF chunk (without its padding byte).
#[derive(Debug, Clone, Copy)]
struct Chunk<'a> {
    id: [u8; 4],
    data: &'a [u8],
}

/// A parsed 16-bit PCM WAV file.
#[derive(Debug, Clone)]
pub struct Wav<'a> {
    /// Number of interleaved channels.
    pub channels: u16,
    /// Sample rate in hertz.
    pub sample_rate: u32,
    /// Always 16 in this version.
    pub bits_per_sample: u16,
    /// Raw little-endian interleaved sample bytes of the `data` chunk.
    pub pcm: &'a [u8],
    /// The `VOXT` manifest chunk, if present.
    pub manifest: Option<&'a [u8]>,
    chunks: Vec<Chunk<'a>>,
}

impl Wav<'_> {
    /// Number of audio frames (one sample per channel).
    pub fn frames(&self) -> u64 {
        (self.pcm.len() / (usize::from(self.channels) * 2)) as u64
    }
}

fn le16(b: &[u8]) -> u16 {
    u16::from_le_bytes([b[0], b[1]])
}

fn le32(b: &[u8]) -> u32 {
    u32::from_le_bytes([b[0], b[1], b[2], b[3]])
}

/// Parses a WAV file, validating everything the protocol relies on.
pub fn parse(bytes: &[u8]) -> Result<Wav<'_>, WavError> {
    if bytes.len() < 12 {
        return Err(WavError::TooShort);
    }
    if &bytes[0..4] != b"RIFF" || &bytes[8..12] != b"WAVE" {
        return Err(WavError::NotRiffWave);
    }
    let riff_size = le32(&bytes[4..8]) as usize;
    let end = riff_size
        .checked_add(8)
        .filter(|end| *end <= bytes.len() && *end >= 12)
        .ok_or(WavError::TruncatedChunk)?;
    if end != bytes.len() {
        return Err(WavError::TrailingBytes);
    }

    let mut chunks = Vec::new();
    let mut pos = 12usize;
    while pos + 8 <= end {
        let id = [bytes[pos], bytes[pos + 1], bytes[pos + 2], bytes[pos + 3]];
        let size = le32(&bytes[pos + 4..pos + 8]) as usize;
        let start = pos + 8;
        let stop = start.checked_add(size).ok_or(WavError::TruncatedChunk)?;
        if stop > end {
            return Err(WavError::TruncatedChunk);
        }
        chunks.push(Chunk {
            id,
            data: &bytes[start..stop],
        });
        pos = stop + (size & 1);
    }
    if pos != end {
        return Err(WavError::SizeMismatch);
    }

    let mut fmt = None;
    let mut data = None;
    let mut manifest = None;
    for chunk in &chunks {
        match &chunk.id {
            b"fmt " => {
                if fmt.replace(chunk.data).is_some() {
                    return Err(WavError::DuplicateChunk);
                }
            }
            b"data" => {
                if data.replace(chunk.data).is_some() {
                    return Err(WavError::DuplicateChunk);
                }
            }
            id if *id == MANIFEST_CHUNK_ID => {
                let duplicate = manifest.replace(chunk.data).is_some();
                if duplicate {
                    return Err(WavError::MultipleManifests);
                }
            }
            _ => {}
        }
    }
    let fmt = fmt.ok_or(WavError::MissingFmt)?;
    let pcm = data.ok_or(WavError::MissingData)?;
    if fmt.len() < 16 {
        return Err(WavError::UnsupportedFormat);
    }
    let (format_tag, channels, sample_rate) =
        (le16(&fmt[0..2]), le16(&fmt[2..4]), le32(&fmt[4..8]));
    let (block_align, bits) = (le16(&fmt[12..14]), le16(&fmt[14..16]));
    if format_tag != 1 || bits != 16 {
        return Err(WavError::UnsupportedFormat);
    }
    if channels == 0 || usize::from(block_align) != usize::from(channels) * 2 {
        return Err(WavError::BadBlockAlign);
    }
    if !pcm.len().is_multiple_of(usize::from(block_align)) {
        return Err(WavError::BadDataLength);
    }
    Ok(Wav {
        channels,
        sample_rate,
        bits_per_sample: bits,
        pcm,
        manifest,
        chunks,
    })
}

fn push_chunk(out: &mut Vec<u8>, id: [u8; 4], data: &[u8]) -> Result<(), WavError> {
    let len = u32::try_from(data.len()).map_err(|_| WavError::TooLarge)?;
    out.extend_from_slice(&id);
    out.extend_from_slice(&len.to_le_bytes());
    out.extend_from_slice(data);
    if data.len() % 2 == 1 {
        out.push(0);
    }
    Ok(())
}

fn finish_riff(body: Vec<u8>) -> Result<Vec<u8>, WavError> {
    let riff_size = body
        .len()
        .checked_add(4)
        .and_then(|n| u32::try_from(n).ok())
        .ok_or(WavError::TooLarge)?;
    let mut out = Vec::with_capacity(body.len() + 12);
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&riff_size.to_le_bytes());
    out.extend_from_slice(b"WAVE");
    out.extend_from_slice(&body);
    Ok(out)
}

/// Re-writes a WAV file with `manifest` as its only `VOXT` chunk, appended last.
///
/// All other chunks are kept, in order. Any existing `VOXT` chunk is replaced. The input
/// must parse strictly (so it has no trailing bytes); the result is [`WavError::TooLarge`]
/// if it would not fit in a RIFF container.
pub fn with_manifest(bytes: &[u8], manifest: &[u8]) -> Result<Vec<u8>, WavError> {
    let wav = parse(bytes)?;
    let mut body = Vec::with_capacity(
        bytes
            .len()
            .saturating_add(manifest.len())
            .saturating_add(16),
    );
    for chunk in wav.chunks.iter().filter(|c| c.id != MANIFEST_CHUNK_ID) {
        push_chunk(&mut body, chunk.id, chunk.data)?;
    }
    push_chunk(&mut body, MANIFEST_CHUNK_ID, manifest)?;
    finish_riff(body)
}

/// Builds a minimal 16-bit PCM WAV file from raw little-endian interleaved sample bytes.
///
/// Fails with [`WavError::BadBlockAlign`] if `channels` is zero or too large,
/// [`WavError::BadDataLength`] if `pcm` is not a whole number of frames, and
/// [`WavError::TooLarge`] if a size does not fit in 32 bits.
pub fn encode_pcm16(channels: u16, sample_rate: u32, pcm: &[u8]) -> Result<Vec<u8>, WavError> {
    if channels == 0 {
        return Err(WavError::BadBlockAlign);
    }
    let block_align = channels.checked_mul(2).ok_or(WavError::BadBlockAlign)?;
    if !pcm.len().is_multiple_of(usize::from(block_align)) {
        return Err(WavError::BadDataLength);
    }
    let byte_rate = sample_rate
        .checked_mul(u32::from(block_align))
        .ok_or(WavError::TooLarge)?;
    let mut fmt = Vec::with_capacity(16);
    fmt.extend_from_slice(&1u16.to_le_bytes());
    fmt.extend_from_slice(&channels.to_le_bytes());
    fmt.extend_from_slice(&sample_rate.to_le_bytes());
    fmt.extend_from_slice(&byte_rate.to_le_bytes());
    fmt.extend_from_slice(&block_align.to_le_bytes());
    fmt.extend_from_slice(&16u16.to_le_bytes());
    let mut body = Vec::new();
    push_chunk(&mut body, *b"fmt ", &fmt)?;
    push_chunk(&mut body, *b"data", pcm)?;
    finish_riff(body)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pcm(frames: usize, channels: usize) -> Vec<u8> {
        (0..frames * channels)
            .flat_map(|i| (i as i16).wrapping_mul(37).to_le_bytes())
            .collect()
    }

    #[test]
    fn encode_then_parse() {
        let bytes = encode_pcm16(2, 44100, &pcm(10, 2)).unwrap();
        let wav = parse(&bytes).unwrap();
        assert_eq!(
            (wav.channels, wav.sample_rate, wav.bits_per_sample),
            (2, 44100, 16)
        );
        assert_eq!(wav.frames(), 10);
        assert_eq!(wav.pcm, pcm(10, 2).as_slice());
        assert!(wav.manifest.is_none());
    }

    #[test]
    fn manifest_is_appended_replaced_and_padded() {
        let base = encode_pcm16(1, 8000, &pcm(5, 1)).unwrap();
        let once = with_manifest(&base, b"abc").unwrap(); // odd length: padded
        assert_eq!(parse(&once).unwrap().manifest, Some(&b"abc"[..]));
        assert_eq!(once.len() % 2, 0);
        let twice = with_manifest(&once, b"defgh").unwrap();
        let wav = parse(&twice).unwrap();
        assert_eq!(wav.manifest, Some(&b"defgh"[..]));
        assert_eq!(wav.pcm, pcm(5, 1).as_slice());
    }

    #[test]
    fn other_chunks_are_preserved() {
        let base = encode_pcm16(1, 8000, &pcm(4, 1)).unwrap();
        // Insert a LIST chunk before `data` by rebuilding: take fmt, add LIST, add data.
        let wav = parse(&base).unwrap();
        let mut body = Vec::new();
        push_chunk(&mut body, *b"fmt ", wav.chunks[0].data).unwrap();
        push_chunk(&mut body, *b"LIST", b"INFOxxxx").unwrap();
        push_chunk(&mut body, *b"data", wav.pcm).unwrap();
        let with_list = finish_riff(body).unwrap();
        let sealed = with_manifest(&with_list, b"m").unwrap();
        let reparsed = parse(&sealed).unwrap();
        assert!(reparsed.chunks.iter().any(|c| &c.id == b"LIST"));
        assert_eq!(reparsed.manifest, Some(&b"m"[..]));
    }

    #[test]
    fn rejects_malformed_files() {
        assert_eq!(parse(b"RIFF").unwrap_err(), WavError::TooShort);
        assert_eq!(parse(&[0u8; 20]).unwrap_err(), WavError::NotRiffWave);

        let good = encode_pcm16(1, 8000, &pcm(4, 1)).unwrap();
        // truncated file
        assert_eq!(
            parse(&good[..good.len() - 3]).unwrap_err(),
            WavError::TruncatedChunk
        );
        // non-PCM format tag
        let mut float = good.clone();
        float[20] = 3;
        assert_eq!(parse(&float).unwrap_err(), WavError::UnsupportedFormat);
        // 8-bit
        let mut eight = good.clone();
        eight[34] = 8;
        assert_eq!(parse(&eight).unwrap_err(), WavError::UnsupportedFormat);
        // zero channels
        let mut zero = good.clone();
        zero[22] = 0;
        assert_eq!(parse(&zero).unwrap_err(), WavError::BadBlockAlign);
        // missing data
        let mut body = Vec::new();
        push_chunk(&mut body, *b"fmt ", parse(&good).unwrap().chunks[0].data).unwrap();
        assert_eq!(
            parse(&finish_riff(body).unwrap()).unwrap_err(),
            WavError::MissingData
        );
    }

    #[test]
    fn rejects_duplicate_and_ambiguous_chunks() {
        let good = encode_pcm16(1, 8000, &pcm(4, 1)).unwrap();
        let wav = parse(&good).unwrap();
        let mut dup_data = Vec::new();
        push_chunk(&mut dup_data, *b"fmt ", wav.chunks[0].data).unwrap();
        push_chunk(&mut dup_data, *b"data", wav.pcm).unwrap();
        push_chunk(&mut dup_data, *b"data", wav.pcm).unwrap();
        assert_eq!(
            parse(&finish_riff(dup_data).unwrap()).unwrap_err(),
            WavError::DuplicateChunk
        );
        let mut two_manifests = Vec::new();
        push_chunk(&mut two_manifests, *b"fmt ", wav.chunks[0].data).unwrap();
        push_chunk(&mut two_manifests, *b"data", wav.pcm).unwrap();
        push_chunk(&mut two_manifests, MANIFEST_CHUNK_ID, b"a").unwrap();
        push_chunk(&mut two_manifests, MANIFEST_CHUNK_ID, b"b").unwrap();
        assert_eq!(
            parse(&finish_riff(two_manifests).unwrap()).unwrap_err(),
            WavError::MultipleManifests
        );
    }

    #[test]
    fn data_must_be_whole_frames() {
        let good = encode_pcm16(2, 8000, &pcm(4, 2)).unwrap();
        let wav = parse(&good).unwrap();
        let mut body = Vec::new();
        push_chunk(&mut body, *b"fmt ", wav.chunks[0].data).unwrap();
        push_chunk(&mut body, *b"data", &wav.pcm[..wav.pcm.len() - 2]).unwrap();
        assert_eq!(
            parse(&finish_riff(body).unwrap()).unwrap_err(),
            WavError::BadDataLength
        );
    }

    #[test]
    fn rejects_trailing_bytes_and_size_mismatches() {
        let good = encode_pcm16(1, 8000, &pcm(4, 1)).unwrap();
        let mut trailing = good.clone();
        trailing.extend_from_slice(b"junk");
        assert_eq!(parse(&trailing).unwrap_err(), WavError::TrailingBytes);
        assert_eq!(
            with_manifest(&trailing, b"m").unwrap_err(),
            WavError::TrailingBytes
        );

        // RIFF size smaller than the file: the rest counts as trailing bytes.
        let mut small = good.clone();
        let size = u32::from_le_bytes(small[4..8].try_into().unwrap()) - 2;
        small[4..8].copy_from_slice(&size.to_le_bytes());
        assert_eq!(parse(&small).unwrap_err(), WavError::TrailingBytes);

        // Leftover bytes inside the container that do not form a chunk header.
        let mut leftover = good.clone();
        leftover.extend_from_slice(&[0, 0, 0]);
        let size = u32::from_le_bytes(leftover[4..8].try_into().unwrap()) + 3;
        leftover[4..8].copy_from_slice(&size.to_le_bytes());
        assert_eq!(parse(&leftover).unwrap_err(), WavError::SizeMismatch);

        // Odd-sized last chunk with its pad byte missing.
        let mut body = Vec::new();
        push_chunk(&mut body, *b"fmt ", parse(&good).unwrap().chunks[0].data).unwrap();
        push_chunk(&mut body, *b"data", &pcm(4, 1)).unwrap();
        push_chunk(&mut body, *b"LIST", b"abc").unwrap();
        let mut file = finish_riff(body).unwrap();
        file.pop();
        let size = u32::from_le_bytes(file[4..8].try_into().unwrap()) - 1;
        file[4..8].copy_from_slice(&size.to_le_bytes());
        assert_eq!(parse(&file).unwrap_err(), WavError::SizeMismatch);
    }

    #[test]
    fn encoder_returns_errors_instead_of_panicking_or_overflowing() {
        assert_eq!(
            encode_pcm16(0, 8000, &[]).unwrap_err(),
            WavError::BadBlockAlign
        );
        assert_eq!(
            encode_pcm16(u16::MAX, 8000, &[]).unwrap_err(),
            WavError::BadBlockAlign
        );
        assert_eq!(
            encode_pcm16(2, 8000, &[0; 3]).unwrap_err(),
            WavError::BadDataLength
        );
        // byte_rate = sample_rate * block_align would overflow u32.
        assert_eq!(
            encode_pcm16(2, u32::MAX, &[]).unwrap_err(),
            WavError::TooLarge
        );
    }

    #[test]
    fn finish_riff_rejects_oversized_bodies() {
        // A zeroed allocation of this size is lazily committed by the OS.
        let body = vec![0u8; u32::MAX as usize - 2];
        assert_eq!(finish_riff(body).unwrap_err(), WavError::TooLarge);
    }
}
