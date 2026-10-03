// Smoke test of the generated Swift bindings (macOS; the same code an iOS app uses).
// Built and run by bindings/swift/test.sh.
import Foundation

func tone() -> Data {
    var d = Data()
    func u32(_ v: UInt32) { withUnsafeBytes(of: v.littleEndian) { d.append(contentsOf: $0) } }
    func u16(_ v: UInt16) { withUnsafeBytes(of: v.littleEndian) { d.append(contentsOf: $0) } }
    let n: UInt32 = 16000
    d.append(contentsOf: Array("RIFF".utf8)); u32(36 + 2 * n); d.append(contentsOf: Array("WAVEfmt ".utf8))
    u32(16); u16(1); u16(1); u32(16000); u32(32000); u16(2); u16(16)
    d.append(contentsOf: Array("data".utf8)); u32(2 * n)
    for i in 0..<Int(n) { u16(UInt16(bitPattern: Int16(((i % 80) - 40) * 200))) }
    return d
}

func check(_ ok: Bool, _ what: String) {
    if !ok { print("FAIL: \(what)"); exit(1) }
}

let seed = Data((0..<32).map { UInt8($0) })
let pk = try publicKey(seed: seed)
let sealed = try sealWav(wav: tone(), mode: .public, key: seed, keyId: 0, createdUnix: 1_790_000_000, counter: 0, chunkFrames: 4000)
let contact = Contact(alwaysSeals: true, strict: false)
let trust = Trust(pinnedPublicKey: pk)
check(decide(check: try verifyWav(wav: sealed, trust: trust).check, contact: contact) == .verified, "verified")

var tampered = sealed
tampered[44 + 2 * 10_000] ^= 0x40
let report = try verifyWav(wav: tampered, trust: trust)
check(decide(check: report.check, contact: contact) == .alert, "alert")
check(report.modifiedChunks == [2], "modified chunk 2")
check(decide(check: try verifyWav(wav: tone(), trust: trust).check, contact: contact) == .warning, "warning")
check(decide(check: .absent, contact: nil) == .unsealed, "unsealed")

let key = Data(repeating: 3, count: 32)
let id = try circleKeyId(key: key)
let circle = try sealWav(wav: tone(), mode: .circle, key: key, keyId: id, createdUnix: 1, counter: 0, chunkFrames: 16000)
check(try verifyWav(wav: circle, trust: Trust(circleKey: key, circleKeyId: id)).check == .valid, "circle valid")
do {
    _ = try sealWav(wav: tone(), mode: .circle, key: Data(count: 5), keyId: 0, createdUnix: 1, counter: 0, chunkFrames: 1)
    check(false, "short key must throw")
} catch is VoxTrustError {}
print("swift bindings: all checks passed")
