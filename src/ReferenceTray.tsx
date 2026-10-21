import { useEffect, useRef, useState, type ReactNode } from 'react';
import { chipLabel, curationLabel, GOAL_MAX_IMAGES, IMAGE_INPUT_MAX_BYTES, isImageFile, isTextFile, TEXT_FILE_MAX_BYTES, targetSize, webpQuality } from './referenceAttach';
import type { Reference } from './types';

export type StageImage = { name: string; dataBase64: string; originalBytes: number; originalSha256: string; caption: string; detail: boolean };

type Props = {
  references: Reference[];
  onStageText: (name: string, text: string, source: 'paste' | 'file') => Promise<Reference | undefined>;
  onStageImage: (image: StageImage) => Promise<Reference | undefined>;
  onDiscard: (id: string) => void;
  onCaption: (id: string, caption: string) => void;
  /** `요구사항` (the PM reads it; the plan must cover it) or `참고` (read on demand). */
  onRole: (id: string, role: 'requirement' | 'reference') => void;
  /** The curator is off by default; on, it groups headless long text and marks background in requirements. */
  curatorEnabled: boolean; onCurator: (enabled: boolean) => void;
  /** Puts a pasted text back into the goal box (the chip's undo). */
  onInsert: (text: string) => void;
  onError: (message: string) => void;
};

async function sha256Hex(buffer: ArrayBuffer): Promise<string> {
  const digest = await crypto.subtle.digest('SHA-256', buffer);
  return Array.from(new Uint8Array(digest)).map(byte => byte.toString(16).padStart(2, '0')).join('');
}

function base64(bytes: Uint8Array): string {
  let binary = '';
  for (let index = 0; index < bytes.length; index += 0x8000) binary += String.fromCharCode(...bytes.subarray(index, index + 0x8000));
  return btoa(binary);
}

/** Resizes to what the model sees, re-encodes as WebP (dropping EXIF and location), first frame only. */
export async function formatImage(file: File, detail: boolean): Promise<{ dataBase64: string; sha: string; blob: Blob }> {
  if (file.size > IMAGE_INPUT_MAX_BYTES) throw new Error('이미지는 20MB까지 첨부할 수 있어요');
  const original = await file.arrayBuffer();
  let bitmap: ImageBitmap;
  try { bitmap = await createImageBitmap(new Blob([original], { type: file.type }), { imageOrientation: 'from-image' }); }
  catch { throw new Error(`${file.name}을(를) 열 수 없어요. PNG·JPEG·WebP·GIF로 바꿔 첨부해 주세요(HEIC는 지원하지 않아요)`); }
  const size = targetSize(bitmap.width, bitmap.height, detail);
  const canvas = document.createElement('canvas');
  canvas.width = size.width; canvas.height = size.height;
  const context = canvas.getContext('2d');
  if (!context) throw new Error('이미지를 변환할 수 없어요');
  context.drawImage(bitmap, 0, 0, size.width, size.height);
  bitmap.close();
  const sample = context.getImageData(0, 0, size.width, size.height).data;
  const colours = new Set<number>();
  const step = Math.max(4, Math.floor(sample.length / 4 / 20_000) * 4);
  for (let index = 0; index < sample.length && colours.size <= 512; index += step) colours.add((sample[index] << 16) | (sample[index + 1] << 8) | sample[index + 2]);
  const blob = await new Promise<Blob | null>(resolve => canvas.toBlob(resolve, 'image/webp', webpQuality(colours.size)));
  if (!blob || blob.type !== 'image/webp') throw new Error('이 화면에서는 WebP로 변환할 수 없어요');
  return { dataBase64: base64(new Uint8Array(await blob.arrayBuffer())), sha: await sha256Hex(original), blob };
}

/**
 * ADR-0049: the composer's attachments. Returns the tray and the two entry points the goal box uses
 * (a long paste, dropped files). Files are created by the app from fixed rules after the goal is sent.
 */
export function useReferenceTray({ references, onStageText, onStageImage, onDiscard, onCaption, onRole, curatorEnabled, onCurator, onInsert, onError }: Props): { tray: ReactNode; paste: (text: string) => Promise<void>; addFiles: (files: File[]) => Promise<void> } {
  const input = useRef<HTMLInputElement>(null);
  // Files being prepared show as placeholder boxes until the backend has them.
  const [preparing, setPreparing] = useState<{ key: string; name: string }[]>([]);
  const [detail, setDetail] = useState(false);
  const [open, setOpen] = useState<string | null>(null);
  // Pasted text is kept here so the box can put it back; the backend copy is the one that is sent.
  const [pasted, setPasted] = useState<Record<string, string>>({});
  // Image previews from the formatted WebP (the same picture the team will see).
  const [thumbs, setThumbs] = useState<Record<string, string>>({});
  const thumbsRef = useRef(thumbs);
  thumbsRef.current = thumbs;
  useEffect(() => () => { Object.values(thumbsRef.current).forEach(url => URL.revokeObjectURL(url)); }, []);
  const images = references.filter(item => item.kind === 'image').length;

  async function prepare<T>(name: string, work: () => Promise<T>): Promise<T | undefined> {
    const key = `${name}-${Math.random().toString(36).slice(2)}`;
    setPreparing(items => [...items, { key, name }]);
    try { return await work(); }
    catch (error) { onError(error instanceof Error ? error.message : String(error)); return undefined; }
    finally { setPreparing(items => items.filter(item => item.key !== key)); }
  }

  async function addFiles(files: File[]) {
    let imageCount = images;
    for (const file of files) {
      await prepare(file.name, async () => {
        if (isTextFile(file.name)) {
          if (file.size > TEXT_FILE_MAX_BYTES) throw new Error(`${file.name}: 글 자료는 1MB까지 첨부할 수 있어요`);
          await onStageText(file.name, await file.text(), 'file');
        } else if (isImageFile(file.name, file.type)) {
          if (imageCount >= GOAL_MAX_IMAGES) throw new Error(`이미지는 목표당 ${GOAL_MAX_IMAGES}장까지 첨부할 수 있어요`);
          const formatted = await formatImage(file, detail);
          const staged = await onStageImage({ name: file.name, dataBase64: formatted.dataBase64, originalBytes: file.size, originalSha256: formatted.sha, caption: '', detail });
          if (staged) { imageCount += 1; const url = URL.createObjectURL(formatted.blob); setThumbs(current => ({ ...current, [staged.id]: url })); }
        } else {
          throw new Error(`${file.name}: md·txt 글과 PNG·JPEG·WebP·GIF 이미지만 첨부할 수 있어요`);
        }
      });
    }
  }

  async function paste(text: string) {
    await prepare('붙여넣은 글', async () => { const staged = await onStageText('', text, 'paste'); if (staged) setPasted(current => ({ ...current, [staged.id]: text })); });
  }

  function remove(id: string) {
    if (thumbs[id]) { URL.revokeObjectURL(thumbs[id]); setThumbs(current => { const next = { ...current }; delete next[id]; return next; }); }
    onDiscard(id);
  }

  const kindBadge = (reference: Reference) => reference.kind === 'image' ? 'IMG' : reference.source === 'paste' ? 'TXT' : (reference.originalName.split('.').pop() ?? 'TXT').slice(0, 4).toUpperCase();
  const size = (reference: Reference) => reference.kind === 'image' ? (reference.width ? `${reference.width}×${reference.height}` : '이미지') : `${reference.chars.toLocaleString('ko-KR')}자`;
  const shown = references.find(item => item.id === open) ?? null;
  const tray = <div className="reference-tray" aria-label="참조 자료">
    {(references.length > 0 || preparing.length > 0) && <ul className="reference-boxes">
      {references.map(reference => <li key={reference.id} className={`reference-box is-${reference.kind}${reference.curation === 'pending' ? ' is-pending' : ''}`}>
        <button type="button" className="reference-box-main" onClick={() => setOpen(reference.id)} title={`${reference.source === 'paste' ? '붙여넣은 글' : reference.originalName} · 자세히 보기`}>
          <span className="reference-box-preview" aria-hidden="true">{thumbs[reference.id] ? <img src={thumbs[reference.id]} alt="" /> : <b>{kindBadge(reference)}</b>}</span>
          <span className="reference-box-name">{reference.source === 'paste' ? '붙여넣은 글' : reference.originalName}</span>
          <small>{size(reference)} · {curationLabel(reference)}</small>
        </button>
        {reference.kind === 'text' && <button type="button" className={`reference-box-role${reference.role === 'requirement' ? ' is-requirement' : ''}`} aria-pressed={reference.role === 'requirement'} title={reference.role === 'requirement' ? 'PM이 읽고 계획이 모든 절을 반영해야 해요. 누르면 참고 자료로 바꿔요' : '필요할 때만 열어요. 누르면 요구사항으로 바꿔요'} onClick={() => onRole(reference.id, reference.role === 'requirement' ? 'reference' : 'requirement')}>{reference.role === 'requirement' ? '요구사항' : '참고'}</button>}
        <button type="button" className="reference-box-remove" aria-label={`${chipLabel(reference)} 지우기`} title="지우기" onClick={() => remove(reference.id)}>×</button>
      </li>)}
      {preparing.map(item => <li key={item.key} className="reference-box is-preparing" aria-live="polite"><span className="reference-box-main"><span className="reference-box-preview" aria-hidden="true"><i /></span><span className="reference-box-name">{item.name}</span><small>준비하는 중…</small></span></li>)}
    </ul>}
    <div className="reference-actions">
      <button type="button" className="text-button" onClick={() => input.current?.click()}>＋ 자료 첨부</button>
      <label className="reference-detail" title="글자가 빽빽한 화면 캡처는 더 크게 저장해요. 토큰이 최대 약 3배 들어요"><input type="checkbox" checked={detail} onChange={event => setDetail(event.target.checked)} /> 이미지 세밀하게</label>
      <label className="reference-detail" title="켜면 제목 없는 긴 글을 주제별로 묶고, 요구사항 자료에서 배경 절을 표시해요. 자료마다 AI 호출이 한 번 더 들어요"><input type="checkbox" checked={curatorEnabled} onChange={event => onCurator(event.target.checked)} /> 자료 정리 담당 사용</label>
      <small>md·txt·이미지를 끌어다 놓거나 4,000자 넘게 붙여 넣으면 자료가 돼요 · 자료는 연결된 AI로 전송돼요</small>
      <input ref={input} type="file" hidden style={{ display: 'none' }} multiple accept=".md,.markdown,.txt,image/png,image/jpeg,image/webp,image/gif" onChange={event => { const files = Array.from(event.target.files ?? []); event.target.value = ''; void addFiles(files); }} />
    </div>
    {shown && <div className="dialog-backdrop" onClick={() => setOpen(null)} onKeyDown={event => { if (event.key === 'Escape') { event.stopPropagation(); setOpen(null); } }}>
      <section className="reason-dialog reference-dialog" role="dialog" aria-modal="true" aria-label={chipLabel(shown)} onClick={event => event.stopPropagation()}>
        <button type="button" className="text-button gc-todo-close" autoFocus onClick={() => setOpen(null)}>닫기</button>
        <span className="card-kicker">{shown.kind === 'image' ? '이미지 자료' : shown.role === 'requirement' ? '요구사항 자료' : '참고 자료'}</span>
        <h3>{shown.title}</h3>
        {thumbs[shown.id] && <img className="reference-dialog-image" src={thumbs[shown.id]} alt={shown.caption || shown.title} />}
        <p className="muted">{chipLabel(shown)} · {curationLabel(shown)}</p>
        {shown.kind === 'image' && <label className="reference-caption">무엇을 보라는 이미지인가요? (선택, 200자)<input defaultValue={shown.caption} maxLength={200} placeholder="예: 로그인 실패 때 나오는 오류 화면" onBlur={event => { if (event.target.value !== shown.caption) onCaption(shown.id, event.target.value); }} /></label>}
        {(shown.summary || shown.caption) && <p><strong>요약</strong> {shown.caption || shown.summary}</p>}
        {shown.tags.length > 0 && <p className="muted">태그: {shown.tags.join(', ')}</p>}
        {shown.relations.length > 0 && <p className="muted">관계: {shown.relations.map(relation => `${relation.kind === 'illustrates' ? '설명함' : '관련'} → ${references.find(item => item.id === relation.target)?.title ?? '다른 자료'}`).join(' · ')}</p>}
        <p className="muted">보내면 앱이 work-references/ 폴더에 정해진 규칙으로 저장하고 인덱스를 만들어요. AI는 인덱스를 보고 필요한 자료만 열어요.</p>
        <div className="chat-decision-actions">
          {pasted[shown.id] && <button type="button" className="secondary" onClick={() => { onInsert(pasted[shown.id]); remove(shown.id); setOpen(null); }}>본문에 넣기</button>}
          <button type="button" className="secondary" onClick={() => { remove(shown.id); setOpen(null); }}>지우기</button>
        </div>
      </section>
    </div>}
  </div>;
  return { tray, paste, addFiles };
}
