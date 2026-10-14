<p align="center"><img src="assets/brand/shaker-256.png" width="96" alt="SHAKER PDF"></p>

<h1 align="center">SHAKER PDF</h1>

<p align="center" dir="rtl">
محرر PDF لويندوز يدعم اللغة العربية واتجاهها: تعديل النصوص العربية الموجودة في الملف، إضافة نصوص
بخطوط عربية وإنجليزية، دمج وتقسيم وتوقيع وأختام وتعليقات ونماذج.
</p>

<p align="center">A Windows PDF editor with first-class Arabic: edit existing Arabic text, add text in
Arabic and English fonts, combine, split, sign, stamp, comment and fill forms.</p>

---

<div dir="rtl">

## المميزات

- **تعديل النص العربي الموجود في الملف**: اضغط على الفقرة وعدّل. الحروف بتتشبك صح، والأرقام
  والكلام الإنجليزي جوه السطر العربي بيفضلوا في مكانهم الصحيح، والسطر بيفضل محاذي من اليمين.
  البرنامج بيحاول يستخدم نفس خط الملف الأصلي لو موجود عنده (مثلاً Tajawal أو Cairo)، والعنوان
  البولد بيفضل بولد.
- **إضافة نص** بأي خط من الخطوط المدمجة:
  - عربي: Cairo، Tajawal، Almarai، IBM Plex Sans Arabic، Noto Kufi Arabic، Noto Naskh Arabic،
    Amiri، Scheherazade New (عادي وعريض).
  - إنجليزي: Roboto، Open Sans، Lato، Montserrat، بالإضافة لـ Helvetica و Times و Courier.
- **واجهة عربي**: البرنامج بيفتح بالعربي، ولوحة الأدوات على اليمين والمعلومات على الشمال.
  تقدر تغيّر اللغة من الإعدادات.
- كل أدوات PdfCraft الأصلية: دمج وتقسيم واستخراج صفحات، تدوير وترتيب، تعليقات وتظليل،
  أختام، تعبئة وتوقيع النماذج، حماية بكلمة سر، OCR، تصدير لـ Word/HTML، وغيرها.

## التنزيل

من صفحة **Releases** في الريبو نزّل `SHAKER-PDF-…-windows-x64.msi` وثبّته، أو النسخة
المحمولة `…-portable.zip` اللي بتشتغل من غير تثبيت.

كل مرة يتعمل push على `main` بتتبني نسخة جديدة تلقائيًا وتلاقيها في تبويب **Actions** تحت
**Artifacts**. ولما تعمل tag زي `v1.0.0` بتتنشر كـ Release.

## حدود حالية

- تعديل فقرة عربية بيعيد رسمها كنص مضاف في نفس المكان. ممكن تعدّلها تاني عادي.
- النص العربي المايل (Rotated) لسه مش مدعوم في التعديل.
- الإيطاليك العربي مش مدعوم (الخطوط العربية مالهاش إيطاليك).

</div>

## Building

```sh
git clone <this repo> && cd shaker-pdf
CRAFT_FONTS_DIR=$PWD/shaker-fonts cargo run -p pdfcraft --release -- file.pdf
```

`shaker-fonts/` holds the embedded fonts (all SIL OFL 1.1; see `shaker-fonts/fonts/manifest.txt`).
The Windows installer is built by `.github/workflows/shaker-windows.yml`
(`packaging/windows/package.ps1`, WiX v5).

## What changed from PdfCraft

- `crates/fonts`: any embedded face can shape text (`ShapedScript::Face`, `pickable_fonts`, `shape_face`).
- `crates/edit/src/arabic_edit.rs`: editing existing Arabic paragraphs (redrawn as shaped text in place,
  row by row), presentation-form normalisation, matching the source font.
- `crates/edit/src/text.rs`: right-to-left chunks drawn from right to left join into one line; lam-alef
  and other ligature glyphs read in the right order.
- `crates/edit/src/added.rs`: added text can use a chosen embedded font (`AddedText::face`, saved in the PDF).
- `crates/ui-egui`: font picker lists the embedded fonts; Arabic interface by default with mirrored panels;
  SHAKER PDF branding.

## Credits and licence

SHAKER PDF is a fork of **[PdfCraft](https://github.com/storytold/pdfcraft)** and keeps its licence:
MIT OR Apache-2.0 (`LICENSE-MIT`, `LICENSE-APACHE`, `NOTICE`). PdfCraft's third-party credits are in
`ATTRIBUTION.md`. The ArtCraft names and logos are not used. Fonts: SIL Open Font License 1.1, by their
respective authors (Google Fonts sources listed in the manifest).
