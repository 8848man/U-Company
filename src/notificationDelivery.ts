import { isPermissionGranted, requestPermission, sendNotification } from '@tauri-apps/plugin-notification';
import type { AttentionKind } from './attentionNotifications';

let audioContext: AudioContext | null = null;

export async function enableSystemNotifications(): Promise<boolean> {
  if (await isPermissionGranted()) return true;
  return (await requestPermission()) === 'granted';
}

export async function showSystemAttention(kind: AttentionKind, sample = false): Promise<void> {
  if (!(await isPermissionGranted())) throw new Error('Windows 알림 권한이 허용되지 않았습니다. 시스템 알림 설정을 확인하세요.');
  const title = sample ? 'Utopia 알림 테스트' : kind === 'approval' ? 'Utopia · 승인 요청' : 'Utopia · 작업 중단';
  const body = sample ? '알림 전달을 확인하는 샘플입니다. 작업 상태는 바뀌지 않았습니다.' : kind === 'approval' ? '확인할 승인 요청이 있습니다. 앱에서 변경 범위를 검토하세요.' : '중단된 목표가 있습니다. 앱에서 원인과 다음 행동을 확인하세요.';
  sendNotification({ title, body });
}

export async function playAttentionSound(kind: AttentionKind): Promise<void> {
  const context = audioContext ?? new AudioContext();
  audioContext = context;
  if (context.state === 'suspended') await context.resume();
  const start = context.currentTime;
  const notes = kind === 'approval' ? [660, 880] : [440, 330];
  notes.forEach((frequency, index) => {
    const oscillator = context.createOscillator();
    const gain = context.createGain();
    const at = start + index * 0.13;
    oscillator.type = 'sine';
    oscillator.frequency.value = frequency;
    gain.gain.setValueAtTime(0.0001, at);
    gain.gain.exponentialRampToValueAtTime(0.055, at + 0.012);
    gain.gain.exponentialRampToValueAtTime(0.0001, at + 0.11);
    oscillator.connect(gain).connect(context.destination);
    oscillator.start(at);
    oscillator.stop(at + 0.12);
  });
}
