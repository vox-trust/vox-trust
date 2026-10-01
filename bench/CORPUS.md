# Benchmark corpus

`fetch-corpus.sh` builds 14 files at 16 kHz, mono, 16-bit, from recordings in public GitHub
repositories, each pinned to a commit. The audio is not stored in this repository.

| File | Language | Source (repository, path) | Licence |
|---|---|---|---|
| `en-libri-198` | English | librosa/data, `audio/198-209-0000.hq.ogg` (LibriSpeech) | CC BY 4.0 |
| `en-libri-3436` | English | librosa/data, `audio/3436-172162-0000.hq.ogg` (LibriSpeech) | CC BY 4.0 |
| `en-libri-5703` | English | librosa/data, `audio/5703-47212-0000.hq.ogg` (LibriSpeech) | CC BY 4.0 |
| `en-jfk` | English | openai/whisper, `tests/jfk.flac` (1961 inaugural address) | Public domain (US government work) |
| `en-ljspeech` | English | coqui-ai/TTS, `tests/data/ljspeech/wavs/LJ001-000{1,3,5}.wav` (LJ Speech) | Public domain |
| `en-conversation` | English, several speakers | Picovoice/leopard, `resources/audio_samples/diarization_test.wav` | Apache-2.0 repository |
| `de`, `es`, `fr`, `it`, `pt` | German, Spanish, French, Italian, Portuguese | Picovoice/leopard and Picovoice/cheetah, `resources/audio_samples/test_*.wav` | Apache-2.0 repositories |
| `ja-ko` | Japanese, then Korean | Picovoice/leopard, `test_ja.wav`, `test_ko.wav` | Apache-2.0 repository |
| `zh` | Mandarin | wenet-e2e/wenet, AISHELL-1 excerpts | Apache-2.0 |
| `control-room-tone` | none (room tone, no speech) | Picovoice/leopard, `empty.wav` | Apache-2.0 repository |

LibriSpeech: V. Panayotov, G. Chen, D. Povey, S. Khudanpur, "LibriSpeech: an ASR corpus based
on public domain audio books", ICASSP 2015. LJ Speech: K. Ito and L. Johnson, 2017.
AISHELL-1: H. Bu, J. Du, X. Na, B. Wu, H. Zheng, 2017.

Limits: about 3.5 minutes of speech, mostly read or prompted, mostly one speaker per file,
recorded in quiet conditions. It is enough to find what clearly works and what clearly fails,
not to estimate rates precisely. A larger corpus with spontaneous, noisy and phone-recorded
speech is a roadmap item.
