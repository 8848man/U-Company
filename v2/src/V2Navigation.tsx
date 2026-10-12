import { useState } from 'react';
import type { Project } from '../../src/types';

export type V2Tab = 'dashboard' | 'board' | 'office' | 'usage' | 'settings';
export type SettingsSection = 'projects' | 'connections' | 'team' | 'spec' | 'notifications' | 'backup';
export const labels: Record<V2Tab, string> = { dashboard: '홈', board: '업무', office: 'Office', usage: '토큰', settings: '설정' };
const navigation: V2Tab[] = ['dashboard', 'board', 'office', 'usage', 'settings'];
const icons: Record<V2Tab, string> = { dashboard: '◫', board: '▤', office: '▦', usage: '◉', settings: '⚙' };

export function V2Sidebar({ tab, project, projects, onTab, onProject, onAddProject, version = 'v2' }: { version?: string; tab: V2Tab; project: Project | null; projects: Project[]; onTab: (tab: V2Tab) => void; onProject: (project: Project) => void; onAddProject: () => void }) {
  const [open, setOpen] = useState(false);
  return <aside className="sidebar"><div className="brand"><span className="brand-mark">✦</span><div><strong>UTOPIA</strong><small>AI ORGANIZATION OS</small></div></div>
    <div className="sidebar-label">WORKSPACE</div><div className="v2-project-picker"><button type="button" className="workspace-switch" aria-expanded={open} onClick={() => setOpen(value => !value)}><small>현재 프로젝트</small><strong>{project?.name ?? '프로젝트 선택'}</strong><span>변경 ▾</span></button>{open && <div className="v2-project-menu" role="menu" aria-label="프로젝트 선택">{projects.map(item => <button type="button" role="menuitem" key={item.id} onClick={() => { setOpen(false); onProject(item); }}>{item.name}<small>{item.id === project?.id ? '현재 선택됨' : item.kind === 'git' ? 'Git 프로젝트' : '일반 폴더'}</small></button>)}<button type="button" role="menuitem" onClick={() => { setOpen(false); onAddProject(); }}>+ 폴더 추가</button></div>}</div>
    <nav aria-label="주 메뉴">{navigation.map(key => <button type="button" key={key} className={tab === key ? 'nav active' : 'nav'} onClick={() => { setOpen(false); onTab(key); }}><span className="nav-icon">{icons[key]}</span>{labels[key]}</button>)}</nav><div className="sidebar-spacer"/><div className="sidebar-foot"><span className="online-dot"/> LOCAL FIRST <span className="version">{version}</span></div></aside>;
}

export function V2SettingsTabs({ section, onSection }: { section: SettingsSection; onSection: (section: SettingsSection) => void }) {
  const sections: { id: SettingsSection; label: string }[] = [{ id: 'projects', label: '프로젝트' }, { id: 'connections', label: 'AI 연결' }, { id: 'team', label: '팀' }, { id: 'spec', label: '프로젝트 명세' }, { id: 'notifications', label: '알림' }, { id: 'backup', label: '백업 삭제' }];
  return <div className="v2-settings-tabs" role="tablist" aria-label="설정 항목">{sections.map(item => <button type="button" role="tab" aria-selected={section === item.id} className="secondary" key={item.id} onClick={() => onSection(item.id)}>{item.label}</button>)}</div>;
}
