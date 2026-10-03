<p align="center">
  🌐 <a href="README.md">English</a> · <a href="README.pt-BR.md">Português (Brasil)</a> · <strong>Español</strong> · <a href="README.zh-CN.md">简体中文</a> · <a href="README.ar.md">العربية</a>
</p>

> Esta es una traducción. El [README en inglés](README.md) y la [especificación](spec/SPEC.md) son las versiones normativas.

<h1 align="center">Vox Trust</h1>

<p align="center"><strong>No detectes voces falsas. Demuestra las reales.</strong></p>

<p align="center">
Un protocolo abierto, con una implementación de referencia en Rust, que sella una voz humana en el origen y permite que cualquiera la verifique después, directamente en el navegador.
</p>

<p align="center">
  <a href="https://github.com/vox-trust/vox-trust/actions/workflows/ci.yml"><img alt="CI" src="https://github.com/vox-trust/vox-trust/actions/workflows/ci.yml/badge.svg"></a>
  <img alt="Rust" src="https://img.shields.io/badge/Rust-2021-orange?logo=rust&logoColor=white">
  <img alt="WebAssembly" src="https://img.shields.io/badge/WebAssembly-no%20imports-654FF0?logo=webassembly&logoColor=white">
  <a href="LICENSE"><img alt="License: Apache-2.0" src="https://img.shields.io/badge/license-Apache--2.0-blue"></a>
  <img alt="Version 0.5.1" src="https://img.shields.io/badge/version-0.5.1-informational">
  <img alt="Not audited" src="https://img.shields.io/badge/security-not%20audited-red">
</p>

<p align="center">
  <a href="https://vox-trust.github.io/demo/?lang=es"><strong>Demo en vivo</strong></a> ·
  <a href="https://vox-trust.github.io/es/">Sitio web</a> ·
  <a href="spec/SPEC.md">Especificación</a> ·
  <a href="spec/THREAT-MODEL.md">Modelo de amenazas</a> ·
  <a href="docs/ROADMAP.md">Hoja de ruta</a>
</p>

> **Estado: v0.5, el modo archivo funciona. Sin auditar.** Puedes sellar un archivo WAV, verificarlo y ver exactamente qué segundos fueron alterados, en la [demo en el navegador](https://vox-trust.github.io/demo/?lang=es) o con la línea de comandos. Los archivos sellados sobreviven **solo a copias idénticas bit a bit**. Una marca de agua de audio **experimental**, que sobrevive a MP3, AAC, Opus y pequeños cambios de velocidad, está construida y [medida](bench/results/2026-10-02-stdm-2/README.md). Pero falla con códecs de llamadas telefónicas y con ruido, y los sellos dentro del audio pueden copiarse a otro audio, así que **todavía no da veredictos**. No uses esto para proteger a nadie hasta que haya sido revisado.

## El problema

Clonar una voz hoy requiere solo unos segundos de audio. Escuchar ya no permite distinguir una voz real de una clonada, y los detectores persiguen un blanco móvil: a medida que mejora la detección, también mejora la generación.

## La idea

En lugar de adivinar si una voz es falsa, comprueba si una voz real fue **sellada**.

1. **Sellar.** El dispositivo de quien habla firma el audio en el origen.
2. **Verificar.** Cualquiera que tenga el protocolo comprueba qué clave lo selló, cuándo, y qué fragmentos del audio fueron alterados.
3. **Decidir.** Una política de confianza local convierte el resultado en uno de cuatro desenlaces.

| Resultado | Significado |
|---|---|
| **Verificado** | Un sello válido de una clave en la que confías. |
| **Sin sello** | No hay sello (o hay un sello de una clave que nunca proporcionaste), de alguien de quien no esperas nada. Neutral, **no** es "falso". |
| **Advertencia** | Falta el sello de un contacto que *siempre* sella: merece una segunda mirada, porque la compresión también puede eliminar un sello. |
| **Alerta** | El sello está roto, proviene de una clave que no fijaste para ese contacto, o falta en un contacto que *siempre* sella (modo estricto). |

Dos modos: **círculo** (personas que se conocen, secreto compartido) y **público** (organizaciones y figuras públicas, claves Ed25519 que los verificadores fijan).

## Pruébalo en 30 segundos

**En el navegador** (no se sube nada; el núcleo en Rust se ejecuta como WebAssembly): abre la [demo](https://vox-trust.github.io/demo/?lang=es), pulsa *Sellar*, luego intenta hacer trampa con los botones y observa cómo el verificador detecta cada intento.

**En la línea de comandos** (hay binarios listos para Linux, macOS y Windows, con sumas de verificación y atestaciones de compilación, en la [página de versiones](https://github.com/vox-trust/vox-trust/releases/latest); o compila desde el código fuente):

```sh
cargo install --locked --git https://github.com/vox-trust/vox-trust --tag v0.5.1 vox-trust-cli

vox-trust keygen me.key                                  # asks for a passphrase
vox-trust seal speech.wav sealed.wav --mode circle --key me.key
vox-trust verify sealed.wav --circle-key me.key          # exit code 0 = verified
```

`keygen` protege la clave con una frase de contraseña (Argon2id y XChaCha20-Poly1305) y la pide cada vez que se usa la clave. En scripts, usa `--passphrase-file` o la variable `VOX_TRUST_PASSPHRASE`; `--plain` escribe una clave sin protección.

Edita una sola muestra de `sealed.wav` y verifica de nuevo: el código de salida pasa a ser 3 y se imprime el fragmento alterado. Códigos de salida: 0 verificado, 1 sin sello, 2 aviso, 3 alerta.

**Como biblioteca de Rust:**

```rust
// A sketch: `wav_bytes` is a 16-bit PCM WAV you already have; run inside a function returning a Result.
use vox_trust_core::file::{seal_wav, verify_wav, SealParams, Signer, Trust};
use vox_trust_core::{circle, SealCheck};

let key = [7u8; 32]; // use a random 32-byte secret in real life
let sealed = seal_wav(
    &wav_bytes,
    Signer::Circle { key: &key, key_id: circle::key_id(&key) },
    SealParams { created_unix: 1_700_000_000, counter: 1, chunk_frames: 16_000 },
)?;
let trust = Trust { circle: Some((circle::key_id(&key), &key)), pinned_public: None };
assert_eq!(verify_wav(&sealed, trust)?.check, SealCheck::Valid);
```

## Intégralo en tu app

Vox Trust es un protocolo abierto pensado para vivir dentro de las apps que la gente ya usa: mensajería, buzón de voz, herramientas de pódcast y de redacción, plataformas de atención. Sella al enviar, verifica al recibir.

```sh
npm install vox-trust        # browsers and Node 20+, WebAssembly, no dependencies, types included
cargo add vox-trust-core     # Rust, no unsafe, no I/O
pip install vox-trust        # Python 3.9+
npx -y vox-trust-mcp         # MCP server: AI assistants verify recordings
```

```js
import { load } from "vox-trust";
const vt = await load();
const sealed = vt.seal(wav, { mode: "public", key: seed, createdUnix, chunkFrames: 16000 });
const report = vt.verify(sealed, { pinnedPublicKey });
vt.decide(report.check, { alwaysSeals: true, strict: false }); // "verified" | "unsealed" | "warning" | "alert"
```

La [guía de integración](docs/INTEGRATION.md) lleva unos 10 minutos (en inglés): claves, emparejamiento, los cuatro veredictos y qué mostrar en cada uno, con [ejemplos](examples/) ejecutables para Node, el navegador (micrófono) y Rust. Las bibliotecas para Android (Kotlin) e iOS (Swift) se adjuntan a cada [release](https://github.com/vox-trust/vox-trust/releases) ([bindings](bindings/)). Apache-2.0, sin servicio que llamar, sin cuenta.

## Lo que esto NO es

- **No** es detección de deepfakes y **no** es biometría de voz.
- **No** demuestra que quien habla sea humano ni que sea quien dice ser. Demuestra que *una clave* selló el audio. Una clave robada o un dispositivo comprometido producen sellos válidos.
- "Sin sello" **no** significa "falso": la compresión y la supresión de ruido pueden borrar una marca de agua.
- El modo archivo **no** sobrevive a MP3, AAC, remuestreo ni regrabación. Eso requiere el portador (carrier), que todavía no existe.
- No protege contra un atacante que controla el dispositivo de quien envía.

Los atacantes, las afirmaciones y las debilidades encontradas hasta ahora (incluida una que sigue sin resolverse) están en el [modelo de amenazas](spec/THREAT-MODEL.md).

## Cómo se prueba

| Capa | Qué comprueba | Comando |
|---|---|---|
| Pruebas unitarias y de integración en Rust | Disposición del sello, HMAC y Ed25519 (contra los vectores de las RFC 4231 y RFC 8032), análisis de WAV, manifiestos, manipulación, política, la interfaz C y la CLI | `cargo test --workspace` |
| Vectores de prueba publicados | Sellos y manifiestos idénticos byte a byte en [`spec/test-vectors/`](spec/test-vectors) | incluido arriba |
| Verificación cruzada en Python | `tools/check_vectors.py` reconstruye cada vector a partir del texto de la especificación (biblioteca estándar, más `cryptography` para Ed25519) | `pip install cryptography && python3 tools/check_vectors.py --strict` |
| WebAssembly de extremo a extremo | El módulo compilado reproduce los vectores byte a byte, maneja entradas basura y no pierde memoria | `scripts/build-web.sh && node --test tests/node/wasm.test.mjs` |
| Navegador real | La página de la demo en Chromium, Firefox y WebKit headless: sellado, seis ataques, fijación de clave pública, accesibilidad, ancho de teléfono, modo oscuro | `BROWSER=firefox node tests/browser/demo.mjs` |
| Fuzzing | Todo parser de entrada no confiable, con invariantes (el audio sellado siempre verifica, cualquier bit de muestra alterado se detecta en el fragmento correcto, el texto de emparejamiento tiene una sola forma) | `cd fuzz && cargo +nightly fuzz run verify_wav` |
| Cobertura | Proporción de líneas de Rust que ejecutan las pruebas; la CI falla por debajo del 96 % | `scripts/coverage.sh` |
| Pruebas de mutación | Cambia operadores y valores de retorno de `vox-trust-core` y del código de archivos de clave, uno a la vez, y comprueba que alguna prueba falla; los pocos cambios que no pueden alterar el comportamiento están listados con el motivo en `.cargo/mutants.toml` | `cargo mutants -p vox-trust-core`; `cargo mutants -p vox-trust-cli -f crates/vox-trust-cli/src/keyfile.rs` |
| Dependencias | Vulnerabilidades conocidas, licencias, orígenes | `cargo deny check` |

La CI ejecuta las pruebas de Rust en Linux, macOS y Windows.

La verificación en Python la escribió el mismo autor, así que es una verificación cruzada, no una implementación independiente. [Una implementación independiente es lo que la especificación todavía necesita.](docs/ROADMAP.md)

## Cómo se compara

Casi toda la industria marca el **audio generado por IA** para reconocerlo después. Vox Trust hace lo contrario: avala el **habla humana real**, con una prueba que cualquiera puede comprobar.

| | <small>**Vox Trust**</small> | <small>Google SynthID</small> | <small>Meta AudioSeal</small> | <small>Resemble PerTh</small> | <small>Detectores de deepfakes¹</small> | <small>C2PA</small> |
|---|:-:|:-:|:-:|:-:|:-:|:-:|
| <small>Avala una grabación humana real</small> | <small>✅</small> | <small>❌ marca salida de IA</small> | <small>❌ marca salida de IA</small> | <small>❌ marca salida de IA</small> | <small>⚠️ estima</small> | <small>✅ si la app de grabación firma</small> |
| <small>Prueba ligada a la clave del propio hablante</small> | <small>✅</small> | <small>❌</small> | <small>❌</small> | <small>❌</small> | <small>❌</small> | <small>✅ certificado del firmante</small> |
| <small>Señala los segundos alterados</small> | <small>✅</small> | <small>❌</small> | <small>⚠️ regiones marcadas o no</small> | <small>❌</small> | <small>❌</small> | <small>❌ archivo completo</small> |
| <small>Cualquiera verifica, sin conexión</small> | <small>✅ en el navegador</small> | <small>❌ detector de Google</small> | <small>✅</small> | <small>⚠️</small> | <small>❌</small> | <small>✅</small> |
| <small>Especificación y código abiertos</small> | <small>✅</small> | <small>❌ para audio</small> | <small>✅ código</small> | <small>⚠️ código</small> | <small>❌</small> | <small>✅ especificación</small> |
| <small>Sobrevive a MP3, AAC, Opus</small> | <small>⚠️ marca de agua experimental, 100 % medido</small> | <small>✅ declarado</small> | <small>✅ medido</small> | <small>✅ declarado</small> | <small>no aplica</small> | <small>❌ los metadatos suelen eliminarse</small> |
| <small>Sobrevive a llamadas (AMR-WB)</small> | <small>❌ 11 %</small> | <small>no publicado</small> | <small>❌ 0 % medido</small> | <small>no publicado</small> | <small>✅</small> | <small>❌</small> |
| <small>Coste de verificar</small> | <small>milisegundos, sin modelo de IA</small> | <small>servicio en la nube</small> | <small>red neuronal</small> | <small>red neuronal</small> | <small>servicio en la nube</small> | <small>milisegundos</small> |
| <small>Benchmark público, con los fallos</small> | <small>✅</small> | <small>❌</small> | <small>artículo científico</small> | <small>❌</small> | <small>cifras del proveedor</small> | <small>no aplica</small> |

"Medido" significa ejecutado en [nuestro benchmark](bench/results/2026-10-02-neural-baselines/README.md), con el mismo corpus y los mismos códecs; "declarado" es la cifra publicada por el proveedor. ¹ Por ejemplo [Pindrop Pulse](https://www.pindrop.com/article/pindrop-pulse-for-audio-deepfake-detection/): estima si una voz es sintética, lo cual es útil, pero es una probabilidad, no una prueba de quién habló. Las fuentes de cada celda están en las [notas de la comparación](docs/COMPARISON.md) (en inglés).

## Dónde encaja

Vox Trust se apoya en el trabajo existente y lo complementa, en lugar de reemplazarlo:

- Credenciales de contenido [C2PA](https://spec.c2pa.org/): procedencia para archivos, con marcas de agua como "soft bindings".
- Marcas de agua de audio como [AudioSeal](https://github.com/facebookresearch/audioseal) y [WavMark](https://github.com/wavmark/wavmark), que podrían servir de portadoras.
- Investigación sobre procedencia del habla con clave pública, por ejemplo [MerkleSpeech](https://arxiv.org/abs/2602.10166).
- Aplicaciones que rotan palabras clave familiares (p. ej., Trust Onion) y proveedores de detección atacan la misma estafa desde otros ángulos.
- STIR/SHAKEN certifica el *número* de quien llama, no la voz.

## Estructura del repositorio

```
spec/        especificación, modelo de amenazas, vectores de prueba
crates/
  vox-trust-core/   disposición del sello, modos círculo y público, modo archivo, política, utilidades de repetición, texto de emparejamiento
  vox-trust-wasm/   el núcleo como módulo WebAssembly (interfaz C simple, sin imports)
  vox-trust-cli/    la herramienta de línea de comandos `vox-trust`
web/         la demo en el navegador (publicada en vox-trust.github.io/demo/)
bindings/    bindings para Kotlin (Android), Swift (iOS, macOS) y Python (UniFFI)
mcp/         servidor MCP para que asistentes de IA verifiquen grabaciones
npm/         el paquete npm `vox-trust` (WebAssembly + envoltorio JavaScript + tipos)
extension/   extensión de navegador para Chrome, Edge, Firefox y Safari
examples/    ejemplos de integración: Node, navegador, Rust
tests/       pruebas de Node (WebAssembly) y de navegador
tools/       verificación independiente de vectores
fuzz/        objetivos de fuzzing (cargo-fuzz) y entradas semilla
scripts/     utilidades de compilación y publicación
docs/        hoja de ruta
```

## Contribuir, seguridad, licencia

- Lee [CONTRIBUTING.md](CONTRIBUTING.md) ([resumen en español](docs/i18n/CONTRIBUTING.es.md)). Lo más útil ahora es el escrutinio: ataca el [modelo de amenazas](spec/THREAT-MODEL.md) o escribe una implementación independiente a partir de la [especificación](spec/SPEC.md).
- Reporta vulnerabilidades **en privado**: consulta [SECURITY.md](SECURITY.md) ([resumen en español](docs/i18n/SECURITY.es.md)).
- Código: [Apache-2.0](LICENSE). Texto de la especificación: CC BY 4.0, consulta [spec/LICENSE.md](spec/LICENSE.md). Cambios: [CHANGELOG.md](CHANGELOG.md).
- Gobernanza y uso del nombre: [GOVERNANCE.md](GOVERNANCE.md), [TRADEMARKS.md](TRADEMARKS.md). Para citar: [CITATION.cff](CITATION.cff).
- Mantenedor: [Roger Oliveira](https://www.linkedin.com/in/rogeroliveira/) · [@rogeroliveira84](https://github.com/rogeroliveira84).
