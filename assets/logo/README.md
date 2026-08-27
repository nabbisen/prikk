# Prikk logo assets

`prikk-mark` (symbol), `prikk-wordmark` (Prikk), `prikk-lockup` (mark + wordmark).
Proportions follow the original reference image: wordmark optical height ~0.51 of the mark,
gap ~0.18 of the mark width, wordmark vertically centred on the mark.

SVG is the master; the PNGs are derived from it:

```
rsvg-convert -w 512  prikk-mark.svg   -o prikk-mark-512.png
rsvg-convert -w 128  prikk-mark.svg   -o prikk-mark-128.png
rsvg-convert -w 1040 prikk-lockup.svg -o prikk-lockup-1040.png
rsvg-convert -w 520  prikk-lockup.svg -o prikk-lockup-520.png
```

Palette: sage `#7B927D`, warm tan `#DCC4A7` (blocks, frame and connector paths),
terracotta `#D97C5F`, cream `#FBF4EC` (connector node centres).

The transparent assets are legible on both light and dark backgrounds, so no per-theme variant
is needed. No SVG here depends on a font — all letterforms are outlined.
