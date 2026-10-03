import io.github.voxtrust.Contact
import io.github.voxtrust.SealCheck
import io.github.voxtrust.SealMode
import io.github.voxtrust.Trust
import io.github.voxtrust.Verdict
import io.github.voxtrust.VoxTrustException
import io.github.voxtrust.circleKeyId
import io.github.voxtrust.decide
import io.github.voxtrust.publicKey
import io.github.voxtrust.sealWav
import io.github.voxtrust.verifyWav
import java.nio.ByteBuffer
import java.nio.ByteOrder
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith

class VoxTrustTest {
    private fun tone(): ByteArray {
        val n = 16000
        val b = ByteBuffer.allocate(44 + 2 * n).order(ByteOrder.LITTLE_ENDIAN)
        b.put("RIFF".toByteArray()).putInt(36 + 2 * n).put("WAVEfmt ".toByteArray())
        b.putInt(16).putShort(1).putShort(1).putInt(16000).putInt(32000).putShort(2).putShort(16)
        b.put("data".toByteArray()).putInt(2 * n)
        for (i in 0 until n) b.putShort((((i % 80) - 40) * 200).toShort())
        return b.array()
    }

    @Test
    fun publicModeVerdictsAndTamperLocalization() {
        val seed = ByteArray(32) { it.toByte() }
        val pk = publicKey(seed)
        val sealed = sealWav(tone(), SealMode.PUBLIC, seed, 0u, 1_790_000_000uL, 0u, 4000u)
        val contact = Contact(alwaysSeals = true, strict = false)
        val trust = Trust(pinnedPublicKey = pk)
        assertEquals(Verdict.VERIFIED, decide(verifyWav(sealed, trust).check, contact))

        val tampered = sealed.copyOf()
        tampered[44 + 2 * 10_000] = (tampered[44 + 2 * 10_000].toInt() xor 0x40).toByte()
        val report = verifyWav(tampered, trust)
        assertEquals(Verdict.ALERT, decide(report.check, contact))
        assertEquals(listOf(2u), report.modifiedChunks)

        assertEquals(Verdict.WARNING, decide(verifyWav(tone(), trust).check, contact))
        assertEquals(Verdict.UNSEALED, decide(SealCheck.ABSENT, null))
    }

    @Test
    fun circleModeAndErrors() {
        val key = ByteArray(32) { 3 }
        val id = circleKeyId(key)
        val sealed = sealWav(tone(), SealMode.CIRCLE, key, id, 1uL, 0u, 16000u)
        assertEquals(SealCheck.VALID, verifyWav(sealed, Trust(circleKey = key, circleKeyId = id)).check)
        assertFailsWith<VoxTrustException> { sealWav(tone(), SealMode.CIRCLE, ByteArray(5), 0u, 1uL, 0u, 1u) }
        assertFailsWith<VoxTrustException> { verifyWav("not a wav".toByteArray(), Trust()) }
    }
}
