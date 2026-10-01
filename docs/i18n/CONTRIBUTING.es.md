> Traducción resumida. El [CONTRIBUTING.md en inglés](../../CONTRIBUTING.md) es la versión normativa.

# Contribuir a Vox Trust (resumen)

El proyecto es un borrador v0.x, así que las contribuciones más útiles ahora son **escrutinio y medición**, no funcionalidades.

## Lo más útil ahora

- **Ataca el diseño.** Lee [spec/THREAT-MODEL.md](../../spec/THREAT-MODEL.md) y [spec/SPEC.md](../../spec/SPEC.md) y abre una incidencia con el fallo, la ambigüedad o el atacante que falta.
- **Mide.** La primera fase de la hoja de ruta es medir cómo sobreviven las marcas de agua de audio a rutas reales de audio de teléfono y de aplicaciones (Opus, AAC, MP3, AMR-WB, supresión de ruido, regrabación). Los resultados reproducibles son muy bienvenidos.
- **Implementaciones independientes.** Una especificación solo es real cuando alguien distinto de su autor puede implementarla. Hay vectores de prueba en `spec/test-vectors/`; una segunda implementación en otro lenguaje es la contribución más valiosa.

## Reglas básicas

- Abre una incidencia antes de un cambio grande, para acordar primero la dirección.
- Mantén los cambios pequeños y centrados. Explica el *porqué* en el pull request.
- El código Rust debe pasar `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings` y `cargo test --workspace`.
- Los cambios en el formato de transmisión (wire format) o en los veredictos también requieren: la especificación, vectores regenerados (`cargo run -p vox-trust-core --example gen_vectors`), `python3 tools/check_vectors.py --strict` y, para cambios web/WASM, `scripts/build-web.sh && node --test tests/node/wasm.test.mjs && node tests/browser/demo.mjs`.
- No añadas afirmaciones que el proyecto no pueda respaldar. Si una frase dice "demuestra" o "garantiza", necesita una entrada en el modelo de amenazas.
- Sigue el [código de conducta](../../CODE_OF_CONDUCT.md).

## Licencia de las contribuciones

Al enviar una contribución aceptas que se licencia bajo las licencias del repositorio: Apache-2.0 para el código, CC BY 4.0 para el texto de la especificación (entrada = salida). Debes tener derecho a enviarla.
