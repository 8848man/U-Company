import type { Agent, Appearance } from '../types';
import PixelCharacter from './PixelCharacter';
import { HAIRS, OUTFITS, SKIN_LABELS, appearanceFor, packAvailable, randomAppearance } from './metrocity';

/** Choose a teammate's Office look from the MetroCity layers; saved with the member settings. */
export function AppearanceEditor({ draft, onDraft }: { draft: Agent; onDraft: (agent: Agent) => void }) {
  if (!packAvailable) return null;
  const look = appearanceFor(draft);
  const hidden = OUTFITS.find(choice => choice.id === look.outfit)?.hidesHair;
  const set = (change: Partial<Appearance>) => onDraft({ ...draft, appearance: { ...look, ...change } });
  return <fieldset className="appearance-editor">
    <legend>캐릭터 모습</legend>
    <div className="appearance-preview"><PixelCharacter index={0} working appearance={look} /></div>
    <div className="appearance-fields">
      <label>피부<select value={look.skin} onChange={event => set({ skin: Number(event.target.value) })}>{SKIN_LABELS.map((label, index) => <option key={label} value={index}>{label}</option>)}</select></label>
      <label>옷<select value={look.outfit} onChange={event => set({ outfit: event.target.value })}>{OUTFITS.map(choice => <option key={choice.id} value={choice.id}>{choice.label}</option>)}</select></label>
      <label>머리<select value={look.hair} disabled={hidden} onChange={event => set({ hair: event.target.value })}><option value="none">없음</option>{HAIRS.map(choice => <option key={choice.id} value={choice.id}>{choice.label}</option>)}</select></label>
      <label className="appearance-check"><input type="checkbox" checked={Boolean(look.beard)} onChange={event => set({ beard: event.target.checked })} />수염</label>
      <div className="appearance-actions"><button type="button" className="secondary" onClick={() => onDraft({ ...draft, appearance: randomAppearance() })}>무작위</button>{draft.appearance && <button type="button" className="secondary" onClick={() => { const next = { ...draft }; delete next.appearance; onDraft(next); }}>기본 모습</button>}</div>
    </div>
    <small>{hidden ? '모자가 있는 옷은 머리를 가립니다. ' : ''}변경 저장을 누르면 Office에 반영됩니다. 그림: MetroCity by JIK-A-4 (CC0).</small>
  </fieldset>;
}
