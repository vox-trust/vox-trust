> Traducción resumida. El [SECURITY.md en inglés](../../SECURITY.md) es la versión normativa.

# Política de seguridad (resumen)

Vox Trust está **antes de la 1.0**: especificación versión 0.2 (modo archivo como release candidate, partes dentro del audio experimentales), software v0.x. El código no ha sido revisado ni auditado. No confíes en él para proteger a nadie.

- **Versiones soportadas:** solo la última versión en `main`.
- **Objetivos de divulgación:** acusar recibo en un plazo de 7 días y publicar una corrección y un aviso (advisory) en un plazo de 90 días tras un informe válido. Es un proyecto voluntario: son objetivos, no garantías. Por ahora no hay programa de recompensas (bug bounty), pero los informes se leen y se reconocen, si lo deseas.
- **Alcance:** el código y la especificación de este repositorio y la demo en vox-trust.github.io/demo/. Fuera de alcance: ataques que requieren la clave o el dispositivo de la víctima (ver el modelo de amenazas).

## Cómo informar de una vulnerabilidad

Informa **en privado** mediante el informe privado de vulnerabilidades de GitHub:

<https://github.com/vox-trust/vox-trust/security/advisories/new>

Si esa página no está disponible para ti, abre una incidencia pública mínima con el título "security contact request" **sin ningún detalle**; un mantenedor acordará un canal privado. No abras una incidencia pública para una vulnerabilidad. Indica qué encontraste, cómo reproducirlo y qué parte de la especificación o del código se ve afectada.

## Vulnerabilidad o comentario de diseño

- **Vulnerabilidad (informa en privado):** una forma de falsificar un sello que verifique, una repetición (replay) o un empalme que pase la política, un fallo en el código de referencia que rompa una garantía declarada.
- **Comentario de diseño (abre una incidencia pública):** críticas al modelo de amenazas, a la especificación en borrador o a una afirmación de la documentación. El escrutinio público del diseño es bienvenido.

Los atacantes y límites que ya conocemos están en [spec/THREAT-MODEL.md](../../spec/THREAT-MODEL.md).
