import type { AttentionPreferences } from '../../src/attentionNotifications';

export function V2NotificationSettings({ preferences, error, pending, onSystem, onSound, onTestSystem, onTestSound }: { preferences: AttentionPreferences; error: string; pending: boolean; onSystem: (enabled: boolean) => void; onSound: (enabled: boolean) => void; onTestSystem: () => void; onTestSound: () => void }) {
  return <section className="card v2-notification-settings v2-settings-narrow"><div className="card-kicker">ATTENTION</div><h2>중요한 일만 알림</h2><p>새 승인 요청과 목표가 중단된 순간에만 알립니다. 앱을 다시 열 때 기존 항목을 한꺼번에 알리지 않습니다.</p>
    <div className="v2-notification-option"><div><strong>시스템 알림</strong><small>Windows 알림 센터에 짧게 표시합니다. 앱을 설치한 뒤 사용할 수 있습니다.</small></div><label><input type="checkbox" checked={preferences.system} disabled={pending} onChange={event => onSystem(event.target.checked)}/> {preferences.system ? '켜짐' : '꺼짐'}</label></div>
    <div className="v2-notification-option"><div><strong>앱 사운드</strong><small>승인 요청과 중단에 서로 다른 짧은 소리를 냅니다. Windows 자체 알림 소리는 OS 설정을 따릅니다.</small></div><label><input type="checkbox" checked={preferences.sound} onChange={event => onSound(event.target.checked)}/> {preferences.sound ? '켜짐' : '꺼짐'}</label></div>
    {error && <p className="v2-notification-error" role="alert">{error}</p>}
    <div className="button-row"><button type="button" className="secondary" onClick={onTestSound}>소리 시험</button><button type="button" className="secondary" onClick={onTestSystem}>시스템 알림 시험</button></div>
    <p className="muted">시험 버튼은 샘플만 재생합니다. 목표·승인 상태는 바꾸지 않습니다. 알림을 눌러도 승인되지 않으며 앱에서 범위를 검토해야 합니다.</p>
  </section>;
}
