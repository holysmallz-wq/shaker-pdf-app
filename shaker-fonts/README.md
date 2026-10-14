# SHAKER PDF fonts

Fonts embedded into SHAKER PDF at build time (`CRAFT_FONTS_DIR=shaker-fonts`), in the
[craft-fonts](https://github.com/storytold/craft-fonts) manifest format. All are under the
SIL Open Font License 1.1 (each folder has its `OFL.txt`); sources are listed in
`fonts/manifest.txt`.

Faces tagged `Pick` appear in the font list of Add text and are used when editing Arabic text.
To add a font: put the `.ttf` files and `OFL.txt` in a new folder under `fonts/`, add one line
per file to `fonts/manifest.txt` (with its SHA-256), and rebuild.
