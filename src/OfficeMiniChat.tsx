import { useEffect, useRef, useState } from 'react';
import { miniChatBlock, miniChatTurns } from './officeWindowState';
import type { Agent, Goal, GoalTurn } from './types';

type Props = {
  goal: Goal | null;
  agent: Agent | undefined;
  turns: GoalTurn[];
  /** Read-only question to this teammate about the goal (`ask_goal`). It never approves, redirects or starts work. */
  onAsk: (agentId: string, question: string) => Promise<boolean>;
  onClose: () => void;
};

/** A short session with one teammate inside the Office window (ADR-0047). The same turns appear in the goal conversation. */
export function OfficeMiniChat({ goal, agent, turns, onAsk, onClose }: Props) {
  const [text, setText] = useState('');
  const [sending, setSending] = useState(false);
  const listRef = useRef<HTMLOListElement>(null);
  const inputRef = useRef<HTMLTextAreaElement>(null);
  const block = miniChatBlock(goal, agent);
  const recent = agent ? miniChatTurns(turns, agent.id) : [];
  const waiting = recent.some(turn => turn.status === 'pending');
  const length = text.trim().length;

  useEffect(() => { inputRef.current?.focus(); }, [agent?.id]);
  useEffect(() => { listRef.current?.scrollTo({ top: listRef.current.scrollHeight }); }, [recent.length, recent[recent.length - 1]?.status]);

  async function send() {
    if (!agent || block || sending || length < 4) return;
    setSending(true);
    const ok = await onAsk(agent.id, text.trim());
    setSending(false);
    if (ok) setText('');
  }

  return <section className="office-mini-chat" aria-label={`${agent?.name ?? '팀원'}과 대화`} onKeyDown={event => { if (event.key === 'Escape') { event.stopPropagation(); onClose(); } }}>
    <header>
      <span className="office-mini-avatar" aria-hidden="true">{(agent?.name ?? '?').slice(0, 1)}</span>
      <div><strong>{agent?.name ?? '팀원'}</strong><small>{agent?.role ?? ''}</small></div>
      <button type="button" aria-label="대화 닫기" title="닫기" onClick={onClose}>×</button>
    </header>
    <ol ref={listRef} className="office-mini-turns" aria-live="polite">
      {recent.map(turn => <li key={turn.id}>
        <p className="is-ceo">{turn.question}</p>
        <p className={`is-agent is-${turn.status}`}>{turn.status === 'pending' ? '답을 쓰고 있어요…' : turn.status === 'answered' ? turn.answer : turn.status === 'failed' ? `답하지 못했어요. ${turn.answer}`.trim() : '대화가 중단됐어요.'}</p>
      </li>)}
      {!recent.length && !block && <li className="office-mini-empty">이 목표에 대해 무엇이든 물어보세요. 질문만 하며, 작업 방향은 바뀌지 않아요.</li>}
    </ol>
    {block ? <p className="office-mini-block" role="note">{block}</p> : <form onSubmit={event => { event.preventDefault(); void send(); }}>
      <textarea ref={inputRef} rows={2} value={text} maxLength={4000} placeholder={`${agent?.name ?? '팀원'}에게 질문`} aria-label={`${agent?.name ?? '팀원'}에게 질문`} onChange={event => setText(event.target.value)} onKeyDown={event => { if (event.key === 'Enter' && !event.shiftKey && !event.nativeEvent.isComposing) { event.preventDefault(); void send(); } }} />
      <button type="submit" className="primary" disabled={sending || waiting || length < 4} title={waiting ? '앞 질문의 답을 기다리는 중' : length < 4 ? '4자 이상 적어 주세요' : '보내기'}>보내기</button>
    </form>}
  </section>;
}
