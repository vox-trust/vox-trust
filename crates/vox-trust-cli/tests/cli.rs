//! Runs the real `vox-trust` binary.

use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU32, Ordering};

use vox_trust_core::wav;

static COUNTER: AtomicU32 = AtomicU32::new(0);

struct Dir(PathBuf);

impl Dir {
    fn new() -> Dir {
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let path = std::env::temp_dir().join(format!("vox-trust-cli-{}-{n}", std::process::id()));
        fs::create_dir_all(&path).unwrap();
        Dir(path)
    }

    fn path(&self, name: &str) -> String {
        self.0.join(name).to_string_lossy().into_owned()
    }
}

impl Drop for Dir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn run(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_vox-trust"))
        .args(args)
        .env_remove("VOX_TRUST_PASSPHRASE")
        .output()
        .unwrap()
}

fn run_with_passphrase(args: &[&str], passphrase: &str) -> Output {
    Command::new(env!("CARGO_BIN_EXE_vox-trust"))
        .args(args)
        .env("VOX_TRUST_PASSPHRASE", passphrase)
        .output()
        .unwrap()
}

fn code(out: &Output) -> i32 {
    out.status.code().unwrap()
}

fn text(out: &Output) -> String {
    String::from_utf8_lossy(&out.stdout).into_owned()
}

fn err(out: &Output) -> String {
    String::from_utf8_lossy(&out.stderr).into_owned()
}

fn write_clip(path: &str) {
    let pcm: Vec<u8> = (0..16000i32)
        .flat_map(|i| (((i * 37) % 20000 - 10000) as i16).to_le_bytes())
        .collect();
    fs::write(path, pcm16_wav(1, 8000, &pcm)).unwrap();
}

/// A minimal canonical PCM16 WAV, built here so the test does not depend on core's encoder.
fn pcm16_wav(channels: u16, rate: u32, pcm: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&(36 + pcm.len() as u32).to_le_bytes());
    out.extend_from_slice(b"WAVEfmt ");
    out.extend_from_slice(&16u32.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&channels.to_le_bytes());
    out.extend_from_slice(&rate.to_le_bytes());
    out.extend_from_slice(&(rate * u32::from(channels) * 2).to_le_bytes());
    out.extend_from_slice(&(channels * 2).to_le_bytes());
    out.extend_from_slice(&16u16.to_le_bytes());
    out.extend_from_slice(b"data");
    out.extend_from_slice(&(pcm.len() as u32).to_le_bytes());
    out.extend_from_slice(pcm);
    out
}

/// A plain key (fast; most tests are not about key protection).
fn keygen(dir: &Dir, name: &str) -> String {
    let path = dir.path(name);
    let out = run(&["keygen", &path, "--plain"]);
    assert_eq!(code(&out), 0, "{}", err(&out));
    path
}

#[test]
fn keygen_writes_64_hex_and_refuses_to_overwrite() {
    let dir = Dir::new();
    let key = keygen(&dir, "k.key");
    let content = fs::read_to_string(&key).unwrap();
    assert_eq!(content.trim().len(), 64);
    assert!(content.trim().bytes().all(|b| b.is_ascii_hexdigit()));
    let again = run(&["keygen", &key, "--plain"]);
    assert_eq!(code(&again), 73);
    assert_eq!(
        fs::read_to_string(&key).unwrap(),
        content,
        "must not overwrite"
    );
}

#[cfg(unix)]
#[test]
fn keygen_creates_a_private_file() {
    use std::os::unix::fs::PermissionsExt;
    let dir = Dir::new();
    let key = keygen(&dir, "k.key");
    let mode = fs::metadata(&key).unwrap().permissions().mode() & 0o777;
    assert_eq!(mode, 0o600);
}

#[test]
fn circle_round_trip_then_tamper() {
    let dir = Dir::new();
    let (clip, sealed) = (dir.path("clip.wav"), dir.path("sealed.wav"));
    write_clip(&clip);
    let key = keygen(&dir, "circle.key");

    let out = run(&[
        "seal",
        &clip,
        &sealed,
        "--mode",
        "circle",
        "--key",
        &key,
        "--chunk-seconds",
        "0.5",
    ]);
    assert_eq!(code(&out), 0, "{}", err(&out));

    let ok = run(&["verify", &sealed, "--circle-key", &key]);
    assert_eq!(code(&ok), 0, "{}", text(&ok));
    assert!(text(&ok).contains("verdict: verified"));
    assert!(text(&ok).contains("chunks:  4 of 0.5 s"));

    // Flip one bit in the audio: chunk 2 (1.0 to 1.5 s) is localised.
    let mut bytes = fs::read(&sealed).unwrap();
    let offset = wav::parse(&bytes).unwrap().pcm.as_ptr() as usize - bytes.as_ptr() as usize;
    bytes[offset + 2 * 4000 * 2 + 10] ^= 1;
    let tampered = dir.path("tampered.wav");
    fs::write(&tampered, bytes).unwrap();
    let bad = run(&["verify", &tampered, "--circle-key", &key]);
    assert_eq!(code(&bad), 3);
    assert!(
        text(&bad).contains("altered: #2 (1.0-1.5 s)"),
        "{}",
        text(&bad)
    );

    let json = run(&["verify", &tampered, "--circle-key", &key, "--json"]);
    assert_eq!(code(&json), 3);
    assert!(text(&json).contains("\"modified_chunks\":[2]"));
}

#[test]
fn no_key_means_untrusted_and_a_different_key_fails() {
    let dir = Dir::new();
    let (clip, sealed) = (dir.path("clip.wav"), dir.path("sealed.wav"));
    write_clip(&clip);
    let key = keygen(&dir, "a.key");
    let other = keygen(&dir, "b.key");
    run(&["seal", &clip, &sealed, "--mode", "circle", "--key", &key]);

    let none = run(&["verify", &sealed]);
    assert_eq!(
        code(&none),
        1,
        "unknown key reads as unsealed for a stranger"
    );
    assert!(text(&none).contains("unknown key"));
    let pinned_contact = run(&["verify", &sealed, "--contact", "always"]);
    assert_eq!(
        code(&pinned_contact),
        3,
        "a pinned contact sealing with an unknown key is an alert"
    );

    let wrong = run(&["verify", &sealed, "--circle-key", &other]);
    assert_eq!(
        code(&wrong),
        1,
        "a different key has a different id, so it is untrusted"
    );
}

#[test]
fn public_mode_needs_a_pinned_key() {
    let dir = Dir::new();
    let (clip, sealed) = (dir.path("clip.wav"), dir.path("sealed.wav"));
    write_clip(&clip);
    let seed = keygen(&dir, "seed.key");
    let attacker = keygen(&dir, "attacker.key");
    run(&["seal", &clip, &sealed, "--mode", "public", "--key", &seed]);

    let pubkey = text(&run(&["pubkey", &seed]));
    let public_hex = pubkey
        .lines()
        .next()
        .unwrap()
        .trim_start_matches("public key: ")
        .to_string();
    assert_eq!(public_hex.len(), 64);

    let pinned = run(&["verify", &sealed, "--pin", &public_hex]);
    assert_eq!(code(&pinned), 0, "{}", text(&pinned));

    let unpinned = run(&["verify", &sealed]);
    assert_eq!(code(&unpinned), 1);
    assert!(text(&unpinned).contains("signed by public key"));

    let attacker_public = text(&run(&["pubkey", &attacker]));
    let attacker_hex = attacker_public
        .lines()
        .next()
        .unwrap()
        .trim_start_matches("public key: ")
        .to_string();
    let wrong_pin = run(&[
        "verify",
        &sealed,
        "--pin",
        &attacker_hex,
        "--contact",
        "always",
    ]);
    assert_eq!(code(&wrong_pin), 3);
}

#[test]
fn an_unsealed_file_follows_the_contact_policy() {
    let dir = Dir::new();
    let clip = dir.path("clip.wav");
    write_clip(&clip);
    assert_eq!(code(&run(&["verify", &clip])), 1);
    assert_eq!(code(&run(&["verify", &clip, "--contact", "always"])), 2);
    assert_eq!(code(&run(&["verify", &clip, "--contact", "strict"])), 3);
}

#[test]
fn errors_use_distinct_exit_codes() {
    let dir = Dir::new();
    assert_eq!(code(&run(&[])), 64);
    assert_eq!(code(&run(&["frobnicate"])), 64);
    assert_eq!(code(&run(&["verify"])), 64);
    assert_eq!(code(&run(&["verify", "x.wav", "--contact", "maybe"])), 64);
    assert_eq!(code(&run(&["verify", "x.wav", "--nope", "1"])), 64);
    assert_eq!(code(&run(&["verify", &dir.path("missing.wav")])), 66);

    let junk = dir.path("junk.wav");
    fs::write(&junk, b"definitely not a wav file").unwrap();
    assert_eq!(code(&run(&["verify", &junk])), 65);

    let (clip, out) = (dir.path("clip.wav"), dir.path("out.wav"));
    write_clip(&clip);
    let key = keygen(&dir, "k.key");
    assert_eq!(
        code(&run(&["seal", &clip, &out, "--key", &key])),
        64,
        "--mode is required"
    );
    assert_eq!(
        code(&run(&[
            "seal", &clip, &out, "--mode", "other", "--key", &key
        ])),
        64
    );
    assert_eq!(
        code(&run(&[
            "seal",
            &clip,
            &out,
            "--mode",
            "circle",
            "--key",
            &key,
            "--chunk-seconds",
            "0"
        ])),
        64
    );
    let short = dir.path("short.key");
    fs::write(&short, "abc\n").unwrap();
    assert_eq!(
        code(&run(&[
            "seal", &clip, &out, "--mode", "circle", "--key", &short
        ])),
        65
    );
    assert!(err(&run(&["verify"])).contains("expected verify FILE.wav"));
}

#[test]
fn help_prints_usage_and_exits_zero() {
    let out = run(&["help"]);
    assert_eq!(code(&out), 0);
    assert!(text(&out).contains("EXIT CODES"));
}

#[test]
fn subcommand_help_works_and_documents_exit_code_one() {
    for args in [["verify", "--help"], ["verify", "-h"], ["seal", "--help"]] {
        let out = run(&args);
        assert_eq!(code(&out), 0, "{args:?}");
        assert!(text(&out).contains("Exit code 1 is NOT an error"));
    }
}

#[test]
fn an_option_cannot_be_another_options_value() {
    let dir = Dir::new();
    let clip = dir.path("clip.wav");
    write_clip(&clip);
    let out = run(&["verify", &clip, "--circle-key", "--json"]);
    assert_eq!(code(&out), 64);
    assert!(err(&out).contains("needs a value"));
}

#[test]
fn seal_refuses_to_overwrite_or_to_write_over_its_input() {
    let dir = Dir::new();
    let (clip, out) = (dir.path("clip.wav"), dir.path("out.wav"));
    write_clip(&clip);
    let key = keygen(&dir, "k.key");
    let seal = |input: &str, output: &str, force: bool| {
        let mut args = vec!["seal", input, output, "--mode", "circle", "--key", &key];
        if force {
            args.push("--force");
        }
        run(&args)
    };
    let original = fs::read(&clip).unwrap();

    assert_eq!(code(&seal(&clip, &clip, true)), 64, "output == input");
    assert_eq!(fs::read(&clip).unwrap(), original);

    assert_eq!(code(&seal(&clip, &out, false)), 0);
    let first = fs::read(&out).unwrap();
    let refused = seal(&clip, &out, false);
    assert_eq!(code(&refused), 73);
    assert!(err(&refused).contains("--force"));
    assert_eq!(fs::read(&out).unwrap(), first);

    assert_eq!(code(&seal(&clip, &out, true)), 0, "--force replaces");
    let leftovers: Vec<_> = fs::read_dir(&dir.0)
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_name().to_string_lossy().ends_with(".tmp"))
        .collect();
    assert!(leftovers.is_empty(), "no temp files left behind");
}

#[test]
fn only_regular_files_within_the_size_cap_are_read() {
    let dir = Dir::new();
    let big_key = dir.path("big.key");
    fs::write(&big_key, vec![b'a'; 5000]).unwrap();
    let clip = dir.path("clip.wav");
    write_clip(&clip);
    let out = run(&["verify", &clip, "--circle-key", &big_key]);
    assert_eq!(code(&out), 65);
    assert!(err(&out).contains("limit"));

    // A directory is not a regular file.
    let out = run(&["verify", &dir.path("")]);
    assert_eq!(code(&out), 66);
    assert!(err(&out).contains("not a regular file"));
}

#[cfg(unix)]
#[test]
fn non_utf8_paths_do_not_panic() {
    use std::ffi::OsString;
    use std::os::unix::ffi::OsStringExt;
    let dir = Dir::new();
    let mut name = dir.0.clone().into_os_string().into_vec();
    name.extend_from_slice(b"/bad-\xff.wav");
    let path = OsString::from_vec(name);
    let out = Command::new(env!("CARGO_BIN_EXE_vox-trust"))
        .arg("verify")
        .arg(&path)
        .output()
        .unwrap();
    assert_eq!(code(&out), 66, "a missing file, not a panic");
}

#[test]
fn report_names_are_unchanged() {
    let dir = Dir::new();
    let (clip, sealed) = (dir.path("clip.wav"), dir.path("sealed.wav"));
    write_clip(&clip);
    let key = keygen(&dir, "k.key");
    run(&["seal", &clip, &sealed, "--mode", "circle", "--key", &key]);

    let ok = run(&["verify", &sealed, "--circle-key", &key, "--json"]);
    let json = text(&ok);
    assert!(
        json.starts_with("{\"verdict\":\"verified\",\"report\":{\"check\":\"valid\",\"reason\":\"none\",\"mode\":\"circle\","),
        "{json}"
    );
    let human = text(&run(&["verify", &sealed, "--circle-key", &key]));
    assert!(
        human.contains("verdict: verified\nseal:    valid (none)\nmode:    circle   key id: "),
        "{human}"
    );

    let none = run(&["verify", &sealed, "--json"]);
    assert!(text(&none).starts_with("{\"verdict\":\"unsealed\",\"report\":{\"check\":\"unknown_key\",\"reason\":\"untrusted_key\""));
    let human = text(&run(&["verify", &sealed]));
    assert!(
        human.contains("seal:    unknown key (untrusted_key)"),
        "{human}"
    );
    assert!(
        human.contains("declared mode: circle   declared key id: "),
        "{human}"
    );

    for (contact, name) in [("always", "alert"), ("strict", "alert")] {
        let out = run(&["verify", &sealed, "--contact", contact]);
        assert!(text(&out).starts_with(&format!("verdict: {name}\n")));
    }
    let unsealed = text(&run(&["verify", &clip, "--contact", "always"]));
    assert!(
        unsealed.starts_with("verdict: warning\nseal:    absent (no_manifest)\n"),
        "{unsealed}"
    );
}

#[test]
fn an_unverified_embedded_key_is_never_attributed() {
    let dir = Dir::new();
    let (clip, sealed, forged) = (
        dir.path("clip.wav"),
        dir.path("sealed.wav"),
        dir.path("forged.wav"),
    );
    write_clip(&clip);
    let seed = keygen(&dir, "seed.key");
    run(&["seal", &clip, &sealed, "--mode", "public", "--key", &seed]);

    // Genuine signature, key not pinned: the key is real but not trusted.
    let genuine = text(&run(&["verify", &sealed]));
    assert!(genuine.contains("signed by public key: "), "{genuine}");
    assert!(
        genuine.contains("\nmode:    public   key id: "),
        "{genuine}"
    );

    // Break the signature: the embedded key is only a claim.
    let mut bytes = fs::read(&sealed).unwrap();
    let manifest_end = {
        let w = wav::parse(&bytes).unwrap();
        w.manifest.unwrap().as_ptr() as usize - bytes.as_ptr() as usize + w.manifest.unwrap().len()
    };
    bytes[manifest_end - 1] ^= 1;
    fs::write(&forged, bytes).unwrap();
    let out = run(&["verify", &forged]);
    let shown = text(&out);
    assert_eq!(code(&out), 3, "{shown}");
    assert!(shown.contains("embedded key (unverified): "), "{shown}");
    assert!(!shown.contains("signed by"), "{shown}");
    assert!(
        shown.contains("declared mode: public   declared key id: "),
        "{shown}"
    );
}

#[cfg(unix)]
#[test]
fn a_closed_stdout_does_not_panic_and_keeps_the_verdict_code() {
    use std::process::Stdio;
    let dir = Dir::new();
    let clip = dir.path("clip.wav");
    write_clip(&clip);
    for _ in 0..20 {
        let mut child = Command::new(env!("CARGO_BIN_EXE_vox-trust"))
            .args(["verify", &clip, "--contact", "always"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        drop(child.stdout.take()); // the reader goes away before the tool writes
        let out = child.wait_with_output().unwrap();
        assert_eq!(
            out.status.code(),
            Some(2),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        assert!(!String::from_utf8_lossy(&out.stderr).contains("panicked"));
    }
}

#[cfg(unix)]
#[test]
fn seal_with_force_keeps_the_mode_of_the_replaced_file() {
    use std::os::unix::fs::PermissionsExt;
    let dir = Dir::new();
    let (clip, out) = (dir.path("clip.wav"), dir.path("out.wav"));
    write_clip(&clip);
    let key = keygen(&dir, "k.key");
    fs::write(&out, b"old").unwrap();
    fs::set_permissions(&out, fs::Permissions::from_mode(0o640)).unwrap();
    let sealed = run(&[
        "seal", &clip, &out, "--mode", "circle", "--key", &key, "--force",
    ]);
    assert_eq!(code(&sealed), 0, "{}", err(&sealed));
    assert_eq!(
        fs::metadata(&out).unwrap().permissions().mode() & 0o777,
        0o640
    );
}

#[test]
fn protected_key_round_trip() {
    let dir = Dir::new();
    let key = dir.path("p.key");
    let plain = dir.path("plain.key");
    let out = run_with_passphrase(&["keygen", &key], "correct horse battery");
    assert_eq!(code(&out), 0, "{}", err(&out));
    let content = fs::read_to_string(&key).unwrap();
    assert!(
        content.starts_with("vox-trust-key:1:argon2id:m=65536,t=3,p=4:"),
        "{content}"
    );

    // The same seal and verify flow works with the protected key.
    let (clip, sealed) = (dir.path("a.wav"), dir.path("s.wav"));
    write_clip(&clip);
    let out = run_with_passphrase(
        &["seal", &clip, &sealed, "--mode", "circle", "--key", &key],
        "correct horse battery",
    );
    assert_eq!(code(&out), 0, "{}", err(&out));
    let out = run_with_passphrase(
        &["verify", &sealed, "--circle-key", &key],
        "correct horse battery",
    );
    assert_eq!(code(&out), 0, "{}{}", text(&out), err(&out));

    // pubkey from the protected key equals pubkey from the same secret in plain form.
    let out = run_with_passphrase(&["pubkey", &key], "correct horse battery");
    assert_eq!(code(&out), 0, "{}", err(&out));
    let public = text(&out);
    let other = run(&["keygen", &plain, "--plain"]);
    assert_eq!(code(&other), 0);
    assert_ne!(text(&run(&["pubkey", &plain])), public);
}

#[test]
fn wrong_or_missing_passphrase() {
    let dir = Dir::new();
    let key = dir.path("p.key");
    assert_eq!(
        code(&run_with_passphrase(&["keygen", &key], "right one")),
        0
    );
    let out = run_with_passphrase(&["pubkey", &key], "wrong one");
    assert_eq!(code(&out), 77, "{}", err(&out));
    assert!(err(&out).contains("wrong passphrase"), "{}", err(&out));
    let out = run_with_passphrase(&["pubkey", &key], "");
    assert_eq!(code(&out), 64);
    assert!(err(&out).contains("empty"), "{}", err(&out));
}

#[test]
fn passphrase_file_wins_over_the_environment() {
    let dir = Dir::new();
    let (key, pass) = (dir.path("p.key"), dir.path("pass.txt"));
    fs::write(&pass, "from the file\nsecond line is ignored\n").unwrap();
    let out = run_with_passphrase(&["keygen", &key, "--passphrase-file", &pass], "from env");
    assert_eq!(code(&out), 0, "{}", err(&out));
    assert_eq!(
        code(&run_with_passphrase(&["pubkey", &key], "from the file")),
        0
    );
    assert_eq!(
        code(&run_with_passphrase(&["pubkey", &key], "from env")),
        77
    );
    let out = run(&["pubkey", &key, "--passphrase-file", &pass]);
    assert_eq!(code(&out), 0, "{}", err(&out));
}

#[test]
fn protect_converts_a_plain_key_once() {
    let dir = Dir::new();
    let plain = keygen(&dir, "plain.key");
    let protected = dir.path("protected.key");
    let out = run_with_passphrase(&["protect", &plain, &protected], "pw for protect");
    assert_eq!(code(&out), 0, "{}", err(&out));
    let a = text(&run(&["pubkey", &plain]));
    let b = text(&run_with_passphrase(
        &["pubkey", &protected],
        "pw for protect",
    ));
    assert_eq!(a, b, "same secret");
    // Never overwrites, and refuses to protect twice.
    assert_eq!(
        code(&run_with_passphrase(&["protect", &plain, &protected], "x")),
        73
    );
    let again = dir.path("again.key");
    assert_eq!(
        code(&run_with_passphrase(&["protect", &protected, &again], "x")),
        65
    );
}

#[test]
fn plain_and_passphrase_file_conflict() {
    let dir = Dir::new();
    let (key, pass) = (dir.path("k.key"), dir.path("pass.txt"));
    fs::write(&pass, "pw\n").unwrap();
    let out = run(&["keygen", &key, "--plain", "--passphrase-file", &pass]);
    assert_eq!(code(&out), 64);
    assert!(fs::metadata(&key).is_err(), "nothing written");
}
