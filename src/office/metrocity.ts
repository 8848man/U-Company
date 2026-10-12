/// <reference types="vite/client" />
/**
 * Office characters composed at runtime from the layered MetroCity pack by JIK-A-4 (CC0 1.0,
 * ADR-0040). The raw pack stays as downloaded in `assets/characters/`; the app stacks
 * shadow → body → outfit → hair (→ beard) on a canvas per appearance, so no build step or script
 * is needed. Every layer row is 24 frames of 32×32: down 0–5, right 6–11, up 12–17, left 18–23.
 * The picture is only a skin: every state shown still comes from saved engine state.
 */
import type { Agent, Appearance } from '../types';

export const FRAME = 32;
export const COLUMNS = 24;
export const SCALE = 3;
/** Idle: standing, facing down. Working: the facing-down walk cycle (the pack has no typing pose). */
export const IDLE_COLUMN = 0;
export const WORK_COLUMNS = [0, 1, 2, 3, 4, 5];
export const WORK_FPS = 6;

const files = import.meta.glob('../../assets/characters/**/*.png', { eager: true, query: '?url', import: 'default' }) as Record<string, string>;
const find = (suffix: string) => Object.entries(files).find(([path]) => path.endsWith(suffix))?.[1];

type Layer = 'body' | 'shadow' | 'hairs' | 'hair2' | 'shirts' | 'suit' | 'outfit1' | 'outfit2' | 'outfit3' | 'outfit4' | 'outfit5' | 'outfit6';
const layerUrls: Record<Layer, string | undefined> = {
  body: find('MetroCity/CharacterModel/Character Model.png'),
  shadow: find('MetroCity/CharacterModel/Shadow.png'),
  hairs: find('MetroCity/Hair/Hairs.png'),
  hair2: find('MetroCity 2.0/Hair.png'),
  shirts: find('MetroCity 2.0/Suit1.png'),
  suit: find('MetroCity 2.0/Suit.png'),
  outfit1: find('MetroCity/Outfits/Outfit1.png'),
  outfit2: find('MetroCity/Outfits/Outfit2.png'),
  outfit3: find('MetroCity/Outfits/Outfit3.png'),
  outfit4: find('MetroCity/Outfits/Outfit4.png'),
  outfit5: find('MetroCity/Outfits/Outfit5.png'),
  outfit6: find('MetroCity/Outfits/Outfit6.png'),
};

/** True when the base layers of the pack are bundled; otherwise Office keeps the built-in figures. */
export const packAvailable = Boolean(layerUrls.body && layerUrls.shadow);

export type Choice = { id: string; label: string; layer: Layer; row: number; hidesHair?: boolean };
export const SKIN_LABELS = ['밝은 1', '밝은 2', '밝은 3', '어두운 1', '밝은 4', '어두운 2'];
export const OUTFITS: Choice[] = [
  { id: 'shirt-0', label: '흰 셔츠·검정 바지', layer: 'shirts', row: 0 },
  { id: 'shirt-1', label: '분홍 셔츠', layer: 'shirts', row: 1 },
  { id: 'shirt-2', label: '파랑 작업복', layer: 'shirts', row: 2 },
  { id: 'shirt-3', label: '하늘색 작업복', layer: 'shirts', row: 3 },
  { id: 'shirt-4', label: '자홍 셔츠', layer: 'shirts', row: 4 },
  { id: 'outfit-1', label: '흰 상의', layer: 'outfit1', row: 0 },
  { id: 'outfit-2', label: '보라 티셔츠', layer: 'outfit2', row: 0 },
  { id: 'outfit-3', label: '회색 반바지', layer: 'outfit3', row: 0 },
  { id: 'outfit-4', label: '주황 티셔츠', layer: 'outfit4', row: 0 },
  { id: 'outfit-5', label: '흰 티·청바지', layer: 'outfit5', row: 0 },
  { id: 'outfit-6', label: '빨강 티·청바지', layer: 'outfit6', row: 0 },
  { id: 'suit-0', label: '경찰 제복(모자)', layer: 'suit', row: 0, hidesHair: true },
  { id: 'suit-1', label: '빨강 제복(모자)', layer: 'suit', row: 1, hidesHair: true },
  { id: 'suit-2', label: '남색 정장', layer: 'suit', row: 2 },
  { id: 'suit-3', label: '주황 작업복(모자)', layer: 'suit', row: 3, hidesHair: true },
];
export const HAIRS: Choice[] = [
  ...['적갈색 짧은 머리', '은발 단발', '분홍 땋은 머리', '주황 짧은 머리', '주황 곱슬', '주황 긴 머리', '검정 땋은 머리', '검정 단발'].map((label, row) => ({ id: `hairs-${row}`, label, layer: 'hairs' as Layer, row })),
  { id: 'hair2-0', label: '금발 긴 머리', layer: 'hair2', row: 0 },
  { id: 'hair2-2', label: '금발 땋은 머리', layer: 'hair2', row: 2 },
  { id: 'hair2-3', label: '금발 짧은 머리', layer: 'hair2', row: 3 },
  { id: 'hair2-4', label: '금발 묶은 머리', layer: 'hair2', row: 4 },
];
const BEARD: Choice = { id: 'beard', label: '수염', layer: 'hair2', row: 1 };

/** Defaults for the seeded teammates; everyone else gets a stable pick from their id. */
const SEEDED: Record<string, Appearance> = {
  cto: { skin: 0, outfit: 'shirt-0', hair: 'hairs-0', beard: false },
  pm: { skin: 1, outfit: 'shirt-1', hair: 'hair2-0', beard: false },
  'developer-a': { skin: 3, outfit: 'shirt-2', hair: 'hairs-3', beard: false },
  'developer-b': { skin: 2, outfit: 'shirt-3', hair: 'hairs-6', beard: false },
  qa: { skin: 4, outfit: 'outfit-5', hair: 'hair2-3', beard: false },
};

export function validAppearance(value: unknown): value is Appearance {
  if (!value || typeof value !== 'object') return false;
  const item = value as Record<string, unknown>;
  return Number.isInteger(item.skin) && (item.skin as number) >= 0 && (item.skin as number) < SKIN_LABELS.length
    && OUTFITS.some(choice => choice.id === item.outfit)
    && (item.hair === 'none' || HAIRS.some(choice => choice.id === item.hair))
    && (item.beard === undefined || typeof item.beard === 'boolean');
}

function hash(text: string): number {
  let value = 2166136261;
  for (const char of text) value = Math.imul(value ^ char.charCodeAt(0), 16777619) >>> 0;
  return value;
}

/** A stable pick for an id, so a teammate keeps the same look across restarts. */
export function generatedAppearance(id: string): Appearance {
  const value = hash(id);
  const outfits = OUTFITS.filter(choice => !choice.hidesHair);
  return { skin: value % SKIN_LABELS.length, outfit: outfits[(value >>> 3) % outfits.length].id, hair: HAIRS[(value >>> 8) % HAIRS.length].id, beard: false };
}

export function appearanceFor(agent: Pick<Agent, 'id' | 'appearance'>): Appearance {
  return validAppearance(agent.appearance) ? agent.appearance : SEEDED[agent.id] ?? generatedAppearance(agent.id);
}

export function randomAppearance(random: () => number = Math.random): Appearance {
  const pick = <T,>(items: T[]) => items[Math.floor(random() * items.length) % items.length];
  return { skin: Math.floor(random() * SKIN_LABELS.length) % SKIN_LABELS.length, outfit: pick(OUTFITS).id, hair: pick(HAIRS).id, beard: random() < 0.15 };
}

/** Layers to stack, bottom to top. A hat costume hides the hair layer. */
export function layersFor(appearance: Appearance): { layer: Layer; row: number }[] {
  const outfit = OUTFITS.find(choice => choice.id === appearance.outfit) ?? OUTFITS[0];
  const hair = HAIRS.find(choice => choice.id === appearance.hair);
  const stack: { layer: Layer; row: number }[] = [{ layer: 'body', row: appearance.skin }, { layer: outfit.layer, row: outfit.row }];
  if (appearance.beard) stack.push({ layer: BEARD.layer, row: BEARD.row });
  if (hair && !outfit.hidesHair) stack.push({ layer: hair.layer, row: hair.row });
  return stack;
}

export const appearanceKey = (appearance: Appearance) => `${appearance.skin}|${appearance.outfit}|${appearance.hair}|${appearance.beard ? 1 : 0}`;

const images = new Map<string, Promise<HTMLImageElement>>();
function image(url: string): Promise<HTMLImageElement> {
  let pending = images.get(url);
  if (!pending) {
    pending = new Promise((resolve, reject) => { const item = new Image(); item.onload = () => resolve(item); item.onerror = reject; item.src = url; });
    images.set(url, pending);
  }
  return pending;
}

const sheets = new Map<string, Promise<HTMLCanvasElement>>();
/** One 768×32 sheet per appearance, composed once and cached. */
export function sheetFor(appearance: Appearance): Promise<HTMLCanvasElement> {
  const key = appearanceKey(appearance);
  let pending = sheets.get(key);
  if (!pending) {
    pending = (async () => {
      const canvas = document.createElement('canvas');
      canvas.width = FRAME * COLUMNS; canvas.height = FRAME;
      const context = canvas.getContext('2d')!;
      context.imageSmoothingEnabled = false;
      const shadow = await image(layerUrls.shadow!);
      for (let column = 0; column < COLUMNS; column++) context.drawImage(shadow, column * FRAME, 0);
      for (const { layer, row } of layersFor(appearance)) {
        const url = layerUrls[layer];
        if (!url) continue;
        const sheet = await image(url);
        context.drawImage(sheet, 0, row * FRAME, FRAME * COLUMNS, FRAME, 0, 0, FRAME * COLUMNS, FRAME);
      }
      return canvas;
    })();
    sheets.set(key, pending);
  }
  return pending;
}

/** The frame column to show: still when idle or when motion is reduced. */
export function frameColumn(working: boolean, tick: number, reducedMotion: boolean): number {
  if (!working || reducedMotion) return IDLE_COLUMN;
  return WORK_COLUMNS[tick % WORK_COLUMNS.length];
}
