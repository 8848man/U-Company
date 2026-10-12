import type { AiConnection } from './types';

const providers: Record<string, { name: string; login: string }> = {
  claude: { name: 'Claude Code', login: 'Claude 로그인 창을 열고 브라우저 인증을 마친 뒤 같은 경로를 다시 검사하세요.' },
  codex: { name: 'Codex CLI', login: 'Codex 로그인 창을 열고 인증을 마친 뒤 같은 경로를 다시 검사하세요.' },
  gemini: { name: 'Gemini CLI', login: 'Gemini 창에서 로그인 방식을 선택하고 인증을 마친 뒤 다시 검사하세요.' },
  opencode: { name: 'OpenCode', login: 'OpenCode 로그인 창에서 사용할 AI 제공자를 선택해 인증한 뒤 다시 검사하세요.' },
};

export function canOpenProviderLogin(connection: AiConnection): boolean {
  return connection.kind === 'cli' && Object.hasOwn(providers, connection.id) && Boolean(connection.executable);
}

export function connectionRecovery(connection: AiConnection): string {
  if (connection.status === 'needs_setup') return providers[connection.id]?.login ?? '제공자 CLI에서 로그인한 뒤 다시 검사하세요.';
  if (connection.status === 'needs_auth_check') return `${providers[connection.id]?.name ?? connection.name} 로그인 상태를 확인하지 못했습니다. 로그인 창에서 상태를 확인하고 다시 검사하세요.`;
  if (connection.status === 'failed' || connection.status === 'missing') return '저장된 실행 경로가 실제 설치 파일인지 확인하고, 필요하면 연결 경로를 변경하세요.';
  return '로그인 상태를 다시 확인하려면 저장된 경로를 검사하세요.';
}

export function runtimeRecovery(report: string): string | null {
  const provider = /Provider=(claude|codex|gemini|opencode)\b/i.exec(report)?.[1].toLowerCase();
  if (!provider) return null;
  const detail = report.toLowerCase();
  if (provider === 'claude' && /organization disabled/.test(detail)) return 'Claude 로그인과 별개로 ANTHROPIC_API_KEY 환경 변수가 우선 적용되는지 확인하세요. 의도하지 않은 키라면 해당 터미널 환경에서 제거한 뒤 다시 검사하세요.';
  if (provider === 'gemini' && /named user|entitlement|google_cloud_project/.test(detail)) return 'Gemini의 Google 계정 유형과 GOOGLE_CLOUD_PROJECT 설정을 확인하세요. 개인 계정에 조직 프로젝트 값이 잡혀 있는지도 확인하세요.';
  if (provider === 'opencode' && /provider not found|no provider configured/.test(detail)) return 'OpenCode 로그인 창에서 실제 사용할 AI 제공자를 연결하고 모델을 선택하세요.';
  if (/not logged in|login required|authentication required|unauthorized|unauthenticated|invalid api key|invalid token|로그인 필요|로그인되지/.test(detail)) return providers[provider].login;
  if (/rate.limit|too many requests|429|quota|usage.limit|credit|billing/.test(detail)) return `${providers[provider].name}의 요청 한도 또는 결제 상태를 확인하고, 제공자가 허용하는 시점에 다시 실행하세요.`;
  if (/model.*(not found|unavailable|unsupported|invalid)|unknown model|404.*model/.test(detail)) return `${providers[provider].name}에서 사용 가능한 모델을 확인하고 팀원의 모델 선택을 바꾸세요.`;
  if (/network|connection refused|dns|certificate|timed out|timeout|fetch failed/.test(detail)) return `${providers[provider].name}의 네트워크 연결과 프록시 설정을 확인한 뒤 다시 시도하세요.`;
  if (/not found|no such file|spawn|win32|os error 193|unsupported.wrapper|실행 파일/.test(detail)) return `${providers[provider].name}의 저장된 실행 경로를 AI 연결에서 다시 검사하세요.`;
  return `${providers[provider].name}의 저장된 경로를 다시 검사하고 오류의 원인을 확인하세요.`;
}
