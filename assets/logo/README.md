# Prikk logo assets

**`prikk-header-1040.png` / `-520.png`** — the README header. Cropped directly from the reference
image (`prikk-logo-base.png`), with the descriptor and body text removed and the margins evened up.
Nothing else is altered, so the mark, the wordmark and the balance between them are the original's own.

**`prikk-mark.svg`**, **`prikk-wordmark.svg`** — the vector rebuild, exactly as delivered in the design
bundle, unmodified. The PNGs beside them are rendered from the SVG:

```
rsvg-convert -w 512 prikk-mark.svg -o prikk-mark-512.png
rsvg-convert -w 128 prikk-mark.svg -o prikk-mark-128.png
```

Palette: sage `#7B927D`, warm tan `#DCC4A7`, terracotta `#D97C5F`, cream `#FBF4EC`.

## Known limits

- **There is no vector lockup.** The bundle's `prikk-lockup.svg` renders its descriptor as live text in
  Comfortaa, which overflows its own canvas and clips to "DISTRIBUTED VERSION CONTROL SY". Fixing it
  needs the font to outline the text, so it is not included here. The header PNG is used instead.
- **The vector mark is a tracing and differs from the reference image**: it encloses the blocks in a
  full rounded rectangle where the original has only a soft arc at the top right, and its connector
  paths are cream where the original's are warm tan.
- **The header PNG has no alpha** — it carries the reference image's cream background, as the original
  does.
