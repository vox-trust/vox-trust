# vox-trust-carrier

**Experimental, research only.** An in-band audio watermark that carries a 102-bit Vox Trust seal inside the sound itself (stdm-2: spread-transform dither modulation on log-spectral tiles, 9.6 s per seal, with a tempo search in the detector).

Measured on the project's benchmark, at PESQ-WB 4.41: it survives MP3, AAC, Opus 24 kbit/s, G.722 and MP3 then Opus re-sharing, with no wrong seal and no false alarm. It fails phone-grade AMR-WB 12.65 kbit/s, noise at 20 dB SNR, noise reduction and echo. Full numbers: [benchmark](https://github.com/vox-trust/vox-trust/blob/main/bench/results/2026-10-02-stdm-2/README.md).

It **gives no verdict**: a watermark can be copied from a real recording onto a fake one ([threat model A11](https://github.com/vox-trust/vox-trust/blob/main/spec/THREAT-MODEL.md)), and that is unsolved. Use [`vox-trust-core`](https://crates.io/crates/vox-trust-core) file mode for verdicts.

Licensed under Apache-2.0.
