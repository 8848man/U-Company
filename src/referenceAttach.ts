// ADR-0049: reference material in the composer. Pure rules here; the browser work is in ReferenceTray.
import type { Reference } from './types';

/** Pasted text longer than this becomes a reference file instead of goal text (the backend checks it too). */
export const PASTE_FILE_OVER = 4_000;
export const TEXT_FILE_MAX_BYTES = 1_048_576;
export const IMAGE_INPUT_MAX_BYTES = 20 * 1_048_576;
export const GOAL_MAX_IMAGES = 20;

/**
 * The size the model sees without shrinking it again (Claude vision: 28×28 px patches, standard tier
 * up to 1,568 px on the long edge and 1,568 patches; `detail` uses the high tier, 2,576 px and 4,784).
 * Never enlarges.
 */
export function targetSize(width: number, height: number, detail = false): { width: number; height: number } {
  const edge = detail ? 2576 : 1568;
  const patches = detail ? 4784 : 1568;
  let scale = Math.min(1, edge / Math.max(width, height));
  const fits = (s: number) => Math.ceil((width * s) / 28) * Math.ceil((height * s) / 28) <= patches;
  while (scale > 0.01 && !fits(scale)) scale *= 0.98;
  return { width: Math.max(1, Math.floor(width * scale)), height: Math.max(1, Math.floor(height * scale)) };
}

/** Screens and diagrams (few colours) keep a higher WebP quality so text in them stays sharp. */
export function webpQuality(distinctColours: number): number { return distinctColours <= 512 ? 0.95 : 0.85; }

export function isTextFile(name: string): boolean { return /\.(md|markdown|txt)$/i.test(name); }
export function isImageFile(name: string, type: string): boolean { return /^image\/(png|jpeg|webp|gif)$/i.test(type) || /\.(png|jpe?g|webp|gif)$/i.test(name); }

/** What a chip says about one reference. */
export function chipLabel(reference: Reference): string {
  const size = reference.kind === 'image' ? `${reference.width}×${reference.height}` : `${reference.chars.toLocaleString('ko-KR')}자`;
  const name = reference.source === 'paste' ? '붙여넣은 글' : reference.originalName;
  return `${reference.label ? `${reference.label} · ` : ''}${name} · ${size}`;
}

export function curationLabel(reference: Reference): string {
  if (reference.kind === 'image') return reference.caption ? '설명 있음' : '설명 없음 · CTO가 채워요';
  if (reference.curation === 'pending') return '자료 정리 담당이 정리하는 중';
  if (reference.curation === 'done') return reference.sections.length ? `정리됨 · ${reference.sections.length}절` : '정리됨';
  if (reference.curation === 'skipped' || reference.curation === 'failed') return reference.curationNote || '규칙으로 정리';
  const background = reference.sections.filter(section => section.background).length;
  if (reference.role === 'requirement' && reference.sections.length) return `요구사항 ${reference.sections.length - background}절${background ? ` · 배경 ${background}절` : ''}`;
  return reference.kind === 'text' && reference.chars > 20_000 ? '제목 기준으로 나눔' : '규칙으로 정리';
}

/** Only the given staged ids that still exist, in the order they were attached. */
export function stagedReferences(all: Reference[] | undefined, ids: string[]): Reference[] {
  return ids.map(id => all?.find(item => item.id === id && item.status === 'staged')).filter((item): item is Reference => Boolean(item));
}

/** A goal's material: its own and any inherited from the earlier versions it replaced. */
export function goalReferences(goalId: string, snapshot: { references?: Reference[]; goalGuidance?: { goalId: string; replacementGoalId: string | null }[] }): Reference[] {
  const chain = [goalId];
  for (let current = goalId; ;) {
    const previous = snapshot.goalGuidance?.find(item => item.replacementGoalId === current)?.goalId;
    if (!previous || chain.includes(previous)) break;
    chain.push(previous); current = previous;
  }
  const order = (reference: Reference) => [chain.indexOf(reference.goalId ?? ''), Number(reference.label.replace(/^R/, '')) || 0] as const;
  return (snapshot.references ?? []).filter(item => item.goalId && chain.includes(item.goalId)).sort((a, b) => order(a)[0] - order(b)[0] || order(a)[1] - order(b)[1]);
}

/**
 * ADR-0049: how many requirement sections the plan's tasks name as evidence (the plan card line).
 * Background sections are not counted; exclusions with reasons are listed in the approval scope.
 */
export function requirementCoverage(refs: Reference[], tasks: { evidence?: string[] }[]): { total: number; covered: number } | null {
  const targets = refs.filter(item => item.role === 'requirement' && item.kind === 'text').flatMap(item => item.sections.length ? item.sections.filter(section => !section.background).map(section => section.file) : [item.path]);
  if (!targets.length) return null;
  const evidence = tasks.flatMap(task => task.evidence ?? []).map(path => path.toLowerCase());
  return { total: targets.length, covered: targets.filter(file => evidence.includes(file.toLowerCase())).length };
}
