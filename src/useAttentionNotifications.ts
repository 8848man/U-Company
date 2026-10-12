import { useEffect, useRef, useState } from 'react';
import { attentionPreferencesKey, captureAttentionBaseline, detectAttentionTransitions, readAttentionPreferences, type AttentionBaseline, type AttentionPreferences } from './attentionNotifications';
import { enableSystemNotifications, playAttentionSound, showSystemAttention } from './notificationDelivery';
import type { Snapshot } from './types';

export function useAttentionNotifications(snapshot: Snapshot, ready: boolean) {
  const [preferences, setPreferences] = useState<AttentionPreferences>(() => {
    try { return readAttentionPreferences(window.localStorage); }
    catch { return { system: false, sound: false }; }
  });
  const [error, setError] = useState('');
  const [pending, setPending] = useState(false);
  const baseline = useRef<AttentionBaseline | null>(null);
  const currentPreferences = useRef(preferences);
  currentPreferences.current = preferences;

  function persist(next: AttentionPreferences): boolean {
    try {
      window.localStorage.setItem(attentionPreferencesKey, JSON.stringify(next));
      setPreferences(next);
      setError('');
      return true;
    } catch {
      setError('알림 설정을 저장할 수 없습니다. 앱 데이터 저장소를 확인하세요.');
      return false;
    }
  }

  async function setSystem(enabled: boolean) {
    if (!enabled) { persist({ ...currentPreferences.current, system: false }); return; }
    setPending(true);
    try {
      if (await enableSystemNotifications()) persist({ ...currentPreferences.current, system: true });
      else setError('시스템 알림 권한이 허용되지 않았습니다. Windows 알림 설정에서 이 앱을 허용한 뒤 다시 시도하세요.');
    } catch (cause) {
      setError(`시스템 알림을 켤 수 없습니다: ${String(cause)}. 설치된 앱인지 확인하세요.`);
    } finally {
      setPending(false);
    }
  }

  function setSound(enabled: boolean) {
    if (persist({ ...currentPreferences.current, sound: enabled }) && enabled) {
      void playAttentionSound('approval').catch(cause => setError(`소리를 재생할 수 없습니다: ${String(cause)}`));
    }
  }

  function testSound() {
    void playAttentionSound('approval').catch(cause => setError(`소리를 재생할 수 없습니다: ${String(cause)}`));
  }

  function testSystem() {
    if (!currentPreferences.current.system) { setError('먼저 시스템 알림을 켜세요.'); return; }
    void showSystemAttention('approval', true).then(() => setError('')).catch(cause => setError(`시험 알림을 보내지 못했습니다: ${String(cause)}. 설치된 앱인지 확인하세요.`));
  }

  useEffect(() => {
    if (!ready) return;
    const previous = baseline.current;
    baseline.current = captureAttentionBaseline(snapshot);
    if (!previous) return;
    const signals = detectAttentionTransitions(previous, snapshot);
    if (!signals.length) return;
    const enabled = currentPreferences.current;
    if (enabled.sound) void playAttentionSound(signals[0].kind).catch(cause => setError(`소리를 재생할 수 없습니다: ${String(cause)}`));
    if (enabled.system) for (const signal of signals) {
      void showSystemAttention(signal.kind).catch(cause => setError(`시스템 알림을 보내지 못했습니다: ${String(cause)}. 설치된 앱인지 확인하세요.`));
    }
  }, [snapshot, ready]);

  return { preferences, error, pending, setSystem, setSound, testSound, testSystem };
}
