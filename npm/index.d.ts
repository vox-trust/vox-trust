/** Error thrown with a stable machine-readable code, for localized messages. */
export interface CodedError extends Error {
  code: string;
  params: Record<string, unknown>;
}

export type SealMode = "circle" | "public";
export type SealCheck = "valid" | "invalid" | "unknown_key" | "absent";
export type Verdict = "verified" | "unsealed" | "warning" | "alert";
export type Reason =
  | "none"
  | "no_manifest"
  | "malformed"
  | "unsupported_version"
  | "bad_authenticator"
  | "bad_signature"
  | "untrusted_key"
  | "format_changed"
  | "modified";

export interface SealOptions {
  mode: SealMode;
  /** 32 bytes: the circle secret, or the Ed25519 seed in public mode. */
  key: Uint8Array;
  /** Circle key identifier written into the manifest (circle mode). Default 0. */
  keyId?: number;
  /** Creation time in Unix seconds. */
  createdUnix: number;
  /** Informational counter, authenticated with the manifest. Default 0. */
  counter?: number;
  /** Audio frames per chunk, the unit of tamper localization (for example the sample rate, for 1 s). */
  chunkFrames: number;
}

export interface Trust {
  circleKey?: Uint8Array;
  circleKeyId?: number;
  /** The Ed25519 public key pinned for this contact. */
  pinnedPublicKey?: Uint8Array;
}

export interface Contact {
  /** This contact always seals, so a missing seal is suspicious. */
  alwaysSeals: boolean;
  /** A missing seal from this contact is an alert, not a warning. */
  strict: boolean;
}

export interface Report {
  check: SealCheck;
  reason: Reason;
  mode: SealMode | null;
  /** Key identifier as 8 hex characters. */
  key_id: string | null;
  created_unix: number | null;
  counter: number | null;
  sample_rate: number;
  channels: number;
  n_frames: number;
  chunk_frames: number | null;
  n_chunks: number | null;
  /** The authenticator verified under a key the verifier trusts. */
  authenticated: boolean;
  /** The authenticator is genuine, whether or not its key is trusted. */
  authenticator_valid: boolean;
  /** Audio format and every chunk match the manifest (integrity, not authorship). */
  content_matches: boolean;
  /** Indexes of the chunks that changed after sealing. */
  modified_chunks: number[];
  /** Hex public key carried by a public-mode manifest. */
  embedded_public_key: string | null;
}

export interface VoxTrust {
  abiVersion(): number;
  memoryBytes(): number;
  circleKeyId(key: Uint8Array): number;
  publicKey(seed: Uint8Array): { publicKey: Uint8Array; keyId: number };
  /** Seals a 16-bit PCM WAV file and returns the sealed bytes. */
  seal(wav: Uint8Array, options: SealOptions): Uint8Array;
  verify(wav: Uint8Array, trust?: Trust): Report;
  /** The trust policy. Pass null when the speaker is not a known contact. */
  decide(check: SealCheck, contact?: Contact | null): Verdict;
}

export interface WavInfo {
  channels: number;
  sampleRate: number;
  bits: number;
  pcmOffset: number;
  pcmLength: number;
  manifestOffset: number;
  manifestLength: number;
  chunks: { id: string; start: number; size: number }[];
}

/** Loads the bundled WebAssembly core, or the one at `source` if given. */
export function load(source?: ArrayBuffer | Uint8Array | string | URL): Promise<VoxTrust>;
export function loadVoxTrust(source: ArrayBuffer | Uint8Array | string | URL): Promise<VoxTrust>;
/** Derives a 32-byte circle key from a passphrase (PBKDF2-HMAC-SHA-256, 600 000 iterations by default). */
export function deriveCircleKey(passphrase: string, salt?: string, iterations?: number): Promise<Uint8Array>;
export function hex(bytes: Uint8Array): string;
export function unhex(text: string): Uint8Array;
export function coded(code: string, message: string, params?: Record<string, unknown>): CodedError;
export function wavInfo(bytes: Uint8Array): WavInfo;
/** Builds a 16-bit PCM WAV file from interleaved samples. */
export function encodeWav(samples: Int16Array, sampleRate: number, channels?: number): Uint8Array;
/** A copy of the WAV without its seal. */
export function stripManifest(bytes: Uint8Array): Uint8Array;
