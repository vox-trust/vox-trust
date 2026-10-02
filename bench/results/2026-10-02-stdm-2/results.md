Carrier parameters: `Params { lo_bin: 10, hi_bin: 122, tile_bins: 8, tile_frames: 2, columns: 300, sync_every: 6, step_db: 7.0, max_db: 4.5, sync_threshold: 6.0, silence_db: 45.0, valley_db: 30.0, pattern_seed: 8534171955496169729, max_tempo_pct: 2.0 }`

Window: 9.60 s, capacity 10.6 bit/s. Seals per file: 3. Mean SNR of the marked audio: 13.2 dB (segmental 21.6 dB). Embedding speed: 444x real time on this machine. Detection time total: 189.1 s. Wall time: 123 s.

| Condition | What | Windows recovered | Files with the seal | Wrong seals | False alarms (unmarked) | Max unmarked sync score |
|---|---|---:|---:|---:|---:|---:|
| `original` | no change | 100.0% (45/45) | 39/39 | 0 | 0 | 3.22 |
| `resample-44k` | resample to 44.1 kHz and back | 100.0% (45/45) | 39/39 | 0 | 0 | 3.23 |
| `trim-1.234s` | first 1.234 s removed (tests blind synchronisation) | 100.0% (6/6) | 6/39 | 0 | 0 | 3.13 |
| `noise-20db` | white noise at 20 dB SNR | 0.0% (0/45) | 0/39 | 0 | 0 | 1.19 |
| `noise-30db` | white noise at 30 dB SNR | 51.1% (23/45) | 20/39 | 0 | 0 | 2.47 |
| `noise-10db` | white noise at 10 dB SNR | 0.0% (0/45) | 0/39 | 0 | 0 | 0.60 |
| `mp3-128k` | MP3 128 kbit/s (LAME, 44.1 kHz) | 100.0% (45/45) | 39/39 | 0 | 0 | 3.23 |
| `mp3-64k` | MP3 64 kbit/s (LAME, 44.1 kHz) | 100.0% (45/45) | 39/39 | 0 | 0 | 3.01 |
| `aac-64k` | AAC-LC 64 kbit/s (ffmpeg aac, 44.1 kHz) | 100.0% (45/45) | 39/39 | 0 | 0 | 3.17 |
| `opus-32k` | Opus 32 kbit/s, VoIP mode | 100.0% (45/45) | 39/39 | 0 | 0 | 3.05 |
| `opus-24k` | Opus 24 kbit/s, VoIP mode | 100.0% (45/45) | 39/39 | 0 | 0 | 2.82 |
| `opus-12k` | Opus 12 kbit/s, VoIP mode | 0.0% (0/45) | 0/39 | 0 | 0 | 2.50 |
| `amr-wb-12.65k` | AMR-WB 12.65 kbit/s (mobile HD voice) | 11.1% (5/45) | 5/39 | 0 | 0 | 2.69 |
| `amr-wb-23.85k` | AMR-WB 23.85 kbit/s | 88.9% (40/45) | 35/39 | 0 | 0 | 3.15 |
| `g722` | G.722 64 kbit/s (wideband VoIP) | 100.0% (45/45) | 39/39 | 0 | 0 | 2.65 |
| `mp3-128k+opus-24k` | MP3 128k, then re-shared as Opus 24k | 97.8% (44/45) | 38/39 | 0 | 0 | 2.58 |
| `denoise` | ffmpeg afftdn noise reduction | 6.7% (3/45) | 3/39 | 0 | 0 | 4.01 |
| `echo` | two simulated reflections (40 and 60 ms) | 0.0% (0/45) | 0/39 | 0 | 0 | 2.49 |
| `tempo+1%` | 1 % faster, same pitch (atempo) | 84.4% (38/45) | 32/39 | 0 | 0 | 2.82 |
