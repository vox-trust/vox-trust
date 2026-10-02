//! Experimental in-band carrier for Vox Trust seals ("stdm-2"; "stdm-1" via
//! [`Params::stdm1`]).
//!
//! **Status: experimental and measured, not reviewed.** A carrier only *carries* the 13-byte
//! seal; it never decides authenticity (spec section 10). A missing or damaged watermark must
//! read as *Absent*: the decoder returns a seal only when blind synchronisation and a CRC both
//! pass, and the protocol then checks the seal's own tag.
//!
//! # Method (prior art, not new)
//!
//! Spread-transform dither modulation (STDM, Chen and Wornell, IEEE Trans. Information
//! Theory, 2001) on the log-magnitude short-time spectrum, the domain used by classic
//! spread-spectrum audio watermarking (Kirovski and Malvar, IEEE Trans. Signal Processing,
//! 2003):
//!
//! - Audio is analysed at 16 kHz with a 512-point STFT (hop 256, square-root Hann windows).
//! - The speech band is cut into *tiles* (a few bins by a few frames). Each tile's level, in
//!   dB, with the column's mean and the slowly varying envelope removed, is one *chip*.
//! - A *window* of tiles carries one seal. Its chips are split into groups: one per coded bit
//!   of the seal, plus synchronisation groups with known bits. Each group's chips are
//!   combined with public pseudo-random signs into one *projection*.
//! - Embedding moves each projection to the nearest point of a quantisation lattice (step
//!   `step_db`, offset by a public dither) whose parity is the bit. Unlike additive spread
//!   spectrum this rejects interference from the audio itself by construction: the required
//!   move is at most half a step, whatever the audio. Every tile change is also clamped to
//!   `max_db`. Working on normalised dB levels makes the method insensitive to overall
//!   volume changes, the classic weakness of quantisation methods.
//! - Embedding is closed-loop: it re-analyses the marked audio and corrects the remaining
//!   error a few times, because STFT frames overlap.
//! - The detector searches every time offset (to a quarter of a hop) and, optionally, small
//!   tempo changes (by spacing its analysis frames differently), scores how closely the
//!   synchronisation groups sit on their lattices, decodes the candidates far above chance
//!   with a soft Viterbi decoder, and keeps a result only if the CRC matches.
//!
//! stdm-2 differs from stdm-1 only in its window: 300 tile columns (9.6 s) instead of 200
//! (6.4 s), which the benchmark showed to be more robust at the same quality. The detector's
//! tempo search is on by default (plus or minus 2 %); it does not change the format.
//!
//! The pattern is public: anyone can detect a seal, and anyone can try to remove it (which
//! yields *Absent*). **Anyone can also read a seal from one recording and embed it into
//! another**: the in-band seal is not bound to the audio content (the copy attack, threat
//! model A11). Until it is, an in-band seal must not be shown to a user as *Verified*; this
//! crate is for measurement and research.
//!
//! Robustness and quality numbers come from the benchmark in `bench/`, not from this
//! documentation.
#![forbid(unsafe_code)]
#![deny(missing_docs)]

pub mod fec;
mod layout;
mod stft;

use layout::Layout;
use stft::Stft;

/// Number of bytes of a packed seal.
pub const SEAL_BYTES: usize = 13;
/// Sample rate the carrier works at. Callers resample to and from it.
pub const SAMPLE_RATE: u32 = 16_000;
/// STFT size in samples.
pub const FRAME: usize = 512;
/// STFT hop in samples.
pub const HOP: usize = 256;

/// Carrier parameters. [`Params::default`] is the measured configuration; the fields are
/// public so the benchmark can explore trade-offs. Embedder and detector must agree on all
/// fields except `sync_threshold`.
#[derive(Debug, Clone, PartialEq)]
pub struct Params {
    /// First STFT bin of the band (inclusive).
    pub lo_bin: usize,
    /// Last STFT bin of the band (exclusive).
    pub hi_bin: usize,
    /// Bins per tile.
    pub tile_bins: usize,
    /// STFT frames per tile.
    pub tile_frames: usize,
    /// Tile columns per window (one seal per window).
    pub columns: usize,
    /// One tile in `sync_every` belongs to a synchronisation group.
    pub sync_every: usize,
    /// Quantisation step of the projections, in dB.
    pub step_db: f32,
    /// Largest change applied to any tile, in dB.
    pub max_db: f32,
    /// Detector: minimum synchronisation score (in standard deviations under the
    /// no-watermark hypothesis).
    pub sync_threshold: f32,
    /// Tile columns quieter than the loudest column by more than this carry no chips.
    pub silence_db: f32,
    /// Tiles quieter than the loudest tile of their column by more than this carry no chips
    /// (spectral valleys are mostly noise and codecs discard them).
    pub valley_db: f32,
    /// Seed of the public chip pattern.
    pub pattern_seed: u64,
    /// Detector only: also search tempo changes up to this many percent either way, in
    /// steps of [`TEMPO_STEP_PCT`] (0 disables the search). It does not change the format.
    pub max_tempo_pct: f32,
}

/// Step of the detector's tempo search, in percent.
pub const TEMPO_STEP_PCT: f32 = 0.5;

/// stdm-2.
impl Default for Params {
    fn default() -> Self {
        Params {
            lo_bin: 10,
            hi_bin: 122,
            tile_bins: 8,
            tile_frames: 2,
            columns: 300,
            sync_every: 6,
            step_db: 7.0,
            max_db: 4.5,
            sync_threshold: 6.0,
            silence_db: 45.0,
            valley_db: 30.0,
            // "vox-ss1\x01" in ASCII: the seed of the measured configuration.
            pattern_seed: 0x766f_782d_7373_3101,
            max_tempo_pct: 2.0,
        }
    }
}

impl Params {
    /// stdm-1, the first measured configuration: 200-column (6.4 s) windows, no tempo
    /// search. Its seals are not readable with the stdm-2 defaults, and the reverse.
    pub fn stdm1() -> Params {
        Params {
            columns: 200,
            max_tempo_pct: 0.0,
            ..Params::default()
        }
    }

    /// Number of subbands per column.
    pub fn subbands(&self) -> usize {
        (self.hi_bin - self.lo_bin) / self.tile_bins
    }

    /// Audio samples (at [`SAMPLE_RATE`]) covered by one window.
    pub fn window_samples(&self) -> usize {
        self.columns * self.tile_frames * HOP
    }

    /// Payload capacity in bits per second (102 seal bits per window).
    pub fn capacity_bps(&self) -> f32 {
        fec::SEAL_BITS as f32 * SAMPLE_RATE as f32 / self.window_samples() as f32
    }

    fn validate(&self) -> Result<(), CarrierError> {
        let ok = self.tile_bins > 0
            && self.tile_frames > 0
            && self.lo_bin >= 1
            && self.hi_bin <= FRAME / 2
            && self.lo_bin < self.hi_bin
            && self.subbands() > 0
            && self.sync_every >= 2
            && self.columns > 0
            && self.step_db > 0.0
            && self.max_db > 0.0
            && (0.0..=10.0).contains(&self.max_tempo_pct);
        if !ok {
            return Err(CarrierError::BadParams);
        }
        let layout = Layout::new(self);
        if layout.chips_per_group() < 4 || layout.sync_groups().len() < 16 {
            return Err(CarrierError::BadParams);
        }
        Ok(())
    }
}

/// Errors from the carrier.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum CarrierError {
    /// The parameters are inconsistent or leave too few chips per bit.
    BadParams,
    /// The audio is shorter than one window.
    TooShort,
}

impl core::fmt::Display for CarrierError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            CarrierError::BadParams => f.write_str("inconsistent carrier parameters"),
            CarrierError::TooShort => f.write_str("audio is shorter than one carrier window"),
        }
    }
}

impl std::error::Error for CarrierError {}

/// A seal found by the detector.
#[derive(Debug, Clone, PartialEq)]
pub struct Detection {
    /// The 13 seal bytes (the two pad bits are zero). Not yet authenticated.
    pub seal: [u8; SEAL_BYTES],
    /// Sample index (at 16 kHz) where the window starts in the analysed audio.
    pub start_sample: usize,
    /// Normalised synchronisation correlation of this window.
    pub sync_score: f32,
}

/// Embedding iterations: each one re-analyses the marked audio and corrects every
/// projection towards its lattice point, which compensates for the STFT's overlap.
const EMBED_ITERATIONS: usize = 4;

/// Embeds the seal into every whole window of `samples` (mono, 16 kHz, nominal range ±1).
/// Audio more than one frame ([`FRAME`] samples) after the last whole window is unchanged.
pub fn embed(
    samples: &[f32],
    seal: &[u8; SEAL_BYTES],
    params: &Params,
) -> Result<Vec<f32>, CarrierError> {
    params.validate()?;
    let layout = Layout::new(params);
    let mut stft = Stft::new();
    let frames = stft.analyse(samples, 0);
    let cols_total = frames.len() / params.tile_frames;
    let windows = cols_total / params.columns;
    if windows == 0 {
        return Err(CarrierError::TooShort);
    }
    let coded = fec::encode(&fec::info_bits(seal));
    let sb = params.subbands();
    let groups: Vec<(&layout::Group, bool)> = layout
        .data_groups()
        .iter()
        .zip(coded.iter().copied())
        .chain(layout.sync_groups().iter().map(|g| (g, g.known_bit)))
        .collect();
    // Per tile (column * subbands + subband), the change in dB.
    let mut tile_db = vec![0f32; cols_total * sb];
    // Per window and group, the lattice point chosen at the first iteration.
    let mut targets: Vec<Option<f32>> = vec![None; windows * groups.len()];
    let mut current = frames.clone();

    for _ in 0..EMBED_ITERATIONS {
        let feats = tile_features(&current, params);
        for w in 0..windows {
            let base = w * params.columns * sb;
            for (gi, (group, bit)) in groups.iter().enumerate() {
                let Some(p) = feats.project(base, group, &layout) else {
                    continue;
                };
                let target = *targets[w * groups.len() + gi]
                    .get_or_insert_with(|| lattice_point(p, group.dither, *bit, params.step_db));
                let step = target - p;
                for &c in &group.chips {
                    // Only chips the detector counts; silent and valley tiles stay untouched.
                    if feats.weight[base + c] > 0.0 {
                        let t = &mut tile_db[base + c];
                        *t = (*t + step * layout.sign(c)).clamp(-params.max_db, params.max_db);
                    }
                }
            }
        }
        let gains_db = expand_to_frames(&tile_db, params, frames.len());
        let delta = stft.synthesise_gain_delta(&frames, &gains_db, params, samples.len());
        let marked: Vec<f32> = samples.iter().zip(&delta).map(|(x, d)| x + d).collect();
        current = stft.analyse(&marked, 0);
    }

    let gains_db = expand_to_frames(&tile_db, params, frames.len());
    let delta = stft.synthesise_gain_delta(&frames, &gains_db, params, samples.len());
    Ok(samples.iter().zip(delta).map(|(x, d)| x + d).collect())
}

/// The point of the lattice for `bit` (`(dither + bit / 2 + k) * step`) nearest to `p`.
fn lattice_point(p: f32, dither: f32, bit: bool, step: f32) -> f32 {
    let offset = (dither + if bit { 0.5 } else { 0.0 }) * step;
    offset + step * ((p - offset) / step).round()
}

/// Where a projection sits relative to its lattices, as an angle: 0 on the lattice for bit
/// 0, pi on the lattice for bit 1.
fn lattice_phase(p: f32, dither: f32, step: f32) -> f32 {
    2.0 * std::f32::consts::PI * (p / step - dither)
}

fn expand_to_frames(tile_db: &[f32], p: &Params, frames: usize) -> Vec<f32> {
    let sb = p.subbands();
    let mut out = vec![0f32; frames * sb];
    for (i, &g) in tile_db.iter().enumerate() {
        let (col, q) = (i / sb, i % sb);
        for f in 0..p.tile_frames {
            out[(col * p.tile_frames + f) * sb + q] = g;
        }
    }
    out
}

/// Searches `samples` (mono, 16 kHz) for sealed windows and returns every window whose
/// synchronisation and CRC both pass, strongest first. An empty result means *Absent*.
///
/// With `max_tempo_pct > 0`, the search is repeated at each tempo in the range (audio
/// played faster or slower, same pitch), and windows found at several tempos are kept once,
/// with their best score.
pub fn detect(samples: &[f32], params: &Params) -> Result<Vec<Detection>, CarrierError> {
    let mut found = detect_at(&Scan::new(samples, params)?, params);
    let steps = (params.max_tempo_pct / TEMPO_STEP_PCT).floor() as i32;
    for k in (1..=steps).flat_map(|k| [k, -k]) {
        let tempo = 1.0 + f64::from(k) * f64::from(TEMPO_STEP_PCT) / 100.0;
        match Scan::with_tempo(samples, params, tempo) {
            Ok(scan) => found.extend(detect_at(&scan, params)),
            Err(CarrierError::TooShort) => continue,
            Err(e) => return Err(e),
        }
    }
    found.sort_by(|a, b| b.sync_score.total_cmp(&a.sync_score));
    let min_gap = params.window_samples() / 2;
    let mut kept: Vec<Detection> = Vec::new();
    for d in found {
        if kept
            .iter()
            .all(|k| k.start_sample.abs_diff(d.start_sample) >= min_gap)
        {
            kept.push(d);
        }
    }
    Ok(kept)
}

fn detect_at(scan: &Scan, params: &Params) -> Vec<Detection> {
    let mut found: Vec<Detection> = Vec::new();
    for cand in scan.candidates() {
        let soft = scan.soft_bits(cand.shift, cand.col0);
        let Some(seal) = fec::seal_from_info(&fec::decode(&soft)) else {
            continue;
        };
        let column = (params.tile_frames * HOP) as f64 / scan.tempo;
        found.push(Detection {
            seal,
            start_sample: cand.shift + (cand.col0 as f64 * column).round() as usize,
            sync_score: cand.score,
        });
    }
    found.sort_by(|a, b| b.sync_score.total_cmp(&a.sync_score));
    found
}

/// Low-level access to the detector, for measurement (bit error rates, score
/// distributions). Applications should use [`detect`].
pub struct Scan {
    params: Params,
    layout: Layout,
    /// Per quarter-hop shift: tile features of every column.
    shifts: Vec<Features>,
    /// Tempo the analysis assumes (1.0 = unchanged; 1.01 = played 1 % faster).
    tempo: f64,
}

/// A synchronisation candidate.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Candidate {
    /// Sample shift of the analysis grid (a multiple of a quarter hop).
    pub shift: usize,
    /// First tile column of the window on that grid.
    pub col0: usize,
    /// Normalised synchronisation correlation.
    pub score: f32,
}

impl Scan {
    /// Analyses the audio at four quarter-hop shifts.
    pub fn new(samples: &[f32], params: &Params) -> Result<Scan, CarrierError> {
        Scan::with_tempo(samples, params, 1.0)
    }

    /// Analyses the audio as if it had been played `tempo` times faster (same pitch), by
    /// spacing the analysis frames `HOP / tempo` samples apart.
    pub fn with_tempo(samples: &[f32], params: &Params, tempo: f64) -> Result<Scan, CarrierError> {
        params.validate()?;
        if !(0.5..=2.0).contains(&tempo) {
            return Err(CarrierError::BadParams);
        }
        let mut stft = Stft::new();
        let mut shifts = Vec::new();
        for s in 0..4 {
            let shift = s * HOP / 4;
            let frames = stft.analyse_hop(samples, shift, HOP as f64 / tempo);
            // A shifted grid may be one column short of a window; only the unshifted grid
            // has to hold one.
            if s == 0 && frames.len() / params.tile_frames < params.columns {
                return Err(CarrierError::TooShort);
            }
            shifts.push(tile_features(&frames, params));
        }
        Ok(Scan {
            params: params.clone(),
            layout: Layout::new(params),
            shifts,
            tempo,
        })
    }

    /// Synchronisation score of the window starting at `col0` on grid `shift`: how closely
    /// the synchronisation groups sit on the lattices of their known bits, in standard
    /// deviations of the same score on unmarked audio (where the dither makes every phase
    /// equally likely).
    /// Returns 0 if the window does not fit in the audio.
    pub fn sync_score(&self, shift: usize, col0: usize) -> f32 {
        if !self.fits(shift, col0) {
            return 0.0;
        }
        let f = &self.shifts[shift / (HOP / 4)];
        let base = col0 * self.params.subbands();
        let (mut sum, mut n) = (0f32, 0f32);
        for g in self.layout.sync_groups() {
            if let Some(p) = f.project(base, g, &self.layout) {
                let phase = lattice_phase(p, g.dither, self.params.step_db);
                let expected = if g.known_bit {
                    std::f32::consts::PI
                } else {
                    0.0
                };
                sum += (phase - expected).cos();
                n += 1.0;
            }
        }
        if n > 0.0 {
            sum / (n / 2.0).sqrt()
        } else {
            0.0
        }
    }

    fn fits(&self, shift: usize, col0: usize) -> bool {
        shift.is_multiple_of(HOP / 4)
            && shift < HOP
            && col0 + self.params.columns <= self.columns(shift)
    }

    /// Number of tile columns on grid `shift`.
    pub fn columns(&self, shift: usize) -> usize {
        self.shifts[shift / (HOP / 4)].level.len()
    }

    /// Every local maximum of the synchronisation score above the threshold, strongest
    /// first, at least half a window apart.
    pub fn candidates(&self) -> Vec<Candidate> {
        let mut all = Vec::new();
        for s in 0..4 {
            let shift = s * HOP / 4;
            let cols = self.columns(shift);
            for col0 in 0..=cols.saturating_sub(self.params.columns) {
                let score = self.sync_score(shift, col0);
                if score >= self.params.sync_threshold {
                    all.push(Candidate { shift, col0, score });
                }
            }
        }
        all.sort_by(|a, b| b.score.total_cmp(&a.score));
        let span = self.params.tile_frames * HOP;
        let min_gap = self.params.columns * span / 2;
        let mut kept: Vec<Candidate> = Vec::new();
        for c in all {
            let pos = c.shift + c.col0 * span;
            if kept
                .iter()
                .all(|k| (k.shift + k.col0 * span).abs_diff(pos) >= min_gap)
            {
                kept.push(c);
            }
        }
        kept
    }

    /// Soft values of the 248 coded bits for the window at (`shift`, `col0`), in [-1, 1];
    /// positive means 1, zero means no information (all zeros if the window does not fit).
    pub fn soft_bits(&self, shift: usize, col0: usize) -> [f32; fec::CODED_BITS] {
        let mut soft = [0f32; fec::CODED_BITS];
        if !self.fits(shift, col0) {
            return soft;
        }
        let f = &self.shifts[shift / (HOP / 4)];
        let base = col0 * self.params.subbands();
        for (s, g) in soft.iter_mut().zip(self.layout.data_groups()) {
            if let Some(p) = f.project(base, g, &self.layout) {
                *s = -lattice_phase(p, g.dither, self.params.step_db).cos();
            }
        }
        soft
    }
}

/// Normalised tile levels: the mean log-power of each tile, with the column's mean removed
/// (gain changes do not matter) and the slowly varying envelope removed (the mean of the
/// neighbouring columns), plus a weight per tile.
struct Features {
    value: Vec<f32>,
    /// Column energy in dB.
    level: Vec<f32>,
    /// Tile log-power in dB before normalisation.
    raw_level: Vec<f32>,
    /// Per tile: how much it counts (0 = silent column or deep spectral valley, 1 = clearly
    /// above both limits, with a ramp so that small level changes move weights only a
    /// little). Embedder and detector compute it the same way on the audio they see.
    weight: Vec<f32>,
}

/// Width of the ramp between "ignored" and "full weight", in dB.
const WEIGHT_RAMP_DB: f32 = 6.0;

impl Features {
    /// Weighted mean of a group's chips times their signs, or `None` if every chip of the
    /// group has zero weight.
    fn project(&self, base: usize, group: &layout::Group, layout: &Layout) -> Option<f32> {
        let (mut num, mut total) = (0f32, 0f32);
        for &c in &group.chips {
            let w = self.weight[base + c];
            num += w * layout.sign(c) * self.value[base + c];
            total += w;
        }
        (total > 0.0).then(|| num / total)
    }

    fn weigh(&mut self, p: &Params) {
        let sb = p.subbands();
        let loudest = self.level.iter().copied().fold(f32::NEG_INFINITY, f32::max);
        let ramp = |x: f32| (x / WEIGHT_RAMP_DB).clamp(0.0, 1.0);
        for (col, &l) in self.level.iter().enumerate() {
            let tiles = &self.raw_level[col * sb..(col + 1) * sb];
            let peak = tiles.iter().copied().fold(f32::NEG_INFINITY, f32::max);
            let wc = ramp(l - (loudest - p.silence_db));
            for (q, &t) in tiles.iter().enumerate() {
                self.weight[col * sb + q] = wc * ramp(t - (peak - p.valley_db));
            }
        }
    }
}

/// Columns on each side used to estimate the slowly varying spectral envelope.
const ENVELOPE_COLUMNS: usize = 2;

fn tile_features(frames: &[stft::Spectrum], p: &Params) -> Features {
    let floor = stft::power_floor(frames);
    let sb = p.subbands();
    let cols = frames.len() / p.tile_frames;
    let mut raw = vec![0f32; cols * sb];
    let mut raw_level = vec![0f32; cols * sb];
    let mut level = vec![0f32; cols];
    for col in 0..cols {
        let row = &mut raw[col * sb..(col + 1) * sb];
        let mut energy = 0f32;
        for (q, v) in row.iter_mut().enumerate() {
            let mut acc = 0f32;
            for f in 0..p.tile_frames {
                let frame = &frames[col * p.tile_frames + f];
                for b in 0..p.tile_bins {
                    let pw = frame[p.lo_bin + q * p.tile_bins + b].norm_sqr();
                    energy += pw;
                    acc += 10.0 * (pw + floor).log10();
                }
            }
            *v = acc / (p.tile_frames * p.tile_bins) as f32;
            raw_level[col * sb + q] = *v;
        }
        let mean = row.iter().sum::<f32>() / sb as f32;
        row.iter_mut().for_each(|v| *v -= mean);
        level[col] = 10.0 * (energy + floor).log10();
    }
    let mut value = vec![0f32; cols * sb];
    for col in 0..cols {
        let lo = col.saturating_sub(ENVELOPE_COLUMNS);
        let hi = (col + ENVELOPE_COLUMNS + 1).min(cols);
        let n = (hi - lo - 1).max(1) as f32;
        for q in 0..sb {
            let around: f32 = (lo..hi)
                .filter(|&c| c != col)
                .map(|c| raw[c * sb + q])
                .sum();
            value[col * sb + q] = raw[col * sb + q] - around / n;
        }
    }
    let mut f = Features {
        value,
        level,
        raw_level,
        weight: vec![1.0; cols * sb],
    };
    f.weigh(p);
    f
}

#[cfg(test)]
mod tests;
