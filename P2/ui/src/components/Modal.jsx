import { useEffect, useRef } from 'react';
import { createPortal } from 'react-dom';
import { X } from 'lucide-react';

export function Modal({ open, onClose, title, description, children, footer, size = 'md' }) {
  const ref = useRef(null);

  useEffect(() => {
    if (!open) return undefined;
    const onKey = (e) => {
      if (e.key === 'Escape') onClose?.();
    };
    document.addEventListener('keydown', onKey);
    const prev = document.body.style.overflow;
    document.body.style.overflow = 'hidden';
    return () => {
      document.removeEventListener('keydown', onKey);
      document.body.style.overflow = prev;
    };
  }, [open, onClose]);

  if (!open) return null;

  const sizes = {
    sm: 'max-w-sm',
    md: 'max-w-lg',
    lg: 'max-w-2xl',
    xl: 'max-w-4xl',
  };

  return createPortal(
    <div
      className="fixed inset-0 z-40 flex items-center justify-center p-4 bg-black/60 backdrop-blur-sm animate-fadeIn"
      onMouseDown={(e) => {
        if (e.target === e.currentTarget) onClose?.();
      }}
    >
      <div
        ref={ref}
        role="dialog"
        aria-modal="true"
        aria-labelledby="modal-title"
        className={`panel border-ink-700 shadow-soft w-full ${sizes[size]} animate-fadeIn`}
      >
        <div className="flex items-start justify-between p-5 border-b border-ink-800">
          <div className="space-y-0.5">
            {title && (
              <h2 id="modal-title" className="heading-serif text-2xl text-ink-100">
                {title}
              </h2>
            )}
            {description && <p className="text-sm text-ink-400">{description}</p>}
          </div>
          <button
            onClick={onClose}
            aria-label="Cerrar"
            className="text-ink-400 hover:text-ink-100 transition-colors p-1 -mr-1"
          >
            <X className="w-5 h-5" />
          </button>
        </div>
        <div className="p-5">{children}</div>
        {footer && (
          <div className="px-5 py-4 border-t border-ink-800 flex items-center justify-end gap-2">{footer}</div>
        )}
      </div>
    </div>,
    document.body,
  );
}
