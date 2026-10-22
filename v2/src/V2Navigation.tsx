import { useState } from 'react';
import type { Project } from '../../src/types';

export type V2Tab = 'dashboard' | 'board' | 'office' | 'usage' | 'settings';
export type SettingsSection = 'projects' | 'connections' | 'team' | 'spec' | 'notifications' | 'backup';
export const labels: Record<V2Tab, string> = { dashboard: '홈', board: '업무', office: 'Office', usage: '토큰', settings: '설정' };
const navigation: V2Tab[] = ['dashboard', 'board', 'office', 'usage', 'settings'];
const icons: Record<V2Tab, string> = { dashboard: '◫', board: '▤', office: '▦', usage: '◉', settings: '⚙' };

/** Line icons for the compact v3 rail: chat (home), kanban (work), people (office), gauge (tokens), gear (settings). */
const PATHS: Record<V2Tab | 'project', string> = {
  dashboard: 'M4 5h16v10H9l-5 4z M8 9h8 M8 12h5',
  board: 'M4 4h4v16H4z M10 4h4v10h-4z M16 4h4v13h-4z',
  office: 'M8.5 10a3 3 0 1 0 0-6a3 3 0 0 0 0 6z M3 20c.5-3.5 3-5.5 5.5-5.5S13.5 16.5 14 20 M16.5 10.5a2.5 2.5 0 1 0 0-5 M17 14.5c2 .3 3.6 2.2 4 5',
  usage: 'M4 17a8 8 0 1 1 16 0 M12 17l4-5 M7 17h.01 M17 17h.01',
  settings: 'M12 15a3 3 0 1 0 0-6a3 3 0 0 0 0 6z M19 12a7 7 0 0 0-.1-1.2l2-1.6-2-3.4-2.4 1a7 7 0 0 0-2-1.2L14 3h-4l-.5 2.6a7 7 0 0 0-2 1.2l-2.4-1-2 3.4 2 1.6A7 7 0 0 0 5 12c0 .4 0 .8.1 1.2l-2 1.6 2 3.4 2.4-1a7 7 0 0 0 2 1.2L10 21h4l.5-2.6a7 7 0 0 0 2-1.2l2.4 1 2-3.4-2-1.6c.1-.4.1-.8.1-1.2z',
  project: 'M3 7a2 2 0 0 1 2-2h4l2 2h8a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z',
};
function RailIcon({ name }: { name: V2Tab | 'project' }) {
  return <svg className="rail-icon" viewBox="0 0 24 24" width="22" height="22" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true"><path d={PATHS[name]} /></svg>;
}

/** v3 icon rail: fixed narrow width; hovering or focusing an item shows its card to the right without widening the rail. */
function CompactSidebar({ tab, project, projects, onTab, onProject, onAddProject, version }: { version: string; tab: V2Tab; project: Project | null; projects: Project[]; onTab: (tab: V2Tab) => void; onProject: (project: Project) => void; onAddProject: () => void }) {
  const [open, setOpen] = useState(false);
  return <aside className="sidebar rail" aria-label="주 메뉴">
    <div className="rail-brand" title="UTOPIA · AI ORGANIZATION OS"><span className="brand-mark">✦</span></div>
    <div className="rail-item rail-project">
      <button type="button" className="rail-button" aria-expanded={open} aria-label={`현재 프로젝트: ${project?.name ?? '선택 안 됨'}. 바꾸기`} onClick={() => setOpen(value => !value)}><RailIcon name="project" /></button>
      {!open && <span className="rail-card" aria-hidden="true"><small>현재 프로젝트</small><strong>{project?.name ?? '프로젝트 선택'}</strong></span>}
      {open && <div className="v2-project-menu rail-menu" role="menu" aria-label="프로젝트 선택"><small className="rail-menu-head">현재 프로젝트 · {project?.name ?? '없음'}</small>{projects.map(item => <button type="button" role="menuitem" key={item.id} onClick={() => { setOpen(false); onProject(item); }}>{item.name}<small>{item.id === project?.id ? '현재 선택됨' : item.kind === 'git' ? 'Git 프로젝트' : '일반 폴더'}</small></button>)}<button type="button" role="menuitem" onClick={() => { setOpen(false); onAddProject(); }}>+ 폴더 추가</button></div>}
    </div>
    <nav className="rail-nav">{navigation.map(key => <div className="rail-item" key={key}>
      <button type="button" className={tab === key ? 'rail-button active' : 'rail-button'} aria-label={labels[key]} aria-current={tab === key ? 'page' : undefined} onClick={() => { setOpen(false); onTab(key); }}><RailIcon name={key} /></button>
      <span className="rail-card" aria-hidden="true"><strong>{labels[key]}</strong></span>
    </div>)}</nav>
    <div className="sidebar-spacer"/>
    <div className="rail-foot" title={`LOCAL FIRST · ${version}`}><span className="online-dot"/><small>{version}</small></div>
  </aside>;
}

export function V2Sidebar({ tab, project, projects, onTab, onProject, onAddProject, version = 'v2', compact = false }: { version?: string; compact?: boolean; tab: V2Tab; project: Project | null; projects: Project[]; onTab: (tab: V2Tab) => void; onProject: (project: Project) => void; onAddProject: () => void }) {
  const [open, setOpen] = useState(false);
  if (compact) return <CompactSidebar tab={tab} project={project} projects={projects} onTab={onTab} onProject={onProject} onAddProject={onAddProject} version={version} />;
  return <aside className="sidebar"><div className="brand"><span className="brand-mark">✦</span><div><strong>UTOPIA</strong><small>AI ORGANIZATION OS</small></div></div>
    <div className="sidebar-label">WORKSPACE</div><div className="v2-project-picker"><button type="button" className="workspace-switch" aria-expanded={open} onClick={() => setOpen(value => !value)}><small>현재 프로젝트</small><strong>{project?.name ?? '프로젝트 선택'}</strong><span>변경 ▾</span></button>{open && <div className="v2-project-menu" role="menu" aria-label="프로젝트 선택">{projects.map(item => <button type="button" role="menuitem" key={item.id} onClick={() => { setOpen(false); onProject(item); }}>{item.name}<small>{item.id === project?.id ? '현재 선택됨' : item.kind === 'git' ? 'Git 프로젝트' : '일반 폴더'}</small></button>)}<button type="button" role="menuitem" onClick={() => { setOpen(false); onAddProject(); }}>+ 폴더 추가</button></div>}</div>
    <nav aria-label="주 메뉴">{navigation.map(key => <button type="button" key={key} className={tab === key ? 'nav active' : 'nav'} onClick={() => { setOpen(false); onTab(key); }}><span className="nav-icon">{icons[key]}</span>{labels[key]}</button>)}</nav><div className="sidebar-spacer"/><div className="sidebar-foot"><span className="online-dot"/> LOCAL FIRST <span className="version">{version}</span></div></aside>;
}

export function V2SettingsTabs({ section, onSection }: { section: SettingsSection; onSection: (section: SettingsSection) => void }) {
  const sections: { id: SettingsSection; label: string }[] = [{ id: 'projects', label: '프로젝트' }, { id: 'connections', label: 'AI 연결' }, { id: 'team', label: '팀' }, { id: 'spec', label: '프로젝트 명세' }, { id: 'notifications', label: '알림' }, { id: 'backup', label: '백업 삭제' }];
  return <div className="v2-settings-tabs" role="tablist" aria-label="설정 항목">{sections.map(item => <button type="button" role="tab" aria-selected={section === item.id} className="secondary" key={item.id} onClick={() => onSection(item.id)}>{item.label}</button>)}</div>;
}
