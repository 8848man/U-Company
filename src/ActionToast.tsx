import { useEffect, useRef } from 'react';

type Props = {
  message: string;
  actionLabel?: string;
  onAction?: () => void;
  onDismiss: () => void;
};

export default function ActionToast({ message, actionLabel, onAction, onDismiss }: Props) {
  const dismissRef = useRef(onDismiss);
  dismissRef.current = onDismiss;
  useEffect(() => {
    const timer = window.setTimeout(() => dismissRef.current(), 6000);
    return () => window.clearTimeout(timer);
  }, [message]);
  return <div className="action-toast" role="status" aria-live="polite">
    <span className="toast-mark">✓</span><span className="toast-message">{message}</span>
    {actionLabel && onAction && <button type="button" className="toast-action" onClick={onAction}>{actionLabel}</button>}
    <button type="button" className="toast-close" aria-label="알림 닫기" onClick={onDismiss}>×</button>
  </div>;
}
