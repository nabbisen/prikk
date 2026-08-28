# Prikk logo assets

Every file here is derived from the single reference image `prikk-logo-base.png` by cropping and
resizing only. **Nothing is redrawn or recoloured**, so the mark, the wordmark, and the balance
between them are the original's own.

| file | what it is |
|---|---|
| `prikk-header-1040.png`, `-520.png` | README header — mark + `Prikk`, descriptor and body text removed, margins evened |
| `prikk-mark-512.png`, `-256.png` | the symbol alone, square, for an avatar or a social image |

How they were produced from the reference image (base coordinates):

```
header : paint out the descriptor at 742,476 .. 1425,548, then crop 1304x528+166+112
mark   : crop 528x528+186+112
```

Palette: sage `#7B927D`, warm tan `#DCC4A7`, terracotta `#D97C5F`, cream `#FBF4EC`.

## Known limits

- **These are raster images with no alpha** — they carry the reference image's cream background, as
  the original does. There is no vector version.
- **The mark is too detailed for a 32px favicon**; a favicon needs a simplified mark that does not
  exist yet.

## Copies

`prikk-header-520.png` is duplicated at `docs/src/assets/prikk-header-520.png` for the mdbook site,
because mdbook only serves files under `docs/src/`. **Update both together**, or the documentation
site will keep showing the old logo.
