import { useCallback, useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import type { Approval, CommandMode, Goal } from './types';

/**
 * ADR-0063: the goal conversation's opt-in `명령 전부 허용` mode.
 * This hook loads and changes its own state through `get_command_mode` / `set_command_mode`
 * instead of the shared v3 controller (worklog records the duplication and its removal path).
 */
export function useCommandMode(goalId: string, onError: (message: string) => void) {
  const [mode, setMode] = useState<CommandMode | null>(null);
  const [working, setWorking] = useState(false);
  const load = useCallback(async () => {
    try { setMode(await invoke<CommandMode>('get_command_mode', { goalId })); } catch { setMode(null); }
  }, [goalId]);
  useEffect(() => {
    void load();
    let stop: (() => void) | undefined;
    let cancelled = false;
    listen('state-changed', () => void load()).then(unlisten => { if (cancelled) unlisten(); else stop = unlisten; }).catch(() => undefined);
    return () => { cancelled = true; stop?.(); };
  }, [load]);
  const change = useCallback(async (allowAll: boolean, confirmed: boolean) => {
    setWorking(true);
    try { setMode(await invoke<CommandMode>('set_command_mode', { goalId, allowAll, confirmed })); return true; }
    catch (error) { onError(String(error)); return false; }
    finally { setWorking(false); }
  }, [goalId, onError]);
  return { allowAll: mode?.allowAll ?? false, working, change };
}

/** Whether the toggle may offer `켜기`: a plan card of this execution conversation waits for approval. */
export function canTurnOn(goal: Goal, approvals: Approval[]): boolean {
  return goal.kind !== 'deliberation' && approvals.some(approval => approval.goalId === goal.id && approval.operation === 'project_write' && approval.status === 'pending');
}

/** What the confirmation dialog states, in plain words (the backend still checks everything). */
export const ALLOW_ALL_RISKS = [
  '팀원이 명령을 실행할 때 더 이상 묻지 않아요. Windows에서는 내 컴퓨터 사용자 권한으로 실행돼요.',
  '작업 폴더 밖 파일과 인터넷에도 닿을 수 있어요. 프로젝트 파일은 지금처럼 승인한 파일만 반영돼요.',
  '저장소 안 문서에 숨은 지시를 팀원이 그대로 실행할 수도 있어요.',
  'push, GitHub PR, 배포·게시는 이 모드에서도 막아요. 다만 일부러 우회하는 명령까지 완전히 막지는 못해요.',
  '이 목표 대화에만 적용되고, 언제든 끌 수 있어요.',
];
