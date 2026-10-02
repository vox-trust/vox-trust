//! `vt-bench`: measures how the experimental carrier survives real audio paths.
//!
//! For every corpus file and several different seals it embeds the seal, sends the audio
//! through each condition (real codecs via ffmpeg, plus a few signal-level changes), runs the
//! detector, and counts the windows whose seal comes back exactly. The same conditions are
//! applied to the unmarked audio to count false alarms. Results go to `results.json` and
//! `results.md`.
//!
//! Usage:
//!   vt-bench --corpus bench/corpus --out bench/results/runs/NAME [--ffmpeg PATH]
//!            [--seals N] [--jobs N] [--only cond1,cond2] [carrier overrides]
//! Carrier overrides: --step DB --max-db DB --columns N --tile-bins N --tile-frames N
//!                    --sync-every N --valley DB --lo-bin N --hi-bin N --threshold Z --seed N
//!                    --max-tempo PCT (detector only: tempo search)

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;
use std::time::Instant;

use vox_trust_carrier::{self as carrier, Params, SAMPLE_RATE};
use vox_trust_core::{circle, wav, Seal};

/// Test key for the benchmark seals. Public on purpose: never use it for anything else.
const BENCH_KEY: [u8; 32] = [0x42; 32];

#[derive(Clone)]
enum Condition {
    /// No change.
    Identity,
    /// ffmpeg encode with these arguments to a file of this extension, then decode to 16 kHz.
    Codec(&'static [&'static str], &'static str),
    /// ffmpeg filter graph applied at 16 kHz.
    Filter(&'static str),
    /// Two codecs in a row (re-sharing).
    Chain(
        &'static [&'static str],
        &'static str,
        &'static [&'static str],
        &'static str,
    ),
    /// White noise at this SNR (dB).
    Noise(f32),
    /// Remove this many samples from the start.
    Trim(usize),
}

fn conditions() -> Vec<(&'static str, &'static str, Condition)> {
    use Condition::*;
    vec![
        ("original", "no change", Identity),
        (
            "resample-44k",
            "resample to 44.1 kHz and back",
            Filter("aresample=44100,aresample=16000"),
        ),
        (
            "trim-1.234s",
            "first 1.234 s removed (tests blind synchronisation)",
            Trim(19_744),
        ),
        ("noise-20db", "white noise at 20 dB SNR", Noise(20.0)),
        ("noise-30db", "white noise at 30 dB SNR", Noise(30.0)),
        ("noise-10db", "white noise at 10 dB SNR", Noise(10.0)),
        (
            "mp3-128k",
            "MP3 128 kbit/s (LAME, 44.1 kHz)",
            Codec(
                &["-ar", "44100", "-c:a", "libmp3lame", "-b:a", "128k"],
                "mp3",
            ),
        ),
        (
            "mp3-64k",
            "MP3 64 kbit/s (LAME, 44.1 kHz)",
            Codec(
                &["-ar", "44100", "-c:a", "libmp3lame", "-b:a", "64k"],
                "mp3",
            ),
        ),
        (
            "aac-64k",
            "AAC-LC 64 kbit/s (ffmpeg aac, 44.1 kHz)",
            Codec(&["-ar", "44100", "-c:a", "aac", "-b:a", "64k"], "m4a"),
        ),
        (
            "opus-32k",
            "Opus 32 kbit/s, VoIP mode",
            Codec(
                &["-c:a", "libopus", "-b:a", "32k", "-application", "voip"],
                "ogg",
            ),
        ),
        (
            "opus-24k",
            "Opus 24 kbit/s, VoIP mode",
            Codec(
                &["-c:a", "libopus", "-b:a", "24k", "-application", "voip"],
                "ogg",
            ),
        ),
        (
            "opus-12k",
            "Opus 12 kbit/s, VoIP mode",
            Codec(
                &["-c:a", "libopus", "-b:a", "12k", "-application", "voip"],
                "ogg",
            ),
        ),
        (
            "amr-wb-12.65k",
            "AMR-WB 12.65 kbit/s (mobile HD voice)",
            Codec(
                &["-ar", "16000", "-c:a", "libvo_amrwbenc", "-b:a", "12.65k"],
                "amr",
            ),
        ),
        (
            "amr-wb-23.85k",
            "AMR-WB 23.85 kbit/s",
            Codec(
                &["-ar", "16000", "-c:a", "libvo_amrwbenc", "-b:a", "23.85k"],
                "amr",
            ),
        ),
        (
            "g722",
            "G.722 64 kbit/s (wideband VoIP)",
            Codec(&["-ar", "16000", "-c:a", "g722"], "wav"),
        ),
        (
            "mp3-128k+opus-24k",
            "MP3 128k, then re-shared as Opus 24k",
            Chain(
                &["-ar", "44100", "-c:a", "libmp3lame", "-b:a", "128k"],
                "mp3",
                &["-c:a", "libopus", "-b:a", "24k", "-application", "voip"],
                "ogg",
            ),
        ),
        (
            "denoise",
            "ffmpeg afftdn noise reduction",
            Filter("afftdn=nr=20:nf=-40"),
        ),
        (
            "echo",
            "two simulated reflections (40 and 60 ms)",
            Filter("aecho=0.8:0.7:40|60:0.3|0.2"),
        ),
        (
            "tempo+1%",
            "1 % faster, same pitch (atempo)",
            Filter("atempo=1.01"),
        ),
    ]
}

struct Args {
    corpus: PathBuf,
    out: PathBuf,
    ffmpeg: String,
    seals: usize,
    jobs: usize,
    only: Option<Vec<String>>,
    params: Params,
}

fn parse_args() -> Result<Args, String> {
    let mut a = Args {
        corpus: PathBuf::from("bench/corpus"),
        out: PathBuf::from("bench/results/runs/latest"),
        ffmpeg: "ffmpeg".into(),
        seals: 3,
        jobs: std::thread::available_parallelism().map_or(2, |n| n.get()),
        only: None,
        params: Params::default(),
    };
    let mut it = std::env::args().skip(1);
    while let Some(flag) = it.next() {
        let mut val = || it.next().ok_or(format!("{flag} needs a value"));
        let num = |v: String| v.parse::<f64>().map_err(|e| format!("{flag}: {e}"));
        match flag.as_str() {
            "--corpus" => a.corpus = val()?.into(),
            "--out" => a.out = val()?.into(),
            "--ffmpeg" => a.ffmpeg = val()?,
            "--seals" => a.seals = num(val()?)? as usize,
            "--jobs" => a.jobs = num(val()?)? as usize,
            "--only" => a.only = Some(val()?.split(',').map(String::from).collect()),
            "--step" => a.params.step_db = num(val()?)? as f32,
            "--max-db" => a.params.max_db = num(val()?)? as f32,
            "--columns" => a.params.columns = num(val()?)? as usize,
            "--max-tempo" => a.params.max_tempo_pct = num(val()?)? as f32,
            "--tile-bins" => a.params.tile_bins = num(val()?)? as usize,
            "--tile-frames" => a.params.tile_frames = num(val()?)? as usize,
            "--sync-every" => a.params.sync_every = num(val()?)? as usize,
            "--valley" => a.params.valley_db = num(val()?)? as f32,
            "--silence" => a.params.silence_db = num(val()?)? as f32,
            "--seed" => a.params.pattern_seed = num(val()?)? as u64,
            "--lo-bin" => a.params.lo_bin = num(val()?)? as usize,
            "--hi-bin" => a.params.hi_bin = num(val()?)? as usize,
            "--threshold" => a.params.sync_threshold = num(val()?)? as f32,
            "-h" | "--help" => {
                println!(
                    "{}",
                    include_str!("main.rs")
                        .lines()
                        .skip(1)
                        .take(12)
                        .map(|l| l.trim_start_matches("//!").trim_start())
                        .collect::<Vec<_>>()
                        .join("\n")
                );
                std::process::exit(0);
            }
            other => return Err(format!("unknown option {other}")),
        }
    }
    Ok(a)
}

fn read_wav(path: &Path) -> Result<Vec<f32>, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let w = wav::parse(&bytes).map_err(|e| format!("{}: {e}", path.display()))?;
    if w.channels != 1 || w.sample_rate != SAMPLE_RATE {
        return Err(format!("{}: need 16 kHz mono", path.display()));
    }
    Ok(w.pcm
        .as_chunks::<2>()
        .0
        .iter()
        .map(|&c| f32::from(i16::from_le_bytes(c)) / 32768.0)
        .collect())
}

fn write_wav(path: &Path, samples: &[f32]) -> Result<(), String> {
    let pcm: Vec<u8> = samples
        .iter()
        .flat_map(|&x| ((x * 32768.0).round().clamp(-32768.0, 32767.0) as i16).to_le_bytes())
        .collect();
    let bytes = wav::encode_pcm16(1, SAMPLE_RATE, &pcm).map_err(|e| e.to_string())?;
    std::fs::write(path, bytes).map_err(|e| format!("{}: {e}", path.display()))
}

fn ffmpeg(bin: &str, args: &[&str]) -> Result<(), String> {
    let out = Command::new(bin)
        .args(["-hide_banner", "-loglevel", "error", "-y"])
        .args(args)
        .output()
        .map_err(|e| format!("cannot run {bin}: {e}"))?;
    if out.status.success() {
        Ok(())
    } else {
        Err(format!(
            "ffmpeg {args:?}: {}",
            String::from_utf8_lossy(&out.stderr)
        ))
    }
}

fn to_16k(bin: &str, input: &Path, output: &Path) -> Result<(), String> {
    let (i, o) = (input.to_str().unwrap(), output.to_str().unwrap());
    ffmpeg(
        bin,
        &["-i", i, "-ar", "16000", "-ac", "1", "-c:a", "pcm_s16le", o],
    )
}

/// Applies a condition; returns the processed 16 kHz samples.
fn apply(
    cond: &Condition,
    samples: &[f32],
    tmp: &Path,
    bin: &str,
    seed: u64,
) -> Result<Vec<f32>, String> {
    let input = tmp.join("in.wav");
    let output = tmp.join("out.wav");
    match cond {
        Condition::Identity => return Ok(samples.to_vec()),
        Condition::Trim(n) => return Ok(samples[(*n).min(samples.len())..].to_vec()),
        Condition::Noise(snr) => {
            let power = samples.iter().map(|x| x * x).sum::<f32>() / samples.len() as f32;
            let sigma = (power / 10f32.powf(snr / 10.0)).sqrt();
            let mut state = seed | 1;
            return Ok(samples
                .iter()
                .map(|x| {
                    // Box-Muller on a xorshift generator: Gaussian white noise.
                    let mut u = || {
                        state ^= state << 13;
                        state ^= state >> 7;
                        state ^= state << 17;
                        ((state >> 11) as f32 + 1.0) / (1u64 << 53) as f32
                    };
                    let g = (-2.0 * u().ln()).sqrt() * (2.0 * std::f32::consts::PI * u()).cos();
                    x + sigma * g
                })
                .collect());
        }
        _ => {}
    }
    write_wav(&input, samples)?;
    let i = input.to_str().unwrap();
    match cond {
        Condition::Filter(graph) => ffmpeg(
            bin,
            &[
                "-i",
                i,
                "-af",
                graph,
                "-ar",
                "16000",
                "-ac",
                "1",
                "-c:a",
                "pcm_s16le",
                output.to_str().unwrap(),
            ],
        )?,
        Condition::Codec(enc, ext) => {
            let mid = tmp.join(format!("mid.{ext}"));
            let mut args = vec!["-i", i];
            args.extend_from_slice(enc);
            args.push(mid.to_str().unwrap());
            ffmpeg(bin, &args)?;
            to_16k(bin, &mid, &output)?;
        }
        Condition::Chain(e1, x1, e2, x2) => {
            let m1 = tmp.join(format!("m1.{x1}"));
            let m2 = tmp.join(format!("m2.{x2}"));
            let mut args = vec!["-i", i];
            args.extend_from_slice(e1);
            args.push(m1.to_str().unwrap());
            ffmpeg(bin, &args)?;
            let mut args = vec!["-i", m1.to_str().unwrap()];
            args.extend_from_slice(e2);
            args.push(m2.to_str().unwrap());
            ffmpeg(bin, &args)?;
            to_16k(bin, &m2, &output)?;
        }
        _ => unreachable!(),
    }
    read_wav(&output)
}

#[derive(Default, Clone)]
struct Tally {
    windows: usize,
    recovered: usize,
    wrong: usize,
    files_with_seal: usize,
    files: usize,
    false_alarms: usize,
    unmarked_files: usize,
    max_unmarked_score: f32,
    marked_scores: Vec<f32>,
}

fn seal_for(file: usize, k: usize) -> [u8; 13] {
    let key_id = circle::key_id(&BENCH_KEY);
    let mut s = Seal::new_circle(&BENCH_KEY, key_id, (file * 16 + k) as u16, 1000 + k as u16)
        .to_bytes()
        .expect("version 0 encodes");
    s[12] &= 0xFC;
    s
}

fn main() {
    let args = match parse_args() {
        Ok(a) => a,
        Err(e) => {
            eprintln!("vt-bench: {e}");
            std::process::exit(64);
        }
    };
    if let Err(e) = run(&args) {
        eprintln!("vt-bench: {e}");
        std::process::exit(1);
    }
}

fn run(args: &Args) -> Result<(), String> {
    let mut files: Vec<PathBuf> = std::fs::read_dir(&args.corpus)
        .map_err(|e| format!("{}: {e}", args.corpus.display()))?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x == "wav"))
        .collect();
    files.sort();
    if files.is_empty() {
        return Err("empty corpus; run bench/fetch-corpus.sh first".into());
    }
    std::fs::create_dir_all(args.out.join("marked")).map_err(|e| e.to_string())?;
    let conds: Vec<_> = conditions()
        .into_iter()
        .filter(|(n, _, _)| args.only.as_ref().is_none_or(|o| o.iter().any(|x| x == n)))
        .collect();
    let p = &args.params;

    // Jobs: (file, Some(seal index)) for marked audio, (file, None) for the unmarked control.
    let mut jobs = Vec::new();
    for f in 0..files.len() {
        jobs.push((f, None));
        for k in 0..args.seals {
            jobs.push((f, Some(k)));
        }
    }
    let next = AtomicUsize::new(0);
    let tallies: Mutex<BTreeMap<&str, Tally>> = Mutex::new(BTreeMap::new());
    let quality: Mutex<Vec<(String, f32, f32)>> = Mutex::new(Vec::new());
    let errors: Mutex<Vec<String>> = Mutex::new(Vec::new());
    let timing: Mutex<(f64, f64, f64)> = Mutex::new((0.0, 0.0, 0.0));
    let started = Instant::now();

    std::thread::scope(|scope| {
        for worker in 0..args.jobs.max(1) {
            let (next, tallies, quality, errors, timing, files, conds, jobs) = (
                &next, &tallies, &quality, &errors, &timing, &files, &conds, &jobs,
            );
            scope.spawn(move || {
                let tmp = args.out.join(format!(".tmp{worker}"));
                let _ = std::fs::create_dir_all(&tmp);
                loop {
                    let j = next.fetch_add(1, Ordering::Relaxed);
                    let Some(&(f, k)) = jobs.get(j) else { break };
                    let result = (|| -> Result<(), String> {
                        let name = files[f].file_stem().unwrap().to_string_lossy().to_string();
                        let original = read_wav(&files[f])?;
                        let is_control = name.starts_with("control");
                        let (audio, expected) = match k {
                            None => (original.clone(), None),
                            Some(_) if is_control => return Ok(()),
                            Some(k) => {
                                let seal = seal_for(f, k);
                                let t = Instant::now();
                                let marked = carrier::embed(&original, &seal, p)
                                    .map_err(|e| format!("{name}: {e}"))?;
                                let mut tm = timing.lock().unwrap();
                                tm.0 += t.elapsed().as_secs_f64();
                                tm.2 += original.len() as f64 / SAMPLE_RATE as f64;
                                drop(tm);
                                if k == 0 {
                                    write_wav(
                                        &args.out.join("marked").join(format!("{name}.wav")),
                                        &marked,
                                    )?;
                                    let (snr, seg) = snr(&original, &marked);
                                    quality.lock().unwrap().push((name.clone(), snr, seg));
                                }
                                (marked, Some(seal))
                            }
                        };
                        let windows_total = original.len() / p.window_samples();
                        for (cname, _, cond) in conds.iter() {
                            let processed = apply(
                                cond,
                                &audio,
                                &tmp,
                                &args.ffmpeg,
                                (f * 131 + k.unwrap_or(99)) as u64,
                            )?;
                            let t = Instant::now();
                            let hits = match carrier::detect(&processed, p) {
                                Ok(h) => h,
                                Err(carrier::CarrierError::TooShort) => Vec::new(),
                                Err(e) => return Err(e.to_string()),
                            };
                            timing.lock().unwrap().1 += t.elapsed().as_secs_f64();
                            let mut tallies = tallies.lock().unwrap();
                            let t = tallies.entry(cname).or_default();
                            match expected {
                                None => {
                                    t.unmarked_files += 1;
                                    t.false_alarms += hits.len();
                                    let score = carrier::Scan::new(&processed, p)
                                        .map(|s| max_score(&s, p))
                                        .unwrap_or(0.0);
                                    t.max_unmarked_score = t.max_unmarked_score.max(score);
                                }
                                Some(seal) => {
                                    let expected_windows = match cond {
                                        Condition::Trim(n) => (0..windows_total)
                                            .filter(|m| m * p.window_samples() >= *n)
                                            .count(),
                                        _ => windows_total,
                                    };
                                    let good = hits.iter().filter(|h| h.seal == seal).count();
                                    t.windows += expected_windows;
                                    t.recovered += good.min(expected_windows);
                                    t.wrong += hits.iter().filter(|h| h.seal != seal).count();
                                    t.files += 1;
                                    t.files_with_seal += usize::from(good > 0);
                                    t.marked_scores.extend(
                                        hits.iter()
                                            .filter(|h| h.seal == seal)
                                            .map(|h| h.sync_score),
                                    );
                                }
                            }
                        }
                        Ok(())
                    })();
                    if let Err(e) = result {
                        errors.lock().unwrap().push(e);
                    }
                }
                let _ = std::fs::remove_dir_all(&tmp);
            });
        }
    });

    let errors = errors.into_inner().unwrap();
    if !errors.is_empty() {
        return Err(errors.join("\n"));
    }
    let tallies = tallies.into_inner().unwrap();
    let quality = quality.into_inner().unwrap();
    let timing = timing.into_inner().unwrap();
    report(
        args,
        &conds,
        &tallies,
        &quality,
        timing,
        started.elapsed().as_secs_f64(),
    )
}

fn max_score(scan: &carrier::Scan, p: &Params) -> f32 {
    let mut best = f32::NEG_INFINITY;
    for s in 0..4 {
        let shift = s * carrier::HOP / 4;
        for c in 0..=scan.columns(shift).saturating_sub(p.columns) {
            best = best.max(scan.sync_score(shift, c));
        }
    }
    best
}

/// Global SNR and segmental SNR (20 ms frames, clamped to [-10, 35] dB, speech frames only).
fn snr(original: &[f32], marked: &[f32]) -> (f32, f32) {
    let sig: f64 = original.iter().map(|&x| f64::from(x * x)).sum();
    let err: f64 = original
        .iter()
        .zip(marked)
        .map(|(&a, &b)| f64::from((a - b) * (a - b)))
        .sum();
    let global = 10.0 * (sig / err.max(1e-30)).log10();
    let frame = 320;
    let peak = original
        .chunks(frame)
        .map(|c| c.iter().map(|x| x * x).sum::<f32>())
        .fold(0f32, f32::max);
    let mut segs = Vec::new();
    for (a, b) in original.chunks(frame).zip(marked.chunks(frame)) {
        let s: f32 = a.iter().map(|x| x * x).sum();
        if s < peak * 1e-4 {
            continue;
        }
        let e: f32 = a.iter().zip(b).map(|(x, y)| (x - y) * (x - y)).sum();
        segs.push((10.0 * (s / e.max(1e-20)).log10()).clamp(-10.0, 35.0));
    }
    let seg = segs.iter().sum::<f32>() / segs.len().max(1) as f32;
    (global as f32, seg)
}

fn report(
    args: &Args,
    conds: &[(&str, &str, Condition)],
    tallies: &BTreeMap<&str, Tally>,
    quality: &[(String, f32, f32)],
    timing: (f64, f64, f64),
    wall: f64,
) -> Result<(), String> {
    let p = &args.params;
    let mut md = String::new();
    let _ = writeln!(md, "| Condition | What | Windows recovered | Files with the seal | Wrong seals | False alarms (unmarked) | Max unmarked sync score |");
    let _ = writeln!(md, "|---|---|---:|---:|---:|---:|---:|");
    let mut json_rows = Vec::new();
    for (name, what, _) in conds {
        let t = tallies.get(name).cloned().unwrap_or_default();
        let rate = 100.0 * t.recovered as f64 / t.windows.max(1) as f64;
        let _ = writeln!(
            md,
            "| `{name}` | {what} | {rate:.1}% ({}/{}) | {}/{} | {} | {} | {:.2} |",
            t.recovered,
            t.windows,
            t.files_with_seal,
            t.files,
            t.wrong,
            t.false_alarms,
            t.max_unmarked_score
        );
        json_rows.push(format!(
            "    {{\"condition\": \"{name}\", \"windows\": {}, \"recovered\": {}, \"files\": {}, \"files_with_seal\": {}, \"wrong_seals\": {}, \"false_alarms\": {}, \"unmarked_files\": {}, \"max_unmarked_sync_score\": {:.3}}}",
            t.windows, t.recovered, t.files, t.files_with_seal, t.wrong, t.false_alarms, t.unmarked_files, t.max_unmarked_score
        ));
    }
    let (snr_mean, seg_mean) = (
        quality.iter().map(|q| q.1).sum::<f32>() / quality.len().max(1) as f32,
        quality.iter().map(|q| q.2).sum::<f32>() / quality.len().max(1) as f32,
    );
    let embed_rt = timing.2 / timing.0.max(1e-9);
    let params = format!("{p:?}");
    let header = format!(
        "Carrier parameters: `{params}`\n\nWindow: {:.2} s, capacity {:.1} bit/s. Seals per file: {}. Mean SNR of the marked audio: {snr_mean:.1} dB (segmental {seg_mean:.1} dB). Embedding speed: {embed_rt:.0}x real time on this machine. Detection time total: {:.1} s. Wall time: {wall:.0} s.\n\n",
        p.window_samples() as f32 / SAMPLE_RATE as f32,
        p.capacity_bps(),
        args.seals,
        timing.1,
    );
    std::fs::write(args.out.join("results.md"), header + &md).map_err(|e| e.to_string())?;
    let json = format!(
        "{{\n  \"params\": \"{}\",\n  \"window_seconds\": {:.3},\n  \"seals_per_file\": {},\n  \"snr_db\": {snr_mean:.2},\n  \"segmental_snr_db\": {seg_mean:.2},\n  \"rows\": [\n{}\n  ]\n}}\n",
        params.replace('"', "'"),
        p.window_samples() as f32 / SAMPLE_RATE as f32,
        args.seals,
        json_rows.join(",\n")
    );
    std::fs::write(args.out.join("results.json"), json).map_err(|e| e.to_string())?;
    print!(
        "{}",
        std::fs::read_to_string(args.out.join("results.md")).unwrap_or_default()
    );
    Ok(())
}
