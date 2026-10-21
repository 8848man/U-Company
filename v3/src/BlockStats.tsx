import type { BlockStat } from '../../src/types';

/** ADR-0054: where this project's goals stopped, so the CEO (and the next round of fixes) can see the most common stop. */
export function BlockStats({ projectName, stats }: { projectName: string; stats: BlockStat[] | null }) {
  const total = stats?.reduce((sum, stat) => sum + stat.count, 0) ?? 0;
  return <section className="card v3-block-stats" aria-label="멈춘 이유">
    <h2>목표가 멈춘 이유</h2>
    <p className="muted">{projectName}에서 목표가 멈춘 기록이에요. 가장 많이 멈춘 곳부터 보여요.</p>
    {stats === null ? <p className="muted">불러오는 중…</p> : total === 0 ? <p>아직 멈춘 목표가 없어요.</p> :
      <ul>{stats.slice(0, 5).map(stat => <li key={stat.kind}><span>{stat.label}</span><b>{stat.count}번</b><div className="v3-block-bar" style={{ width: `${Math.max(6, Math.round(stat.count / total * 100))}%` }} /></li>)}</ul>}
  </section>;
}
