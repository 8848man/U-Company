import { useEffect, useRef, useState } from 'react';
import type { Appearance } from '../types';
import { FRAME, SCALE, WORK_FPS, appearanceKey, frameColumn, packAvailable, sheetFor } from './metrocity';

type Props = { index: number; working: boolean; appearance?: Appearance };

/** A MetroCity character composed at runtime from the pack layers (ADR-0040), drawn crisp. */
function SpriteCharacter({ working, appearance }: { working: boolean; appearance: Appearance }) {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const [tick, setTick] = useState(0);
  const [sheet, setSheet] = useState<HTMLCanvasElement | null>(null);
  const reduced = typeof window !== 'undefined' && window.matchMedia?.('(prefers-reduced-motion: reduce)').matches;
  const key = appearanceKey(appearance);
  useEffect(() => {
    let live = true;
    sheetFor(appearance).then(result => { if (live) setSheet(result); }).catch(() => { if (live) setSheet(null); });
    return () => { live = false; };
    // eslint-disable-next-line react-hooks/exhaustive-deps -- the key captures the appearance
  }, [key]);
  useEffect(() => {
    if (!working || reduced) return;
    const timer = window.setInterval(() => setTick(value => value + 1), 1000 / WORK_FPS);
    return () => window.clearInterval(timer);
  }, [working, reduced]);
  useEffect(() => {
    const canvas = canvasRef.current; const context = canvas?.getContext('2d');
    if (!canvas || !context) return;
    context.imageSmoothingEnabled = false;
    context.clearRect(0, 0, canvas.width, canvas.height);
    if (sheet) context.drawImage(sheet, frameColumn(working, tick, Boolean(reduced)) * FRAME, 0, FRAME, FRAME, 0, 0, canvas.width, canvas.height);
  }, [sheet, working, tick, reduced]);
  return <canvas ref={canvasRef} className={`sprite-character${working ? ' is-working' : ''}`} width={FRAME * SCALE} height={FRAME * SCALE} aria-hidden="true" />;
}

const palettes = [
  { hair: '#352a31', skin: '#e5b68c', jacket: '#7da7a8', pants: '#344c62' },
  { hair: '#765340', skin: '#f0c99e', jacket: '#a487bd', pants: '#39455b' },
  { hair: '#1f2833', skin: '#bb8669', jacket: '#d1a865', pants: '#3e4a49' },
  { hair: '#482b31', skin: '#e4aa86', jacket: '#8eae72', pants: '#48516c' },
  { hair: '#d1a274', skin: '#cf9073', jacket: '#ab8c92', pants: '#334e59' },
];

/** MetroCity character when the pack is bundled; otherwise the built-in pixel figure. */
export default function PixelCharacter({ index, working, appearance }: Props) {
  if (packAvailable && appearance) return <SpriteCharacter working={working} appearance={appearance} />;
  const color = palettes[index % palettes.length];
  return <svg className={`pixel-character${working ? ' is-working' : ''}`} viewBox="0 0 32 42" shapeRendering="crispEdges" aria-hidden="true">
    <rect x="8" y="2" width="16" height="4" fill={color.hair} /><rect x="6" y="6" width="20" height="10" fill={color.hair} />
    <rect x="8" y="10" width="16" height="13" fill={color.skin} /><rect x="10" y="5" width="12" height="5" fill={color.hair} />
    <rect x="10" y="15" width="3" height="2" fill="#30303a" /><rect x="19" y="15" width="3" height="2" fill="#30303a" />
    <rect x="13" y="20" width="6" height="2" fill="#a96961" />
    <rect x="11" y="23" width="10" height="3" fill={color.skin} />
    <rect x="7" y="26" width="18" height="10" fill={color.jacket} /><rect x="3" y="27" width="4" height="8" fill={color.jacket} /><rect x="25" y="27" width="4" height="8" fill={color.jacket} />
    <rect x="3" y="35" width="4" height="3" fill={color.skin} /><rect x="25" y="35" width="4" height="3" fill={color.skin} />
    <rect x="8" y="36" width="7" height="5" fill={color.pants} /><rect x="17" y="36" width="7" height="5" fill={color.pants} />
    <rect x="7" y="40" width="8" height="2" fill="#242b31" /><rect x="17" y="40" width="8" height="2" fill="#242b31" />
    <rect x="14" y="27" width="4" height="5" fill="#d5ead0" />
  </svg>;
}
