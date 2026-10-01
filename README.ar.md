<p align="center">
  🌐 <a href="README.md">English</a> · <a href="README.pt-BR.md">Português (Brasil)</a> · <a href="README.es.md">Español</a> · <a href="README.zh-CN.md">简体中文</a> · <strong>العربية</strong>
</p>

<div dir="rtl">

> هذه ترجمة. [ملف README الإنجليزي](README.md) و[المواصفة](spec/SPEC.md) هما النسختان المعتمدتان.

</div>

<h1 align="center">Vox Trust</h1>

<p align="center"><strong>لا تكشف الأصوات المزيفة. أثبت الأصوات الحقيقية.</strong></p>

<p align="center">
بروتوكول مفتوح مع تنفيذ مرجعي بلغة Rust، يختم الصوت البشري عند المصدر ويتيح لأي شخص التحقق منه لاحقًا مباشرة في المتصفح.
</p>

<p align="center">
  <a href="https://github.com/vox-trust/vox-trust/actions/workflows/ci.yml"><img alt="CI" src="https://github.com/vox-trust/vox-trust/actions/workflows/ci.yml/badge.svg"></a>
  <img alt="Rust" src="https://img.shields.io/badge/Rust-2021-orange?logo=rust&logoColor=white">
  <img alt="WebAssembly" src="https://img.shields.io/badge/WebAssembly-no%20imports-654FF0?logo=webassembly&logoColor=white">
  <a href="LICENSE"><img alt="License: Apache-2.0" src="https://img.shields.io/badge/license-Apache--2.0-blue"></a>
  <img alt="Version 0.3.0" src="https://img.shields.io/badge/version-0.3.0-informational">
  <img alt="Not audited" src="https://img.shields.io/badge/security-not%20audited-red">
</p>

<p align="center">
  <a href="https://vox-trust.github.io/demo/?lang=ar"><strong>العرض التجريبي المباشر</strong></a> ·
  <a href="https://vox-trust.github.io/ar/">الموقع</a> ·
  <a href="spec/SPEC.md">المواصفة</a> ·
  <a href="spec/THREAT-MODEL.md">نموذج التهديدات</a> ·
  <a href="docs/ROADMAP.md">خارطة الطريق</a>
</p>

<div dir="rtl">

> **الحالة: الإصدار v0.3، وضع الملفات يعمل. لم يخضع لتدقيق.** يمكنك ختم ملف WAV والتحقق منه ومعرفة الثواني التي جرى تعديلها بالضبط، في [العرض التجريبي عبر المتصفح](https://vox-trust.github.io/demo/?lang=ar) أو عبر سطر الأوامر. الملفات المختومة تصمد **أمام النسخ المطابقة بت بت فقط**. بُنيت علامة مائية صوتية **تجريبية** تصمد أمام MP3 وAAC وOpus و[قيست](bench/results/2026-10-01-stdm-1/README.md)، لكنها تفشل مع ترميزات المكالمات الهاتفية ومع الضوضاء، ويمكن نسخ الأختام المضمّنة في الصوت إلى صوت آخر، لذلك **لا تُصدر أي حكم بعد**. لا تستخدم هذا لحماية أي شخص قبل أن تتم مراجعته.

## المشكلة

أصبح استنساخ صوت ما يتطلب ثواني من التسجيل فقط. لم يعد الاستماع كافيًا للتمييز بين صوت حقيقي وصوت مستنسخ، وأجهزة الكشف تطارد هدفًا متحركًا: كلما تحسّن الكشف تحسّن التوليد.

## الفكرة

بدلًا من تخمين ما إذا كان الصوت مزيفًا، تحقق مما إذا كان الصوت الحقيقي قد **خُتم**.

1. **الختم.** يوقّع جهاز المتحدث الصوت عند المصدر.
2. **التحقق.** يستطيع أي شخص يملك البروتوكول معرفة المفتاح الذي ختمه، ومتى، وأي مقاطع من الصوت جرى تعديلها.
3. **القرار.** تحوّل سياسة ثقة محلية النتيجة إلى واحدة من ثلاث نتائج.

| النتيجة | المعنى |
|---|---|
| **موثّق** | ختم صالح من مفتاح تثق به. |
| **بلا ختم** | لا يوجد ختم، من شخص لم يستخدم البروتوكول قط. نتيجة محايدة، و**ليست** "مزيفًا". |
| **إنذار** | الختم مكسور، أو صادر عن مفتاح لم تثبّته لهذا المتصل، أو مفقود من متصل *يختم دائمًا* (الوضع الصارم). |

وضعان: **الدائرة** (أشخاص يعرفون بعضهم، سر مشترك) و**العلني** (مؤسسات وشخصيات عامة، مفاتيح Ed25519 يثبّتها المتحققون).

## جرّبه في 30 ثانية

**في المتصفح** (لا يُرفع أي شيء؛ تعمل نواة Rust بصيغة WebAssembly): افتح [العرض التجريبي](https://vox-trust.github.io/demo/?lang=ar)، واضغط *ختم*، ثم حاول الغش بالأزرار وراقب المتحقق وهو يكشف كل محاولة.

**في سطر الأوامر** (توجد برامج جاهزة لأنظمة Linux وmacOS وWindows، مع المجاميع الاختبارية وشهادات البناء، في [صفحة الإصدارات](https://github.com/vox-trust/vox-trust/releases/latest)؛ أو ابنِه من الشيفرة المصدرية):

</div>

```sh
cargo install --locked --git https://github.com/vox-trust/vox-trust --tag v0.3.0 vox-trust-cli

vox-trust keygen me.key                                  # asks for a passphrase
vox-trust seal speech.wav sealed.wav --mode circle --key me.key
vox-trust verify sealed.wav --circle-key me.key          # exit code 0 = verified
```

<div dir="rtl">

يحمي `keygen` المفتاح بعبارة مرور (Argon2id وXChaCha20-Poly1305) ويطلبها كلما استُخدم المفتاح. في السكربتات استخدم `--passphrase-file` أو المتغير `VOX_TRUST_PASSPHRASE`؛ ويكتب `--plain` مفتاحًا غير محمي.

</div>

<div dir="rtl">

عدّل عينة واحدة من `sealed.wav` ثم تحقق مجددًا: يصبح رمز الخروج 3 ويُطبع المقطع المعدَّل. رموز الخروج: 0 موثّق، 1 بلا ختم، 2 تحذير، 3 إنذار.

**كمكتبة Rust:**

</div>

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

<div dir="rtl">

## ما الذي لا يمثّله هذا

- هو **ليس** كشفًا للتزييف العميق (deepfake) و**ليس** بصمة صوتية حيوية.
- هو **لا** يثبت أن المتحدث إنسان أو أنه من يدّعي أنه هو. إنه يثبت أن *مفتاحًا* ما ختم الصوت. المفتاح المسروق أو الجهاز المخترَق ينتجان أختامًا صالحة.
- "بلا ختم" **لا** تعني "مزيف": فالضغط وكبت الضوضاء قد يمحوان العلامة المائية.
- وضع الملفات **لا** يصمد أمام MP3 أو AAC أو إعادة أخذ العينات أو إعادة التسجيل. وهذا يتطلب الحامل (carrier)، وهو غير موجود بعد.
- لا يحمي من مهاجم يتحكم في جهاز المرسِل.

المهاجمون والادعاءات ونقاط الضعف التي اكتُشفت حتى الآن (بما فيها واحدة لا تزال بلا حل) موجودة في [نموذج التهديدات](spec/THREAT-MODEL.md).

## كيف يُختبر

| الطبقة | ما الذي تفحصه | التشغيل |
|---|---|---|
| اختبارات الوحدة والتكامل في Rust | تخطيط الختم، وHMAC وEd25519 (مقابل متجهات RFC 4231 وRFC 8032)، وتحليل WAV، والبيانات الوصفية (manifests)، والعبث، والسياسة، وواجهة C وأداة سطر الأوامر | `cargo test --workspace` |
| متجهات الاختبار المنشورة | أختام وبيانات وصفية مطابقة بايتًا ببايت في [`spec/test-vectors/`](spec/test-vectors) | مشمولة أعلاه |
| تحقق متقاطع بلغة Python | يعيد `tools/check_vectors.py` بناء كل متجه من نص المواصفة (المكتبة القياسية، إضافة إلى `cryptography` لـ Ed25519) | `pip install cryptography && python3 tools/check_vectors.py --strict` |
| WebAssembly من الطرف إلى الطرف | تعيد الوحدة المُصرَّفة إنتاج المتجهات بايتًا ببايت، وتتعامل مع المدخلات العشوائية، ولا تسرّب الذاكرة | `scripts/build-web.sh && node --test tests/node/wasm.test.mjs` |
| متصفح حقيقي | صفحة العرض التجريبي في Chromium وFirefox وWebKit بلا واجهة: الختم، وست هجمات، وتثبيت المفتاح العام، وإمكانية الوصول، وعرض الهاتف، والوضع الداكن | `BROWSER=firefox node tests/browser/demo.mjs` |
| الاختبار العشوائي (fuzzing) | كل محلل لمدخلات غير موثوقة، مع ثوابت تُفحص (الصوت المختوم يُتحقق منه دائمًا، وأي بت عينة مقلوب يُكتشف في المقطع الصحيح، ونص الاقتران له صيغة واحدة فقط) | `cd fuzz && cargo +nightly fuzz run verify_wav` |
| التغطية | نسبة أسطر Rust التي تنفذها الاختبارات؛ يفشل CI تحت 96 % | `scripts/coverage.sh` |
| اختبار الطفرات | يغيّر العوامل والقيم المُعادة في `vox-trust-core` وشيفرة ملفات المفاتيح، واحدًا تلو الآخر، ويتأكد من فشل اختبار ما؛ والتغييرات القليلة التي لا يمكن أن تغيّر السلوك مدرجة مع السبب في `.cargo/mutants.toml` | `cargo mutants -p vox-trust-core` |
| الاعتماديات | الثغرات المعروفة، والتراخيص، والمصادر | `cargo deny check` |

يشغّل CI اختبارات Rust على Linux وmacOS وWindows.

كُتب تحقق Python على يد المؤلف نفسه، لذا فهو تحقق متقاطع وليس تنفيذًا مستقلًا. [التنفيذ المستقل هو ما لا تزال المواصفة بحاجة إليه.](docs/ROADMAP.md)

## موقعه بين الأعمال القائمة

يبني Vox Trust على الأعمال القائمة ويكمّلها بدلًا من أن يحل محلها:

- بيانات اعتماد المحتوى [C2PA](https://spec.c2pa.org/): إثبات المصدر للملفات، مع العلامات المائية بوصفها "soft bindings" (روابط مرنة).
- العلامات المائية الصوتية مثل [AudioSeal](https://github.com/facebookresearch/audioseal) و[WavMark](https://github.com/wavmark/wavmark)، التي يمكن أن تعمل حاملات.
- أبحاث عن إثبات مصدر الكلام بالمفتاح العام، مثل [MerkleSpeech](https://arxiv.org/abs/2602.10166).
- التطبيقات التي تدوّر كلمات السر العائلية (مثل Trust Onion) وشركات الكشف تعالج الاحتيال نفسه من جهات أخرى.
- يشهد STIR/SHAKEN على *رقم* المتصل، لا على الصوت.

## بنية المستودع

</div>

```
spec/        المواصفة، نموذج التهديدات، متجهات الاختبار
crates/
  vox-trust-core/   تخطيط الختم، وضعا الدائرة والعلني، وضع الملفات، السياسة، مساعدات إعادة التشغيل، نص الاقتران
  vox-trust-wasm/   النواة كوحدة WebAssembly (واجهة C بسيطة، بلا imports)
  vox-trust-cli/    أداة سطر الأوامر `vox-trust`
web/         العرض التجريبي في المتصفح (منشور على vox-trust.github.io/demo/)
tests/       اختبارات Node (WebAssembly) واختبارات المتصفح
tools/       التحقق المستقل من المتجهات
fuzz/        أهداف الاختبار العشوائي (cargo-fuzz) ومدخلات البذور
scripts/     مساعدات البناء والنشر
docs/        خارطة الطريق
```

<div dir="rtl">

## المساهمة والأمان والترخيص

- اقرأ [CONTRIBUTING.md](CONTRIBUTING.md) ([ملخص بالعربية](docs/i18n/CONTRIBUTING.ar.md)). أنفع ما يمكن فعله الآن هو التمحيص: هاجم [نموذج التهديدات](spec/THREAT-MODEL.md) أو اكتب تنفيذًا مستقلًا انطلاقًا من [المواصفة](spec/SPEC.md).
- أبلغ عن الثغرات **بشكل خاص**: راجع [SECURITY.md](SECURITY.md) ([ملخص بالعربية](docs/i18n/SECURITY.ar.md)).
- الشيفرة: [Apache-2.0](LICENSE). نص المواصفة: CC BY 4.0، راجع [spec/LICENSE.md](spec/LICENSE.md). التغييرات: [CHANGELOG.md](CHANGELOG.md).
- الحوكمة واستخدام الاسم: [GOVERNANCE.md](GOVERNANCE.md)، [TRADEMARKS.md](TRADEMARKS.md). للاقتباس: [CITATION.cff](CITATION.cff).
- المشرف على الصيانة: [Roger Oliveira](https://www.linkedin.com/in/rogeroliveira/) · [@rogeroliveira84](https://github.com/rogeroliveira84).

</div>
