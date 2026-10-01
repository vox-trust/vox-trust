#!/usr/bin/env bash
# Builds the benchmark speech corpus in bench/corpus/ (16 kHz, mono, 16-bit WAV) from openly
# licensed recordings in public GitHub repositories, each pinned to a commit.
#
# Sources and licences (see bench/CORPUS.md):
#   librosa/data          LibriSpeech excerpts, CC BY 4.0
#   openai/whisper        JFK inaugural address excerpt, public domain (US government work)
#   coqui-ai/TTS          LJ Speech excerpts, public domain
#   Picovoice/leopard     test recordings, Apache-2.0 repository
#   Picovoice/cheetah     test recordings, Apache-2.0 repository
#   wenet-e2e/wenet       AISHELL-1 excerpts, Apache-2.0
#
# Usage: bench/fetch-corpus.sh [ffmpeg]   (needs git and ffmpeg)
set -euo pipefail
cd "$(dirname "$0")"
ffmpeg="${1:-ffmpeg}"
src="$(pwd)/.corpus-src"
out="$(pwd)/corpus"
mkdir -p "$src" "$out"

fetch() { # repo commit paths...
  local repo="$1" commit="$2"; shift 2
  local dir="$src/${repo//\//_}"
  if [ ! -d "$dir/.git" ]; then
    git init -q "$dir"
    git -C "$dir" remote add origin "https://github.com/$repo"
  fi
  git -C "$dir" fetch -q --depth 1 --filter=blob:none origin "$commit"
  git -C "$dir" checkout -q FETCH_HEAD -- "$@"
}

fetch librosa/data 38f4b06556fa0ff1acda5e677d8ba05d1bc0fff0 \
  audio/198-209-0000.hq.ogg audio/3436-172162-0000.hq.ogg audio/5703-47212-0000.hq.ogg
fetch openai/whisper 86098128c0b4f24f0e2aa2994de830614b474227 tests/jfk.flac
fetch coqui-ai/TTS dbf1a08a0d4e47fdad6172e433eeb34bc6b13b4e \
  tests/data/ljspeech/wavs/LJ001-0001.wav tests/data/ljspeech/wavs/LJ001-0003.wav \
  tests/data/ljspeech/wavs/LJ001-0005.wav
fetch Picovoice/leopard 302bc81d00b13c8a27519990ba76351578c007dd resources/audio_samples
fetch Picovoice/cheetah 0a80812feed46edc8d35c3668228066351ed5dd0 resources/audio_samples
fetch wenet-e2e/wenet d17059667d6afe0680d19b3a4948ab825ef25105 \
  test/resources/aishell-BAC009S0724W0121.wav runtime/gpu/client/test_wavs

L="$src/Picovoice_leopard/resources/audio_samples"
C="$src/Picovoice_cheetah/resources/audio_samples"
W="$src/wenet-e2e_wenet"

# name: one or more source files, joined with 0.3 s of silence.
make() {
  local name="$1"; shift
  local args=() filter="" i=0
  for f in "$@"; do
    args+=(-i "$f")
    filter+="[$i:a]aformat=sample_fmts=fltp:channel_layouts=mono,aresample=16000[a$i];"
    i=$((i + 1))
  done
  local concat=""
  for ((k = 0; k < i; k++)); do
    concat+="[a$k]"
    if [ $k -lt $((i - 1)) ]; then
      filter+="aevalsrc=0:d=0.3:s=16000[g$k];"
      concat+="[g$k]"
    fi
  done
  local n=$((2 * i - 1))
  "$ffmpeg" -hide_banner -loglevel error -y "${args[@]}" \
    -filter_complex "${filter}${concat}concat=n=${n}:v=0:a=1[o]" -map "[o]" \
    -ar 16000 -ac 1 -c:a pcm_s16le "$out/$name.wav"
}

make en-libri-198 "$src/librosa_data/audio/198-209-0000.hq.ogg"
make en-libri-3436 "$src/librosa_data/audio/3436-172162-0000.hq.ogg"
make en-libri-5703 "$src/librosa_data/audio/5703-47212-0000.hq.ogg"
make en-jfk "$src/openai_whisper/tests/jfk.flac"
make en-ljspeech "$src/coqui-ai_TTS/tests/data/ljspeech/wavs/LJ001-0001.wav" \
  "$src/coqui-ai_TTS/tests/data/ljspeech/wavs/LJ001-0003.wav" \
  "$src/coqui-ai_TTS/tests/data/ljspeech/wavs/LJ001-0005.wav"
make en-conversation "$L/diarization_test.wav"
make de "$L/test_de.wav" "$C/test_de.wav" "$C/test_de_norm.wav"
make es "$L/test_es.wav" "$C/test_es.wav" "$C/test_es_norm.wav"
make fr "$L/test_fr.wav" "$C/test_fr.wav" "$C/test_fr_norm.wav"
make it "$L/test_it.wav" "$C/test_it.wav" "$C/test_it_norm.wav"
make pt "$L/test_pt.wav" "$C/test_pt.wav" "$C/test_pt_norm.wav"
make ja-ko "$L/test_ja.wav" "$L/test_ko.wav"
make zh "$W/test/resources/aishell-BAC009S0724W0121.wav" "$W/runtime/gpu/client/test_wavs/long.wav" \
  "$W/runtime/gpu/client/test_wavs/mid.wav"
# Not speech: used only to measure false alarms.
make control-room-tone "$L/empty.wav"

echo "corpus ready in $out:"
for f in "$out"/*.wav; do
  awk -v s="$(stat -c %s "$f")" -v n="$(basename "$f")" 'BEGIN { printf "  %-24s %6.1f s\n", n, (s - 44) / 32000 }'
done
