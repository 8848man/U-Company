// Screen wording and choices for the SDD origin card (SI-03, ADR-0044).
// Pure functions: the backend owns detection and every file rule; this only decides what to say and offer.

export type OriginMode = 'bootstrap' | 'existing' | 'ask' | 'continue';
export type OriginAction = 'bootstrap' | 'add_section' | 'convert';
export type OriginChange = 'create' | 'append' | 'replace' | 'backup';

export type OriginSignal = { path: string; kind: string; workflow: boolean };
export type OriginStatus = {
  mode: OriginMode;
  scope: string;
  evidence: string[];
  followed: string | null;
  conflict: boolean;
  toolRules: string[];
  actions: OriginAction[];
  nextTask: string | null;
  scopes: { dir: string; mode: OriginMode; signals: OriginSignal[] }[];
  review: { path: string; reason: string }[];
  /** "이 폴더는 묻지 않기" (ADR-0045); absent on an older backend. */
  muted?: boolean;
};
export type OriginPlanChange = { path: string; change: OriginChange; before?: string; after?: string };
export type OriginPlan = { action: OriginAction; changes: OriginPlanChange[]; digest: string };
/** The CTO's work-kind judgement for one goal and the CEO's answer to the planning card's question (ADR-0045). */
export type GoalOrigin = { goalId: string; projectId: string; workKind: 'development' | 'other' | string; offered: boolean; mode: string; advice?: string | null; decision?: string | null; decidedAt?: string | null; createdAt: string };

export type OriginMessage = { headline: string; detail: string | null; notes: string[] };
export type OriginChoice = { action: OriginAction; label: string; primary: boolean };

const TOOL_NAMES: Record<string, string> = { 'CLAUDE.md': 'Claude', 'GEMINI.md': 'Gemini', '.cursorrules': 'Cursor', '.github/copilot-instructions.md': 'Copilot' };

function toolName(path: string): string {
  if (path.startsWith('.cursor/')) return 'Cursor';
  return TOOL_NAMES[path] ?? '';
}

/** The control line every state shows on the first depth. */
export const ORIGIN_CONTROL_LINE = '확인 전에는 프로젝트 파일을 바꾸지 않아요 · AI 호출 없음';

export function originMessage(status: OriginStatus): OriginMessage {
  const notes: string[] = [];
  if (status.mode === 'continue') {
    return { headline: 'U 컴퍼니 구조로 작업 중이에요. 이어서 작업할게요.', detail: status.nextTask ? `다음 작업: ${status.nextTask.replace(/^T-\d+\s+/, '')}` : '아직 진행 중인 기능이 없어요. 첫 기능을 추가하면 다음 작업이 여기에 보여요.', notes };
  }
  if (status.mode === 'bootstrap') {
    return { headline: 'AI 에이전트들이 함께 일하기 쉽도록 프로젝트 구조를 잡아드릴까요?', detail: 'AGENTS.md, CLAUDE.md와 .ucompany/sdd 폴더가 생겨요. 기존 파일은 바뀌지 않아요.', notes };
  }
  if (status.mode === 'ask') {
    return { headline: '코딩 규칙은 있지만 작업 절차(스펙 → 계획 → 작업)는 없어요. 기존 규칙은 그대로 두고 절차를 추가할까요?', detail: `찾은 파일: ${status.evidence.join(', ')}`, notes };
  }
  if (status.scope !== '.') {
    return { headline: `하위 폴더(${status.scope})에서 작업 규칙을 찾았어요. 그 폴더의 작업은 그 규칙을 따를게요.`, detail: '프로젝트 맨 위에는 아무것도 만들지 않아요.', notes };
  }
  const followed = status.followed ?? status.evidence[0] ?? '';
  const tool = toolName(followed);
  const headline = tool ? `${tool} 전용 작업 규칙(${followed})을 찾았어요. 이 규칙대로 작업할게요.` : `이 프로젝트의 작업 규칙(${followed})을 찾았어요. 이 규칙대로 작업할게요.`;
  if (status.conflict) notes.push(`${status.toolRules.join(', ')}에도 규칙이 있어요. 더 범용인 AGENTS.md를 따를게요.`);
  const others = status.evidence.filter(path => path !== followed && !(status.conflict && status.toolRules.includes(path)));
  if (others.length) notes.push(`함께 찾은 파일: ${others.join(', ')}`);
  notes.push('찾은 파일은 아래 "외부 명세 가져오기"에서 골라 미리볼 수 있어요.');
  return { headline, detail: 'U 컴퍼니 구조로 바꾸면 기존 파일은 .ucompany/legacy 폴더에 먼저 백업돼요.', notes };
}

/** Control line on the planning card: the two answers never stand in for each other. */
export const PLANNING_CONTROL_LINE = '구조 답과 계획 승인은 따로예요 · 답하지 않아도 계획은 그대로 진행돼요';

/** Lead sentence of the planning card's structure section. */
export function planningLead(origin: GoalOrigin): string {
  return origin.mode === 'bootstrap'
    ? 'CTO가 이 목표를 개발 작업으로 봤어요. 다른 AI 툴도 이어서 일할 수 있게 프로젝트 구조를 잡을까요?'
    : 'CTO가 이 목표를 개발 작업으로 봤어요. 이 프로젝트의 작업 규칙을 함께 확인해 주세요.';
}

/** What the CEO already answered, in plain words; null while the question is open. */
export function answerLabel(decision: string | null | undefined): string | null {
  if (!decision) return null;
  if (decision.startsWith('applied:')) return `구조 질문에 답했어요: ${LABELS[decision.slice(8) as OriginAction] ?? '적용'}`;
  if (decision === 'muted') return '이 폴더는 앞으로 구조를 묻지 않아요. 설정 › 프로젝트 명세에서 되돌릴 수 있어요.';
  if (decision === 'declined') return '이번 목표에서는 구조를 잡지 않기로 했어요.';
  return null;
}

/** One line when the planned roles lack a usable AI, from the engine's squad shortfall. */
export function connectionGap(shortfall: string[] | undefined): string | null {
  return shortfall && shortfall.length > 0 ? `계획에 필요한 역할에 쓸 수 있는 AI가 부족해요. AI를 연결할까요? (${shortfall.join(' · ')})` : null;
}

/** Entry files whose text changes, with before and after, for the confirmation's comparison view. */
export function changePreviews(plan: OriginPlan): { path: string; label: string; before: string; after: string }[] {
  return plan.changes
    .filter(item => (item.change === 'append' || item.change === 'replace') && item.before !== undefined && item.after !== undefined)
    .map(item => ({ path: item.path, label: CHANGE_LABELS[item.change], before: item.before ?? '', after: item.after ?? '' }));
}

const LABELS: Record<OriginAction, string> = { bootstrap: '구조 잡기', add_section: '절차 추가', convert: 'U 컴퍼니 구조로 바꾸기' };

/** At most three choices; exactly one is primary. `convert` is never the recommended one. */
export function originChoices(status: OriginStatus): OriginChoice[] {
  const actions = status.actions.filter((action): action is OriginAction => action in LABELS);
  const recommended = actions.find(action => action !== 'convert');
  return actions.map(action => ({ action, label: LABELS[action], primary: recommended ? action === recommended : false }));
}

export function confirmTitle(action: OriginAction): string {
  return { bootstrap: '프로젝트 구조를 잡을까요?', add_section: '작업 절차를 추가할까요?', convert: 'U 컴퍼니 구조로 바꿀까요?' }[action];
}

export function confirmBody(action: OriginAction): string {
  return {
    bootstrap: '아래 파일을 새로 만들어요. 다른 파일은 건드리지 않아요.',
    add_section: '기존 규칙 파일 끝에 U 컴퍼니 구간을 덧붙여요. 기존 내용은 그대로 앞에 남아요.',
    convert: '기존 규칙 파일을 먼저 백업한 뒤 바꿔요. AGENTS.md의 기존 내용은 U 컴퍼니 구간 아래에 그대로 남고, 다른 툴의 규칙 파일은 백업만 하고 그대로 둬요. 지우는 파일은 없어요.',
  }[action];
}

const CHANGE_LABELS: Record<OriginChange, string> = { backup: '백업', create: '새로 만듦', append: '끝에 덧붙임', replace: '바꿈' };
const CHANGE_ORDER: OriginChange[] = ['backup', 'create', 'append', 'replace'];

export function changeLabel(change: OriginChange): string {
  return CHANGE_LABELS[change];
}

/** One line such as "백업 2 · 새로 만듦 7 · 바꿈 2", in the order the backend applies them. */
export function planSummary(plan: OriginPlan): string {
  return CHANGE_ORDER
    .map(change => [change, plan.changes.filter(item => item.change === change).length] as const)
    .filter(([, count]) => count > 0)
    .map(([change, count]) => `${CHANGE_LABELS[change]} ${count}`)
    .join(' · ');
}

/** Changes sorted by apply order, so backups are listed first. `.gitkeep` placeholders are hidden. */
export function planRows(plan: OriginPlan): { path: string; label: string }[] {
  return [...plan.changes]
    .filter(item => !item.path.endsWith('/.gitkeep'))
    .sort((a, b) => CHANGE_ORDER.indexOf(a.change) - CHANGE_ORDER.indexOf(b.change))
    .map(item => ({ path: item.path, label: CHANGE_LABELS[item.change] }));
}
