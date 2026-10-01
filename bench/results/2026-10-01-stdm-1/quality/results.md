Carrier parameters: `Params { lo_bin: 10, hi_bin: 122, tile_bins: 8, tile_frames: 2, columns: 200, sync_every: 6, step_db: 5.0, max_db: 3.5, sync_threshold: 6.0, silence_db: 45.0, valley_db: 30.0, pattern_seed: 8534171955496169729 }`

Window: 6.40 s, capacity 15.9 bit/s. Seals per file: 3. Mean SNR of the marked audio: 15.9 dB (segmental 22.3 dB). Embedding speed: 427x real time on this machine. Detection time total: 24.3 s. Wall time: 78 s.

| Condition | What | Windows recovered | Files with the seal | Wrong seals | False alarms (unmarked) | Max unmarked sync score |
|---|---|---:|---:|---:|---:|---:|
| `original` | no change | 100.0% (81/81) | 39/39 | 0 | 0 | 3.79 |
| `resample-44k` | resample to 44.1 kHz and back | 100.0% (81/81) | 39/39 | 0 | 0 | 3.78 |
| `trim-1.234s` | first 1.234 s removed (tests blind synchronisation) | 100.0% (42/42) | 33/39 | 0 | 0 | 3.96 |
| `noise-20db` | white noise at 20 dB SNR | 0.0% (0/81) | 0/39 | 0 | 0 | 2.85 |
| `noise-30db` | white noise at 30 dB SNR | 4.9% (4/81) | 4/39 | 0 | 0 | 3.40 |
| `noise-10db` | white noise at 10 dB SNR | 0.0% (0/81) | 0/39 | 0 | 0 | 2.81 |
| `mp3-128k` | MP3 128 kbit/s (LAME, 44.1 kHz) | 100.0% (81/81) | 39/39 | 0 | 0 | 3.93 |
| `mp3-64k` | MP3 64 kbit/s (LAME, 44.1 kHz) | 100.0% (81/81) | 39/39 | 0 | 0 | 3.44 |
| `aac-64k` | AAC-LC 64 kbit/s (ffmpeg aac, 44.1 kHz) | 97.5% (79/81) | 39/39 | 0 | 0 | 4.09 |
| `opus-32k` | Opus 32 kbit/s, VoIP mode | 92.6% (75/81) | 39/39 | 0 | 0 | 3.64 |
| `opus-24k` | Opus 24 kbit/s, VoIP mode | 69.1% (56/81) | 33/39 | 0 | 0 | 3.77 |
| `opus-12k` | Opus 12 kbit/s, VoIP mode | 0.0% (0/81) | 0/39 | 0 | 0 | 3.58 |
| `amr-wb-12.65k` | AMR-WB 12.65 kbit/s (mobile HD voice) | 0.0% (0/81) | 0/39 | 0 | 0 | 3.94 |
| `amr-wb-23.85k` | AMR-WB 23.85 kbit/s | 17.3% (14/81) | 11/39 | 0 | 0 | 3.68 |
| `g722` | G.722 64 kbit/s (wideband VoIP) | 82.7% (67/81) | 39/39 | 0 | 0 | 3.61 |
| `mp3-128k+opus-24k` | MP3 128k, then re-shared as Opus 24k | 22.2% (18/81) | 16/39 | 0 | 0 | 4.03 |
| `denoise` | ffmpeg afftdn noise reduction | 0.0% (0/81) | 0/39 | 0 | 0 | 3.83 |
| `echo` | two simulated reflections (40 and 60 ms) | 0.0% (0/81) | 0/39 | 0 | 0 | 3.88 |
| `tempo+1%` | 1 % faster, same pitch (atempo) | 0.0% (0/81) | 0/39 | 0 | 0 | 3.76 |
