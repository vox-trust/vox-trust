//! `vox-trust`: seal and verify WAV files from the command line (file mode).
//!
//! Pre-1.0 and unaudited. File mode only survives bit-exact copies of the audio.

use std::ffi::{OsStr, OsString};
use std::fs;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::{SystemTime, UNIX_EPOCH};

use vox_trust_core::file::{self, Reason, SealParams, Signer, Trust};
use vox_trust_core::{circle, decide, to_hex, wav, ContactState, SealCheck, Verdict};
use zeroize::Zeroizing;

const USAGE: &str = "\
vox-trust: seal and verify WAV files (file mode, pre-1.0, unaudited)

USAGE
  vox-trust keygen KEYFILE
  vox-trust pubkey KEYFILE
  vox-trust seal IN.wav OUT.wav --mode circle|public --key KEYFILE
                [--chunk-seconds 1] [--counter 0] [--force]
  vox-trust verify FILE.wav [--circle-key KEYFILE] [--pin PUBLIC_KEY_OR_FILE]
                [--contact stranger|always|strict] [--json]

KEYS
  A key file holds 64 hex characters (32 bytes). For circle mode it is the shared
  secret. For public mode it is the Ed25519 private seed; `pubkey` prints the public key
  to share. `keygen` refuses to overwrite an existing file. Key files are read only if they
  are regular files of at most 4 KiB; WAV inputs must be regular files of at most 512 MiB.

SEAL OUTPUT
  `seal` refuses to overwrite an existing OUT.wav (pass --force to replace it) and never
  writes over IN.wav. The output is written to a temporary file and then moved into place;
  without --force the move cannot replace a file that appeared in the meantime (it needs
  a file system that supports hard links, otherwise use --force).

EXIT CODES (verify)
  0 verified   1 unsealed   2 warning   3 alert
  Exit code 1 is NOT an error and NOT a pass: the file has no seal, or its seal is under a
  key you did not supply, so nothing was verified. Treat only 0 as verified.
  64 usage error   65 not a readable WAV file (or too large)   66 cannot read an input
  73 cannot write output

NOTE
  Seals survive only bit-exact copies. Re-encoding (MP3, AAC, resampling, re-recording)
  makes every chunk read as altered. Only the audio format and samples are authenticated.
";

const EX_USAGE: u8 = 64;
const EX_DATAERR: u8 = 65;
const EX_NOINPUT: u8 = 66;
const EX_CANTCREAT: u8 = 73;

/// Writes to stdout and ignores a closed pipe (`vox-trust verify f | head -0`): the exit
/// code is the verdict, and `println!` would panic with status 101 instead.
fn emit(text: &str) {
    let mut out = io::stdout().lock();
    let _ = out.write_all(text.as_bytes()).and_then(|()| out.flush());
}

struct Failure(u8, String);

type Result<T> = std::result::Result<T, Failure>;

fn usage(message: impl Into<String>) -> Failure {
    Failure(EX_USAGE, message.into())
}

/// Splits arguments into positionals and `--flag value` / `--switch` options.
/// Paths stay as `OsString` so non-UTF-8 file names work.
struct Args {
    positional: Vec<OsString>,
    options: Vec<(String, Option<OsString>)>,
}

impl Args {
    fn parse(raw: &[OsString], switches: &[&str]) -> Result<Args> {
        let mut positional = Vec::new();
        let mut options = Vec::new();
        let mut it = raw.iter();
        while let Some(arg) = it.next() {
            if arg == "-h" {
                options.push(("help".to_string(), None));
                continue;
            }
            let Some(name) = arg.to_str().and_then(|a| a.strip_prefix("--")) else {
                if arg.to_string_lossy().starts_with("--") {
                    return Err(usage("option names must be valid UTF-8"));
                }
                positional.push(arg.clone());
                continue;
            };
            if name == "help" || switches.contains(&name) {
                options.push((name.to_string(), None));
            } else {
                let value = it
                    .next()
                    .ok_or_else(|| usage(format!("--{name} needs a value")))?;
                if value.to_string_lossy().starts_with("--") {
                    return Err(usage(format!(
                        "--{name} needs a value, but got the option {}",
                        value.to_string_lossy()
                    )));
                }
                options.push((name.to_string(), Some(value.clone())));
            }
        }
        Ok(Args {
            positional,
            options,
        })
    }

    fn get_os(&self, name: &str) -> Option<&OsStr> {
        self.options
            .iter()
            .find(|(n, _)| n == name)
            .and_then(|(_, v)| v.as_deref())
    }

    fn get(&self, name: &str) -> Result<Option<&str>> {
        match self.get_os(name) {
            None => Ok(None),
            Some(v) => v
                .to_str()
                .map(Some)
                .ok_or_else(|| usage(format!("--{name} must be valid UTF-8"))),
        }
    }

    fn has(&self, name: &str) -> bool {
        self.options.iter().any(|(n, _)| n == name)
    }

    fn check_known(&self, allowed: &[&str]) -> Result<()> {
        for (name, _) in &self.options {
            if name != "help" && !allowed.contains(&name.as_str()) {
                return Err(usage(format!("unknown option --{name}")));
            }
        }
        Ok(())
    }

    fn expect_positional(&self, n: usize, what: &str) -> Result<()> {
        if self.positional.len() == n {
            Ok(())
        } else {
            Err(usage(format!("expected {what}")))
        }
    }
}

fn parse_hex32(text: &str) -> Option<Zeroizing<[u8; 32]>> {
    let text = text.trim();
    if text.len() != 64 || !text.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    let mut out = Zeroizing::new([0u8; 32]);
    for (i, byte) in out.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&text[2 * i..2 * i + 2], 16).ok()?;
    }
    Some(out)
}

const KEY_FILE_CAP: u64 = 4 * 1024;
const WAV_CAP: u64 = 512 * 1024 * 1024;

/// Reads a regular file, refusing anything else (devices, FIFOs, directories) and anything
/// larger than `cap` bytes. The read itself is bounded, so a file that grows is still capped.
fn read_bounded(path: &Path, cap: u64, what: &str) -> Result<Vec<u8>> {
    let unreadable = |e: std::io::Error| {
        Failure(
            EX_NOINPUT,
            format!("cannot read {what} {}: {e}", path.display()),
        )
    };
    let meta = fs::metadata(path).map_err(unreadable)?;
    if !meta.is_file() {
        return Err(Failure(
            EX_NOINPUT,
            format!("{what} {} is not a regular file", path.display()),
        ));
    }
    let too_big = || {
        Failure(
            EX_DATAERR,
            format!(
                "{what} {} is larger than the {cap}-byte limit",
                path.display()
            ),
        )
    };
    if meta.len() > cap {
        return Err(too_big());
    }
    let mut out = Vec::with_capacity(usize::try_from(meta.len()).unwrap_or(0) + 1);
    fs::File::open(path)
        .map_err(unreadable)?
        .take(cap + 1)
        .read_to_end(&mut out)
        .map_err(unreadable)?;
    if out.len() as u64 > cap {
        return Err(too_big());
    }
    Ok(out)
}

fn read_key_file(path: &Path) -> Result<Zeroizing<[u8; 32]>> {
    let bytes = Zeroizing::new(read_bounded(path, KEY_FILE_CAP, "key file")?);
    let text = std::str::from_utf8(&bytes).ok();
    text.and_then(parse_hex32).ok_or_else(|| {
        Failure(
            EX_DATAERR,
            format!("{} must contain exactly 64 hex characters", path.display()),
        )
    })
}

fn read_file(path: &Path) -> Result<Vec<u8>> {
    read_bounded(path, WAV_CAP, "file")
}

fn same_file(a: &Path, b: &Path) -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        if let (Ok(x), Ok(y)) = (fs::metadata(a), fs::metadata(b)) {
            return x.dev() == y.dev() && x.ino() == y.ino();
        }
    }
    matches!((fs::canonicalize(a), fs::canonicalize(b)), (Ok(x), Ok(y)) if x == y)
}

/// Writes `data` to a sibling temporary file, syncs it, then puts it at `output`.
///
/// With `force` the temporary file is renamed over `output` (keeping the permissions of the
/// file it replaces). Without `force` it is hard-linked to `output`, which fails with
/// `AlreadyExists` instead of replacing a file created after the caller's check, and then
/// the temporary name is removed. A file system without hard links gives an error.
fn write_atomic(output: &Path, data: &[u8], force: bool) -> io::Result<()> {
    let dir = match output.parent() {
        Some(p) if !p.as_os_str().is_empty() => p.to_path_buf(),
        _ => PathBuf::from("."),
    };
    let mut name = OsString::from(".vox-trust-");
    name.push(output.file_name().unwrap_or_else(|| OsStr::new("out")));
    name.push(format!(".{}.tmp", std::process::id()));
    let tmp = dir.join(name);
    let result = (|| {
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&tmp)?;
        file.write_all(data)?;
        if force {
            if let Ok(meta) = fs::metadata(output) {
                if meta.is_file() {
                    fs::set_permissions(&tmp, meta.permissions())?;
                }
            }
        }
        file.sync_all()?;
        drop(file);
        if force {
            fs::rename(&tmp, output)
        } else {
            fs::hard_link(&tmp, output)
        }
    })();
    if !force || result.is_err() {
        let _ = fs::remove_file(&tmp);
    }
    result
}

fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

fn cmd_keygen(args: &Args) -> Result<()> {
    args.check_known(&[])?;
    args.expect_positional(1, "keygen KEYFILE")?;
    let path = Path::new(&args.positional[0]);
    let mut key = Zeroizing::new([0u8; 32]);
    getrandom::fill(&mut *key).map_err(|e| Failure(EX_CANTCREAT, format!("no randomness: {e}")))?;

    let mut options = fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(path).map_err(|e| {
        Failure(
            EX_CANTCREAT,
            format!("cannot create {}: {e}", path.display()),
        )
    })?;
    let hex = Zeroizing::new(to_hex(&*key));
    if let Err(e) = writeln!(file, "{}", &*hex).and_then(|()| file.sync_all()) {
        drop(file);
        let _ = fs::remove_file(path);
        return Err(Failure(
            EX_CANTCREAT,
            format!("cannot write {}: {e}", path.display()),
        ));
    }
    drop(file);
    emit(&format!(
        "wrote a new 32-byte secret to {} (keep it private)\n\
         circle key id: {}\n\
         if you use it as a public-mode seed, the public key is: {}\n",
        path.display(),
        to_hex(&circle::key_id(&key).to_be_bytes()),
        to_hex(&file::public_key(&key))
    ));
    Ok(())
}

fn cmd_pubkey(args: &Args) -> Result<()> {
    args.check_known(&[])?;
    args.expect_positional(1, "pubkey KEYFILE")?;
    let seed = read_key_file(Path::new(&args.positional[0]))?;
    let public = file::public_key(&seed);
    emit(&format!(
        "public key: {}\nkey id:     {}\n",
        to_hex(&public),
        to_hex(&file::public_key_id(&public).to_be_bytes())
    ));
    Ok(())
}

fn cmd_seal(args: &Args) -> Result<()> {
    args.check_known(&["mode", "key", "chunk-seconds", "counter", "force"])?;
    args.expect_positional(2, "seal IN.wav OUT.wav")?;
    let (input, output) = (
        Path::new(&args.positional[0]),
        Path::new(&args.positional[1]),
    );
    let mode = args
        .get("mode")?
        .ok_or_else(|| usage("--mode is required"))?;
    let key_path = args
        .get_os("key")
        .ok_or_else(|| usage("--key is required"))?;
    let seconds: f64 = match args.get("chunk-seconds")? {
        None => 1.0,
        Some(s) => s
            .parse()
            .ok()
            .filter(|v: &f64| v.is_finite() && *v > 0.0)
            .ok_or_else(|| usage("--chunk-seconds must be a positive number"))?,
    };
    let counter: u32 = match args.get("counter")? {
        None => 0,
        Some(s) => s
            .parse()
            .map_err(|_| usage("--counter must be a non-negative integer"))?,
    };
    if !matches!(mode, "circle" | "public") {
        return Err(usage("--mode must be circle or public"));
    }
    if same_file(input, output) {
        return Err(usage(format!(
            "output {} is the input file; choose a different output name",
            output.display()
        )));
    }
    let force = args.has("force");
    if !force && fs::symlink_metadata(output).is_ok() {
        return Err(Failure(
            EX_CANTCREAT,
            format!(
                "{} already exists; pass --force to replace it",
                output.display()
            ),
        ));
    }
    let key = read_key_file(Path::new(key_path))?;
    let bytes = read_file(input)?;
    let rate = wav::parse(&bytes)
        .map_err(|e| Failure(EX_DATAERR, format!("{}: {e}", input.display())))?
        .sample_rate;
    let chunk_frames = (seconds * f64::from(rate))
        .round()
        .clamp(1.0, f64::from(u32::MAX)) as u32;

    let signer = match mode {
        "circle" => Signer::Circle {
            key: &key,
            key_id: circle::key_id(&key),
        },
        "public" => Signer::Public { seed: &key },
        _ => return Err(usage("--mode must be circle or public")),
    };
    let sealed = file::seal_wav(
        &bytes,
        signer,
        SealParams {
            created_unix: now_unix(),
            counter,
            chunk_frames,
        },
    )
    .map_err(|e| Failure(EX_DATAERR, format!("{}: {e}", input.display())))?;
    write_atomic(output, &sealed, force).map_err(|e| {
        let why = if e.kind() == io::ErrorKind::AlreadyExists {
            "it already exists; pass --force to replace it".to_string()
        } else {
            e.to_string()
        };
        Failure(
            EX_CANTCREAT,
            format!("cannot write {}: {why}", output.display()),
        )
    })?;
    emit(&format!(
        "sealed {} -> {} ({mode} mode, chunks of {chunk_frames} frames, {} bytes added)\n",
        input.display(),
        output.display(),
        sealed.len().saturating_sub(bytes.len())
    ));
    Ok(())
}

/// Human wording of a check: the stable name with spaces (`unknown_key` -> `unknown key`).
fn check_label(check: SealCheck) -> String {
    check.as_str().replace('_', " ")
}

fn describe(report: &file::Report, verdict: Verdict) -> String {
    let rate = f64::from(report.sample_rate.max(1));
    let mut out = String::new();
    out.push_str(&format!("verdict: {verdict}\n"));
    out.push_str(&format!(
        "seal:    {} ({})\n",
        check_label(report.check),
        report.reason
    ));
    // Until the authenticator verifies, everything read from the manifest is only what the
    // file claims: a forger can write any mode or key id.
    if let (Some(mode), Some(key_id)) = (report.mode, report.key_id) {
        let id = to_hex(&key_id.to_be_bytes());
        if report.authenticator_valid {
            out.push_str(&format!("mode:    {mode}   key id: {id}\n"));
        } else {
            out.push_str(&format!("declared mode: {mode}   declared key id: {id}\n"));
        }
    }
    if let Some(key) = report.embedded_public_key {
        let label = if report.authenticator_valid {
            "signed by public key"
        } else {
            "embedded key (unverified)"
        };
        out.push_str(&format!("{label}: {}\n", to_hex(&key)));
    }
    if report.authenticated {
        if let (Some(created), Some(counter)) = (report.created_unix, report.counter) {
            out.push_str(&format!("created: unix {created}   counter: {counter}\n"));
        }
        if let (Some(n), Some(frames)) = (report.n_chunks, report.chunk_frames) {
            out.push_str(&format!(
                "chunks:  {n} of {:.1} s\n",
                f64::from(frames) / rate
            ));
            if !report.modified_chunks.is_empty() {
                let spans: Vec<String> = report
                    .modified_chunks
                    .iter()
                    .map(|&i| {
                        let from = f64::from(i) * f64::from(frames) / rate;
                        format!("#{i} ({from:.1}-{:.1} s)", from + f64::from(frames) / rate)
                    })
                    .collect();
                out.push_str(&format!("altered: {}\n", spans.join(", ")));
            }
        }
    } else if report.check != SealCheck::Absent {
        out.push_str("(chunk results are not shown: the seal itself is not trusted)\n");
    }
    out
}

fn cmd_verify(args: &Args) -> Result<u8> {
    args.check_known(&["circle-key", "pin", "contact", "json"])?;
    args.expect_positional(1, "verify FILE.wav")?;
    let path = Path::new(&args.positional[0]);
    let circle_key = args
        .get_os("circle-key")
        .map(|p| read_key_file(Path::new(p)))
        .transpose()?;
    let pinned: Option<[u8; 32]> = match args.get_os("pin") {
        None => None,
        Some(value) => Some(match value.to_str().and_then(parse_hex32) {
            Some(key) => *key,
            None => *read_key_file(Path::new(value))?,
        }),
    };
    let contact = match args.get("contact")?.unwrap_or("stranger") {
        "stranger" => None,
        "always" => Some(ContactState {
            always_seals: true,
            strict: false,
        }),
        "strict" => Some(ContactState {
            always_seals: true,
            strict: true,
        }),
        _ => return Err(usage("--contact must be stranger, always or strict")),
    };

    let bytes = read_file(path)?;
    let trust = Trust {
        circle: circle_key.as_ref().map(|k| (circle::key_id(k), &**k)),
        pinned_public: pinned.as_ref(),
    };
    let report = file::verify_wav(&bytes, trust)
        .map_err(|e| Failure(EX_DATAERR, format!("{}: {e}", path.display())))?;
    let verdict = decide(report.check, contact);
    if args.has("json") {
        emit(&format!(
            "{{\"verdict\":\"{verdict}\",\"report\":{}}}\n",
            report.to_json()
        ));
    } else {
        let mut text = describe(&report, verdict);
        if report.check == SealCheck::UnknownKey && report.reason == Reason::UntrustedKey {
            text.push_str("hint:    pass the key you trust with --circle-key or --pin\n");
        }
        emit(&text);
    }
    Ok(verdict.code() as u8)
}

fn run(raw: &[OsString]) -> Result<u8> {
    let Some((command, rest)) = raw.split_first() else {
        return Err(usage("no command given"));
    };
    let command = command.to_string_lossy();
    let known = matches!(command.as_ref(), "keygen" | "pubkey" | "seal" | "verify");
    let switches: &[&str] = match command.as_ref() {
        "verify" => &["json"],
        "seal" => &["force"],
        _ => &[],
    };
    if known {
        let args = Args::parse(rest, switches)?;
        if args.has("help") {
            emit(USAGE);
            return Ok(0);
        }
        return match command.as_ref() {
            "keygen" => cmd_keygen(&args).map(|()| 0),
            "pubkey" => cmd_pubkey(&args).map(|()| 0),
            "seal" => cmd_seal(&args).map(|()| 0),
            _ => cmd_verify(&args),
        };
    }
    match command.as_ref() {
        "-h" | "--help" | "help" => {
            emit(USAGE);
            Ok(0)
        }
        other => Err(usage(format!("unknown command {other}"))),
    }
}

fn main() -> ExitCode {
    let args: Vec<OsString> = std::env::args_os().skip(1).collect();
    match run(&args) {
        Ok(code) => ExitCode::from(code),
        Err(Failure(code, message)) => {
            eprintln!("vox-trust: {message}");
            if code == EX_USAGE {
                eprintln!("run `vox-trust help` for usage");
            }
            ExitCode::from(code)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("vox-trust-unit-{}-{tag}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn leftovers(dir: &Path) -> usize {
        fs::read_dir(dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_name().to_string_lossy().ends_with(".tmp"))
            .count()
    }

    #[test]
    fn without_force_a_file_that_appeared_is_never_replaced() {
        let dir = scratch("noforce");
        let out = dir.join("out.wav");
        // The file the caller's pre-check did not see.
        fs::write(&out, b"someone else's file").unwrap();
        let err = write_atomic(&out, b"sealed", false).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::AlreadyExists);
        assert_eq!(fs::read(&out).unwrap(), b"someone else's file");
        assert_eq!(leftovers(&dir), 0);

        let fresh = dir.join("fresh.wav");
        write_atomic(&fresh, b"sealed", false).unwrap();
        assert_eq!(fs::read(&fresh).unwrap(), b"sealed");
        assert_eq!(leftovers(&dir), 0, "the temporary name is removed");
        fs::remove_dir_all(&dir).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn force_replaces_and_keeps_the_existing_mode() {
        use std::os::unix::fs::PermissionsExt;
        let dir = scratch("force");
        let out = dir.join("out.wav");
        fs::write(&out, b"old").unwrap();
        fs::set_permissions(&out, fs::Permissions::from_mode(0o640)).unwrap();
        write_atomic(&out, b"new", true).unwrap();
        assert_eq!(fs::read(&out).unwrap(), b"new");
        assert_eq!(
            fs::metadata(&out).unwrap().permissions().mode() & 0o777,
            0o640
        );
        assert_eq!(leftovers(&dir), 0);
        fs::remove_dir_all(&dir).unwrap();
    }
}
