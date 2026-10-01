//! `vox-trust`: seal and verify WAV files from the command line (file mode).
//!
//! Pre-1.0 and unaudited. File mode only survives bit-exact copies of the audio.

use std::fs;
use std::io::Write;
use std::process::ExitCode;
use std::time::{SystemTime, UNIX_EPOCH};

use vox_trust_core::file::{self, Reason, SealParams, Signer, Trust};
use vox_trust_core::{circle, decide, to_hex, wav, ContactState, SealCheck, Verdict};

const USAGE: &str = "\
vox-trust: seal and verify WAV files (file mode, pre-1.0, unaudited)

USAGE
  vox-trust keygen KEYFILE
  vox-trust pubkey KEYFILE
  vox-trust seal IN.wav OUT.wav --mode circle|public --key KEYFILE
                [--chunk-seconds 1] [--counter 0]
  vox-trust verify FILE.wav [--circle-key KEYFILE] [--pin PUBLIC_KEY_OR_FILE]
                [--contact stranger|always|strict] [--json]

KEYS
  A key file holds 64 hex characters (32 bytes). For circle mode it is the shared
  secret. For public mode it is the Ed25519 private seed; `pubkey` prints the public key
  to share. `keygen` refuses to overwrite an existing file.

EXIT CODES (verify)
  0 verified   1 unsealed   2 warning   3 alert
  64 usage error   65 not a readable WAV file   66 cannot read an input   73 cannot write output

NOTE
  Seals survive only bit-exact copies. Re-encoding (MP3, AAC, resampling, re-recording)
  makes every chunk read as altered. Only the audio format and samples are authenticated.
";

const EX_USAGE: u8 = 64;
const EX_DATAERR: u8 = 65;
const EX_NOINPUT: u8 = 66;
const EX_CANTCREAT: u8 = 73;

struct Failure(u8, String);

type Result<T> = std::result::Result<T, Failure>;

fn usage(message: impl Into<String>) -> Failure {
    Failure(EX_USAGE, message.into())
}

/// Splits arguments into positionals and `--flag value` / `--switch` options.
struct Args {
    positional: Vec<String>,
    options: Vec<(String, Option<String>)>,
}

impl Args {
    fn parse(raw: &[String], switches: &[&str]) -> Result<Args> {
        let mut positional = Vec::new();
        let mut options = Vec::new();
        let mut it = raw.iter();
        while let Some(arg) = it.next() {
            if let Some(name) = arg.strip_prefix("--") {
                if switches.contains(&name) {
                    options.push((name.to_string(), None));
                } else {
                    let value = it
                        .next()
                        .ok_or_else(|| usage(format!("--{name} needs a value")))?;
                    options.push((name.to_string(), Some(value.clone())));
                }
            } else {
                positional.push(arg.clone());
            }
        }
        Ok(Args {
            positional,
            options,
        })
    }

    fn get(&self, name: &str) -> Option<&str> {
        self.options
            .iter()
            .find(|(n, _)| n == name)
            .and_then(|(_, v)| v.as_deref())
    }

    fn has(&self, name: &str) -> bool {
        self.options.iter().any(|(n, _)| n == name)
    }

    fn check_known(&self, allowed: &[&str]) -> Result<()> {
        for (name, _) in &self.options {
            if !allowed.contains(&name.as_str()) {
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

fn parse_hex32(text: &str) -> Option<[u8; 32]> {
    let text = text.trim();
    if text.len() != 64 || !text.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    let mut out = [0u8; 32];
    for (i, byte) in out.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&text[2 * i..2 * i + 2], 16).ok()?;
    }
    Some(out)
}

fn read_key_file(path: &str) -> Result<[u8; 32]> {
    let text = fs::read_to_string(path)
        .map_err(|e| Failure(EX_NOINPUT, format!("cannot read key file {path}: {e}")))?;
    parse_hex32(&text).ok_or_else(|| {
        Failure(
            EX_DATAERR,
            format!("{path} must contain exactly 64 hex characters"),
        )
    })
}

fn read_file(path: &str) -> Result<Vec<u8>> {
    fs::read(path).map_err(|e| Failure(EX_NOINPUT, format!("cannot read {path}: {e}")))
}

fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

fn cmd_keygen(args: &Args) -> Result<()> {
    args.check_known(&[])?;
    args.expect_positional(1, "keygen KEYFILE")?;
    let path = &args.positional[0];
    let mut key = [0u8; 32];
    getrandom::fill(&mut key).map_err(|e| Failure(EX_CANTCREAT, format!("no randomness: {e}")))?;

    let mut options = fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options
        .open(path)
        .map_err(|e| Failure(EX_CANTCREAT, format!("cannot create {path}: {e}")))?;
    writeln!(file, "{}", to_hex(&key))
        .map_err(|e| Failure(EX_CANTCREAT, format!("cannot write {path}: {e}")))?;
    println!("wrote a new 32-byte secret to {path} (keep it private)");
    println!(
        "circle key id: {}",
        to_hex(&circle::key_id(&key).to_be_bytes())
    );
    println!(
        "if you use it as a public-mode seed, the public key is: {}",
        to_hex(&file::public_key(&key))
    );
    Ok(())
}

fn cmd_pubkey(args: &Args) -> Result<()> {
    args.check_known(&[])?;
    args.expect_positional(1, "pubkey KEYFILE")?;
    let seed = read_key_file(&args.positional[0])?;
    let public = file::public_key(&seed);
    println!("public key: {}", to_hex(&public));
    println!(
        "key id:     {}",
        to_hex(&file::public_key_id(&public).to_be_bytes())
    );
    Ok(())
}

fn cmd_seal(args: &Args) -> Result<()> {
    args.check_known(&["mode", "key", "chunk-seconds", "counter"])?;
    args.expect_positional(2, "seal IN.wav OUT.wav")?;
    let (input, output) = (&args.positional[0], &args.positional[1]);
    let mode = args
        .get("mode")
        .ok_or_else(|| usage("--mode is required"))?;
    let key_path = args.get("key").ok_or_else(|| usage("--key is required"))?;
    let seconds: f64 = match args.get("chunk-seconds") {
        None => 1.0,
        Some(s) => s
            .parse()
            .ok()
            .filter(|v: &f64| v.is_finite() && *v > 0.0)
            .ok_or_else(|| usage("--chunk-seconds must be a positive number"))?,
    };
    let counter: u32 = match args.get("counter") {
        None => 0,
        Some(s) => s
            .parse()
            .map_err(|_| usage("--counter must be a non-negative integer"))?,
    };
    let key = read_key_file(key_path)?;
    let bytes = read_file(input)?;
    let rate = wav::parse(&bytes)
        .map_err(|e| Failure(EX_DATAERR, format!("{input}: {e}")))?
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
    .map_err(|e| Failure(EX_DATAERR, format!("{input}: {e}")))?;
    fs::write(output, &sealed)
        .map_err(|e| Failure(EX_CANTCREAT, format!("cannot write {output}: {e}")))?;
    println!(
        "sealed {input} -> {output} ({mode} mode, chunks of {chunk_frames} frames, {} bytes added)",
        sealed.len().saturating_sub(bytes.len())
    );
    Ok(())
}

fn verdict_name(v: Verdict) -> &'static str {
    match v {
        Verdict::Verified => "verified",
        Verdict::Unsealed => "unsealed",
        Verdict::Warning => "warning",
        Verdict::Alert => "alert",
    }
}

fn describe(report: &file::Report, verdict: Verdict) -> String {
    let rate = f64::from(report.sample_rate.max(1));
    let mut out = String::new();
    out.push_str(&format!("verdict: {}\n", verdict_name(verdict)));
    out.push_str(&format!(
        "seal:    {} ({})\n",
        match report.check {
            SealCheck::Valid => "valid",
            SealCheck::Invalid => "invalid",
            SealCheck::UnknownKey => "unknown key",
            SealCheck::Absent => "absent",
        },
        report.reason.as_str()
    ));
    if let (Some(mode), Some(key_id)) = (report.mode, report.key_id) {
        out.push_str(&format!(
            "mode:    {}   key id: {}\n",
            match mode {
                vox_trust_core::Mode::Circle => "circle",
                vox_trust_core::Mode::Public => "public",
            },
            to_hex(&key_id.to_be_bytes())
        ));
    }
    if let Some(key) = report.embedded_public_key {
        out.push_str(&format!("signed by public key: {}\n", to_hex(&key)));
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
    let path = &args.positional[0];
    let circle_key = args.get("circle-key").map(read_key_file).transpose()?;
    let pinned: Option<[u8; 32]> = match args.get("pin") {
        None => None,
        Some(value) => Some(match parse_hex32(value) {
            Some(key) => key,
            None => read_key_file(value)?,
        }),
    };
    let contact = match args.get("contact").unwrap_or("stranger") {
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
        circle: circle_key.as_ref().map(|k| (circle::key_id(k), k)),
        pinned_public: pinned.as_ref(),
    };
    let report =
        file::verify_wav(&bytes, trust).map_err(|e| Failure(EX_DATAERR, format!("{path}: {e}")))?;
    let verdict = decide(report.check, contact);
    if args.has("json") {
        println!(
            "{{\"verdict\":\"{}\",\"report\":{}}}",
            verdict_name(verdict),
            report.to_json()
        );
    } else {
        print!("{}", describe(&report, verdict));
        if report.check == SealCheck::UnknownKey && report.reason == Reason::UntrustedKey {
            println!("hint:    pass the key you trust with --circle-key or --pin");
        }
    }
    Ok(verdict.code() as u8)
}

fn run(raw: &[String]) -> Result<u8> {
    let Some((command, rest)) = raw.split_first() else {
        return Err(usage("no command given"));
    };
    match command.as_str() {
        "-h" | "--help" | "help" => {
            print!("{USAGE}");
            Ok(0)
        }
        "keygen" => cmd_keygen(&Args::parse(rest, &[])?).map(|()| 0),
        "pubkey" => cmd_pubkey(&Args::parse(rest, &[])?).map(|()| 0),
        "seal" => cmd_seal(&Args::parse(rest, &[])?).map(|()| 0),
        "verify" => cmd_verify(&Args::parse(rest, &["json"])?),
        other => Err(usage(format!("unknown command {other}"))),
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
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
