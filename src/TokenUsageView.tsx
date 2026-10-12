import { useState } from 'react';
import type { Snapshot, TokenUsageRecord } from './types';
import { goalTotals, memberTotals, shareOfProject, sortByUsage, sumUsage, usageInsights, type MemberTotals, type UsageSort } from './tokenAnalytics';
import { ContentPreview, ListDisclosure, useLimitedItems } from './ContentDensity';
import { goalTitle } from './progress';

const number = (value: number) => value.toLocaleString();
const percent = (value: number, total: number) => `${shareOfProject(value, total).toFixed(1)}%`;

function ShareBar({ value, projectTotal }: { value: number; projectTotal: number }) {
  const share = shareOfProject(value, projectTotal);
  return <div className="usage-share"><div className="usage-track" aria-hidden="true"><span style={{ width: `${share}%` }} /></div><strong>{percent(value, projectTotal)}</strong></div>;
}

function MemberRow({ member, projectTotal }: { member: MemberTotals; projectTotal: number }) {
  const modelWindow = useLimitedItems(member.models);
  return <div className="usage-member">
    <div className="usage-member-head"><strong>{member.name}</strong><span>{number(member.total)} 토큰</span></div>
    <ShareBar value={member.total} projectTotal={projectTotal} />
    <div className="usage-detail">입력 {number(member.input)} · 출력 {number(member.output)}{member.active ? ` · 실행 중 ${number(member.active)}` : ''}{member.estimated ? ` · 추정 ${number(member.estimated)}` : ''}</div>
    {modelWindow.visible.map(model => <div className="usage-model" key={model.key}><div className="usage-model-head"><span>{model.connection} · {model.model}</span><span>{number(model.total)} 토큰</span></div><ShareBar value={model.total} projectTotal={projectTotal} /><div className="usage-detail">입력 {number(model.input)} · 출력 {number(model.output)}{model.estimated ? ` · 추정 ${number(model.estimated)}` : ' · 제공자 보고'}{model.active ? ` · 실행 중 ${number(model.active)}` : ''}</div></div>)}
    <ListDisclosure label="모델" total={modelWindow.total} hidden={modelWindow.hidden} expanded={modelWindow.expanded} onToggle={modelWindow.toggle} />
  </div>;
}

function MemberList({ members, projectTotal, sort }: { members: MemberTotals[]; projectTotal: number; sort: UsageSort }) {
  const sorted = sortByUsage(members, sort, member => member.name);
  const memberWindow = useLimitedItems(sorted);
  if (!sorted.length) return <p className="usage-empty">아직 이 범위에서 토큰을 사용한 팀원이 없습니다.</p>;
  return <div className="usage-members">{memberWindow.visible.map(member => <MemberRow member={member} projectTotal={projectTotal} key={member.id} />)}<ListDisclosure label="팀원" total={memberWindow.total} hidden={memberWindow.hidden} expanded={memberWindow.expanded} onToggle={memberWindow.toggle} /></div>;
}

function GoalUsageList({ entries, projectTotal, memberSort }: { entries: ReturnType<typeof goalTotals>; projectTotal: number; memberSort: UsageSort }) {
  const goalWindow = useLimitedItems(entries);
  return <div className="usage-goals">{goalWindow.visible.map(entry => <details className="card usage-card" key={entry.goal.id}><summary><span><small>목표 · {entry.goal.status}</small><strong title={entry.goal.intent}>{goalTitle(entry.goal, 140)}</strong></span><span className="usage-card-total">{number(entry.total)} 토큰 <span className="usage-chevron">⌄</span></span></summary>
    {goalTitle(entry.goal, 140) !== entry.goal.intent && <ContentPreview><p className="usage-goal-original">{entry.goal.intent}</p></ContentPreview>}
    <ShareBar value={entry.total} projectTotal={projectTotal} />
    <div className="usage-detail">입력 {number(entry.input)} · 출력 {number(entry.output)} · 추정 {number(entry.estimated)} · 실행 중 {number(entry.active)}</div>
    <h3>팀원별 사용량</h3><MemberList members={entry.members} projectTotal={projectTotal} sort={memberSort} />
  </details>)}{!entries.length && <div className="empty-state compact">아직 이 프로젝트의 목표가 없습니다.</div>}<ListDisclosure label="목표" total={goalWindow.total} hidden={goalWindow.hidden} expanded={goalWindow.expanded} onToggle={goalWindow.toggle} /></div>;
}

export default function TokenUsageView({ snapshot }: { snapshot: Snapshot }) {
  const [goalSort, setGoalSort] = useState<UsageSort>('highest');
  const [memberSort, setMemberSort] = useState<UsageSort>('highest');
  const project = snapshot.project;
  if (!project) return <div className="page usage-page"><div className="page-title"><div className="eyebrow">TOKEN USAGE</div><h1>토큰 사용량</h1></div><div className="empty-state">프로젝트를 선택하면 목표와 팀원별 토큰 사용량이 여기에 표시됩니다.</div></div>;
  const rows: TokenUsageRecord[] = (snapshot.usage ?? []).filter(row => row.projectId === project.id);
  const totals = sumUsage(rows);
  const goals = (snapshot.goals ?? []).filter(goal => goal.projectId === project.id);
  const rankedGoals = sortByUsage(goalTotals(rows, goals, snapshot.agents), goalSort, entry => entry.goal.intent);
  const projectMembers = memberTotals(rows, snapshot.agents);
  return <div className="page usage-page">
    <div className="page-title"><div className="eyebrow">TOKEN USAGE</div><h1>토큰 사용량</h1><p>현재 프로젝트의 저장된 실행 기록입니다. 모든 비율은 프로젝트 총 토큰을 100%로 계산합니다. 비용이나 계정 한도를 뜻하지 않습니다.</p></div>
    <div className="usage-toolbar"><label>목표 정렬<select value={goalSort} onChange={event => setGoalSort(event.target.value as UsageSort)}><option value="highest">토큰 많은 순</option><option value="lowest">토큰 적은 순</option><option value="name">이름 순</option></select></label><label>팀원 정렬<select value={memberSort} onChange={event => setMemberSort(event.target.value as UsageSort)}><option value="highest">토큰 많은 순</option><option value="lowest">토큰 적은 순</option><option value="name">이름 순</option></select></label></div>
    <details className="card usage-card"><summary><span><small>현재 프로젝트</small><strong>{project.name}</strong></span><span className="usage-card-total">{number(totals.total)} 토큰 <span className="usage-chevron">⌄</span></span></summary>
      <ShareBar value={totals.total} projectTotal={totals.total} />
      <div className="usage-detail">입력 {number(totals.input)} · 출력 {number(totals.output)} · 추정 {number(totals.estimated)} · 실행 중 {number(totals.active)}</div>
      <h3>팀원별 사용량</h3><MemberList members={projectMembers} projectTotal={totals.total} sort={memberSort} />
    </details>
    <div className="section-head"><h2>현재 프로젝트의 목표</h2><span className="count">{goals.length} GOALS</span></div>
    <GoalUsageList entries={rankedGoals} projectTotal={totals.total} memberSort={memberSort} />
    <section className="card usage-insights"><div className="card-kicker">TOKEN INSIGHTS</div><h2>토큰 사용을 줄이는 힌트</h2>{usageInsights(rows, goals).map((insight, index) => <p key={index}>{insight}</p>)}<small>저장된 사용 기록을 바탕으로 한 점검 제안입니다. 모델별 비용이나 결과 품질을 예측하지 않습니다.</small></section>
  </div>;
}
