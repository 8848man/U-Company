import { describe, expect, it } from 'vitest';
import { HAIRS, OUTFITS, appearanceFor, frameColumn, generatedAppearance, layersFor, packAvailable, randomAppearance, validAppearance } from './metrocity';

describe('MetroCity runtime characters (ADR-0040)', () => {
  it('finds the raw pack layers bundled from assets/characters', () => {
    expect(packAvailable).toBe(true);
  });
  it('stacks body, outfit, beard and hair, and lets a hat costume hide the hair', () => {
    expect(layersFor({ skin: 3, outfit: 'shirt-2', hair: 'hairs-3' })).toEqual([{ layer: 'body', row: 3 }, { layer: 'shirts', row: 2 }, { layer: 'hairs', row: 3 }]);
    expect(layersFor({ skin: 0, outfit: 'suit-0', hair: 'hairs-1', beard: true })).toEqual([{ layer: 'body', row: 0 }, { layer: 'suit', row: 0 }, { layer: 'hair2', row: 1 }]);
    expect(layersFor({ skin: 1, outfit: 'outfit-5', hair: 'none' })).toEqual([{ layer: 'body', row: 1 }, { layer: 'outfit5', row: 0 }]);
  });
  it('uses a saved look, then seeded defaults, then a stable pick from the id', () => {
    const saved = { skin: 5, outfit: 'suit-2', hair: 'hair2-4', beard: true };
    expect(appearanceFor({ id: 'cto', appearance: saved })).toEqual(saved);
    expect(appearanceFor({ id: 'cto' })).toEqual({ skin: 0, outfit: 'shirt-0', hair: 'hairs-0', beard: false });
    expect(appearanceFor({ id: 'cto', appearance: { skin: 9, outfit: 'x', hair: 'y' } }).outfit).toBe('shirt-0');
    expect(appearanceFor({ id: 'ext-123' })).toEqual(generatedAppearance('ext-123'));
    expect(validAppearance(generatedAppearance('anyone'))).toBe(true);
  });
  it('only produces valid random looks and known choice ids', () => {
    let seed = 1;
    const random = () => { seed = (seed * 16807) % 2147483647; return seed / 2147483647; };
    for (let i = 0; i < 50; i++) expect(validAppearance(randomAppearance(random))).toBe(true);
    expect(new Set([...OUTFITS, ...HAIRS].map(choice => choice.id)).size).toBe(OUTFITS.length + HAIRS.length);
  });
  it('stands still when idle or motion is reduced and walks in place while working', () => {
    expect(frameColumn(false, 4, false)).toBe(0);
    expect([0, 1, 6].map(tick => frameColumn(true, tick, false))).toEqual([0, 1, 0]);
    expect(frameColumn(true, 3, true)).toBe(0);
  });
});
