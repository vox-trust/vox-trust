Carrier parameters: `Params { lo_bin: 10, hi_bin: 122, tile_bins: 8, tile_frames: 2, columns: 200, sync_every: 6, step_db: 9.0, max_db: 5.5, sync_threshold: 6.0, silence_db: 45.0, valley_db: 30.0, pattern_seed: 8534171955496169729 }`

Window: 6.40 s, capacity 15.9 bit/s. Seals per file: 3. Mean SNR of the marked audio: 10.6 dB (segmental 18.2 dB). Embedding speed: 387x real time on this machine. Detection time total: 26.0 s. Wall time: 81 s.

| Condition | What | Windows recovered | Files with the seal | Wrong seals | False alarms (unmarked) | Max unmarked sync score |
|---|---|---:|---:|---:|---:|---:|
| `original` | no change | 100.0% (81/81) | 39/39 | 0 | 0 | 3.55 |
| `resample-44k` | resample to 44.1 kHz and back | 100.0% (81/81) | 39/39 | 0 | 0 | 3.54 |
| `trim-1.234s` | first 1.234 s removed (tests blind synchronisation) | 100.0% (42/42) | 33/39 | 0 | 0 | 3.50 |
| `noise-20db` | white noise at 20 dB SNR | 0.0% (0/81) | 0/39 | 0 | 0 | 2.10 |
| `noise-30db` | white noise at 30 dB SNR | 38.3% (31/81) | 22/39 | 0 | 0 | 3.21 |
| `noise-10db` | white noise at 10 dB SNR | 0.0% (0/81) | 0/39 | 0 | 0 | 1.50 |
| `mp3-128k` | MP3 128 kbit/s (LAME, 44.1 kHz) | 100.0% (81/81) | 39/39 | 0 | 0 | 3.60 |
| `mp3-64k` | MP3 64 kbit/s (LAME, 44.1 kHz) | 100.0% (81/81) | 39/39 | 0 | 0 | 3.71 |
| `aac-64k` | AAC-LC 64 kbit/s (ffmpeg aac, 44.1 kHz) | 100.0% (81/81) | 39/39 | 0 | 0 | 3.74 |
| `opus-32k` | Opus 32 kbit/s, VoIP mode | 98.8% (80/81) | 39/39 | 0 | 0 | 3.56 |
| `opus-24k` | Opus 24 kbit/s, VoIP mode | 93.8% (76/81) | 39/39 | 0 | 0 | 3.39 |
| `opus-12k` | Opus 12 kbit/s, VoIP mode | 14.8% (12/81) | 10/39 | 0 | 0 | 3.56 |
| `amr-wb-12.65k` | AMR-WB 12.65 kbit/s (mobile HD voice) | 7.4% (6/81) | 5/39 | 0 | 0 | 3.45 |
| `amr-wb-23.85k` | AMR-WB 23.85 kbit/s | 69.1% (56/81) | 33/39 | 0 | 0 | 3.22 |
| `g722` | G.722 64 kbit/s (wideband VoIP) | 98.8% (80/81) | 39/39 | 0 | 0 | 3.11 |
| `mp3-128k+opus-24k` | MP3 128k, then re-shared as Opus 24k | 85.2% (69/81) | 39/39 | 0 | 0 | 3.37 |
| `denoise` | ffmpeg afftdn noise reduction | 0.0% (0/81) | 0/39 | 0 | 0 | 3.52 |
| `echo` | two simulated reflections (40 and 60 ms) | 0.0% (0/81) | 0/39 | 0 | 0 | 2.95 |
| `tempo+1%` | 1 % faster, same pitch (atempo) | 0.0% (0/81) | 0/39 | 0 | 0 | 3.43 |
