import { describe, expect, it } from 'vitest';
import { chipLabel, curationLabel, goalReferences, isImageFile, isTextFile, requirementCoverage, stagedReferences, targetSize, webpQuality } from './referenceAttach';
import type { Reference } from './types';

const ref = (over: Partial<Reference>): Reference => ({ id: 'r', projectId: 'p', goalId: null, label: '', kind: 'text', source: 'paste', title: 't', originalName: '붙여넣은 글', originalBytes: 1, sha256: 'x', chars: 12340, width: 0, height: 0, bytes: 1, path: '', sections: [], summary: '', summarySource: 'rule', caption: '', tags: [], relations: [], curation: 'none', curationNote: '', detail: false, createdAt: 't', status: 'staged', ...over });

describe('reference material in the composer (ADR-0049)', () => {
  it('sizes images to what the model sees without shrinking again, never enlarging', () => {
    const wide = targetSize(3840, 2160);
    expect(Math.max(wide.width, wide.height)).toBeLessThanOrEqual(1568);
    expect(Math.ceil(wide.width / 28) * Math.ceil(wide.height / 28)).toBeLessThanOrEqual(1568);
    expect(wide.width).toBeGreaterThan(1400);
    const square = targetSize(4000, 4000);
    expect(Math.ceil(square.width / 28) * Math.ceil(square.height / 28)).toBeLessThanOrEqual(1568);
    expect(targetSize(800, 600)).toEqual({ width: 800, height: 600 });
    const fine = targetSize(3840, 2160, true);
    expect(Math.max(fine.width, fine.height)).toBeLessThanOrEqual(2576);
    expect(Math.ceil(fine.width / 28) * Math.ceil(fine.height / 28)).toBeLessThanOrEqual(4784);
  });
  it('keeps screens sharper than photos and accepts only the supported files', () => {
    expect(webpQuality(40)).toBeGreaterThan(webpQuality(50_000));
    expect(isTextFile('요구사항.MD') && isTextFile('a.txt') && !isTextFile('a.pdf')).toBe(true);
    expect(isImageFile('shot.png', '') && isImageFile('x', 'image/jpeg') && !isImageFile('x.heic', 'image/heic')).toBe(true);
  });
  it('labels chips and curation honestly', () => {
    expect(chipLabel(ref({}))).toBe('붙여넣은 글 · 12,340자');
    expect(chipLabel(ref({ kind: 'image', source: 'image', originalName: 'login.png', width: 1456, height: 819, label: 'R3' }))).toBe('R3 · login.png · 1456×819');
    expect(curationLabel(ref({ curation: 'pending' }))).toContain('정리하는 중');
    expect(curationLabel(ref({ curation: 'skipped', curationNote: '자료 정리 담당에 AI 연결이 없어 규칙으로 정리했어요' }))).toContain('AI 연결');
    expect(curationLabel(ref({ chars: 30_000 }))).toBe('제목 기준으로 나눔');
    expect(curationLabel(ref({ kind: 'image', caption: '' }))).toContain('CTO');
  });
  it('lists staged ids that still exist and a goal’s own plus inherited material', () => {
    const all = [ref({ id: 'a' }), ref({ id: 'b', status: 'ready', goalId: 'g' })];
    expect(stagedReferences(all, ['b', 'a', 'gone']).map(item => item.id)).toEqual(['a']);
    const refs = [ref({ id: 'old', goalId: 'g0', label: 'R1', status: 'ready' }), ref({ id: 'new2', goalId: 'g1', label: 'R2', status: 'ready' }), ref({ id: 'new1', goalId: 'g1', label: 'R1', status: 'ready' }), ref({ id: 'other', goalId: 'x', label: 'R1', status: 'ready' })];
    expect(goalReferences('g1', { references: refs, goalGuidance: [{ goalId: 'g0', replacementGoalId: 'g1' }] }).map(item => item.id)).toEqual(['new1', 'new2', 'old']);
  });
  it('counts requirement sections the plan names as evidence, skipping background', () => {
    const sec = (n: number, background = false) => ({ n, title: `절${n}`, file: `work-references/x/R1/0${n}.md`, chars: 10, firstLine: '', summary: '', background });
    const prd = ref({ role: 'requirement', status: 'ready', label: 'R1', sections: [sec(1, true), sec(2), sec(3)] });
    expect(curationLabel(prd)).toBe('요구사항 2절 · 배경 1절');
    expect(requirementCoverage([prd], [{ evidence: ['work-references/x/R1/02.md'] }])).toEqual({ total: 2, covered: 1 });
    expect(requirementCoverage([ref({ role: 'reference' })], [])).toBeNull();
    expect(requirementCoverage([ref({ role: 'requirement', path: 'work-references/x/R2.md' })], [{ evidence: ['WORK-REFERENCES/x/R2.md'] }])).toEqual({ total: 1, covered: 1 });
  });
});
