# Metro City free top-down character pack (JIK-A-4)

- Source: <https://jik-a-4.itch.io/metrocity-free-topdown-character-pack>
- License shown on the page (checked 2026-10-10): Creative Commons Zero v1.0 Universal (CC0 1.0). The author states the pack may be used as wished, including in commercial projects; credit is appreciated but not required.
- Credit: character art by JIK-A-4 (Metro City).
- Used for: optional Office character sprites in `src/office/assets/characters/` (ADR-0040). Pixel Agents (MIT, `PIXEL_AGENTS_LICENSE.txt`) credits the same pack for its characters; its modified sprites and its furniture/floor/wall assets are not used.
- Raw pack in the repository (downloaded by the CEO, 2026-10-11): `assets/characters/MetroCity (1)/MetroCity/` (version 1: `CharacterModel/Character Model.png` 6 skin tones, `Shadow.png`, `Hair/Hairs.png` 8 styles and single-frame previews, `Outfits/Outfit1-6.png`) and `assets/characters/MetroCity 2.0 (2)/MetroCity 2.0/` (update 2.0: `Hair.png` 5 rows, `Suit.png` 4 costumes with hats, `Suit1.png` 5 shirt-and-trouser sets). Every layer uses 32×32 frames, 24 per row: columns 0–5 face down, 6–11 right, 12–17 up, 18–23 left (6-frame walk cycle each).
- Use in the app: `src/office/metrocity.ts` bundles these files unchanged and composes characters at runtime (shadow → body → outfit → beard → hair). No modified copies are stored in the repository.
