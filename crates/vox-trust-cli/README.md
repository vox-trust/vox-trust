# vox-trust-cli

Command-line tool for the [Vox Trust protocol](https://github.com/vox-trust/vox-trust): seal a WAV file with your key, and verify a sealed file, with a verdict as the exit code.

**Pre-1.0, not audited.** It uses file mode, which survives only bit-exact copies (sending the file as a document, not re-encoded as a voice message).

```sh
cargo install vox-trust-cli          # installs the `vox-trust` binary

vox-trust keygen me.key              # asks for a passphrase (Argon2id + XChaCha20-Poly1305)
vox-trust pubkey me.key              # the public key to share with your contacts
vox-trust seal note.wav note.sealed.wav --mode public --key me.key
vox-trust verify note.sealed.wav --pin <PUBLIC_KEY> --contact always
echo $?                              # 0 verified, 1 unsealed, 2 warning, 3 alert
```

`verify --json` prints the full report, including which chunks were modified. Run `vox-trust` with no arguments for every option and exit code.

Exit code 1 means the file has no seal you can check; it is neither an error nor a pass. Treat only 0 as verified.

Licensed under Apache-2.0.
