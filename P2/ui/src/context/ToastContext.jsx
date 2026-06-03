import { createContext, useCallback, useContext, useState } from 'react';
import { CheckCircle2, XCircle, Info, AlertTriangle, X } from 'lucide-react';

const ToastContext = createContext({ toast: () => {} });

const ICONS = {
  success: CheckCircle2,
  error: XCircle,
  info: Info,
  warning: AlertTriangle,
};

const COLORS = {
  success: 'text-success-500 border-success-500/30 bg-success-500/5',
  error: 'text-danger-500 border-danger-500/30 bg-danger-500/5',
  info: 'text-primary-500 border-primary-500/30 bg-primary-500/5',
  warning: 'text-accent-500 border-accent-500/30 bg-accent-500/5',
};

export function ToastProvider({ children }) {
  const [toasts, setToasts] = useState([]);

  const dismiss = useCallback((id) => {
    setToasts((curr) => curr.filter((t) => t.id !== id));
  }, []);

  const toast = useCallback((opts) => {
    const id = Math.random().toString(36).slice(2);
    const next = {
      id,
      kind: opts.kind ?? 'info',
      title: opts.title ?? '',
      message: opts.message ?? '',
      duration: opts.duration ?? 4500,
    };
    setToasts((curr) => [...curr, next]);
    if (next.duration > 0) {
      setTimeout(() => dismiss(id), next.duration);
    }
    return id;
  }, [dismiss]);

  return (
    <ToastContext.Provider value={{ toast, dismiss }}>
      {children}
      <div className="fixed bottom-6 right-6 z-50 flex flex-col gap-2 max-w-sm w-full">
        {toasts.map((t) => {
          const Icon = ICONS[t.kind] ?? Info;
          return (
            <div
              key={t.id}
              className={`panel border ${COLORS[t.kind]} px-4 py-3 flex items-start gap-3 animate-slideIn shadow-soft`}
              role="status"
            >
              <Icon className="w-5 h-5 mt-0.5 shrink-0" />
              <div className="flex-1 min-w-0">
                {t.title && <p className="text-sm font-medium text-ink-100">{t.title}</p>}
                {t.message && <p className="text-xs text-ink-300 mt-0.5 break-words">{t.message}</p>}
              </div>
              <button
                onClick={() => dismiss(t.id)}
                className="text-ink-400 hover:text-ink-100 transition-colors"
                aria-label="Cerrar"
              >
                <X className="w-4 h-4" />
              </button>
            </div>
          );
        })}
      </div>
    </ToastContext.Provider>
  );
}

export function useToast() {
  return useContext(ToastContext).toast;
}
