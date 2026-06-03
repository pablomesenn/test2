import { forwardRef, useState } from 'react';
import { Copy, Check, Loader2 } from 'lucide-react';

// -----------------------------------------------------------------------------
// Button
// -----------------------------------------------------------------------------

export const Button = forwardRef(function Button(
  { variant = 'primary', size = 'md', loading = false, className = '', children, disabled, ...rest },
  ref,
) {
  const variants = {
    primary:
      'bg-accent-500 text-ink-950 hover:bg-accent-400 active:bg-accent-600 focus-visible:outline-accent-500',
    secondary:
      'bg-ink-800 text-ink-100 hover:bg-ink-700 border border-ink-700 focus-visible:outline-ink-500',
    ghost:
      'bg-transparent text-ink-200 hover:bg-ink-800 hover:text-ink-100',
    danger:
      'bg-danger-500 text-white hover:bg-danger-400 active:bg-danger-500/90',
    'danger-outline':
      'border border-danger-500/40 text-danger-400 hover:bg-danger-500/10 hover:border-danger-500',
  };
  const sizes = {
    sm: 'px-2.5 py-1 text-xs',
    md: 'px-3.5 py-1.5 text-sm',
    lg: 'px-5 py-2.5 text-sm',
  };
  return (
    <button
      ref={ref}
      disabled={disabled || loading}
      className={[
        'inline-flex items-center justify-center gap-1.5 rounded-md font-medium transition-colors disabled:opacity-50 disabled:pointer-events-none',
        variants[variant] ?? variants.primary,
        sizes[size] ?? sizes.md,
        className,
      ].join(' ')}
      {...rest}
    >
      {loading && <Loader2 className="w-3.5 h-3.5 animate-spin" />}
      {children}
    </button>
  );
});

// -----------------------------------------------------------------------------
// Input / Select / Textarea
// -----------------------------------------------------------------------------

export function Field({ label, hint, error, children, htmlFor }) {
  return (
    <label htmlFor={htmlFor} className="block space-y-1.5">
      {label && (
        <span className="text-xs font-medium text-ink-300 tracking-wide uppercase">
          {label}
        </span>
      )}
      {children}
      {error ? (
        <span className="text-xs text-danger-400">{error}</span>
      ) : hint ? (
        <span className="text-xs text-ink-400">{hint}</span>
      ) : null}
    </label>
  );
}

const baseControl =
  'w-full rounded-md border border-ink-700 bg-ink-900 text-ink-100 placeholder:text-ink-500 px-3 py-2 text-sm transition-colors focus:border-accent-500 focus:outline-none disabled:opacity-50';

export const Input = forwardRef(function Input({ className = '', mono = false, ...rest }, ref) {
  return (
    <input
      ref={ref}
      className={[baseControl, mono ? 'mono' : '', className].join(' ')}
      {...rest}
    />
  );
});

export const Textarea = forwardRef(function Textarea({ className = '', ...rest }, ref) {
  return <textarea ref={ref} className={[baseControl, 'min-h-[80px]', className].join(' ')} {...rest} />;
});

export const Select = forwardRef(function Select({ className = '', children, ...rest }, ref) {
  return (
    <select ref={ref} className={[baseControl, 'pr-8 appearance-none', className].join(' ')} {...rest}>
      {children}
    </select>
  );
});

// -----------------------------------------------------------------------------
// Badge
// -----------------------------------------------------------------------------

export function Badge({ children, variant = 'neutral', className = '' }) {
  const variants = {
    neutral: 'border-ink-700 bg-ink-800/60 text-ink-200',
    success: 'border-success-500/30 bg-success-500/10 text-success-400',
    warning: 'border-accent-500/30 bg-accent-500/10 text-accent-400',
    danger: 'border-danger-500/30 bg-danger-500/10 text-danger-400',
    info: 'border-primary-500/30 bg-primary-500/10 text-primary-400',
  };
  return (
    <span
      className={[
        'inline-flex items-center gap-1 px-2 py-0.5 text-[11px] font-medium rounded-full border',
        variants[variant],
        className,
      ].join(' ')}
    >
      {children}
    </span>
  );
}

// -----------------------------------------------------------------------------
// Card
// -----------------------------------------------------------------------------

export function Card({ children, className = '', as: As = 'div' }) {
  return <As className={`panel p-5 ${className}`}>{children}</As>;
}

// -----------------------------------------------------------------------------
// Spinner
// -----------------------------------------------------------------------------

export function Spinner({ label }) {
  return (
    <div className="flex items-center gap-2 text-ink-400 text-sm">
      <Loader2 className="w-4 h-4 animate-spin" />
      {label && <span>{label}</span>}
    </div>
  );
}

// -----------------------------------------------------------------------------
// EmptyState
// -----------------------------------------------------------------------------

export function EmptyState({ icon: Icon, title, description, action }) {
  return (
    <div className="panel p-10 flex flex-col items-center text-center gap-3">
      {Icon && (
        <div className="p-3 rounded-full border border-ink-800 bg-ink-900 text-ink-400">
          <Icon className="w-5 h-5" />
        </div>
      )}
      <div>
        <h3 className="text-base text-ink-100">{title}</h3>
        {description && <p className="text-sm text-ink-400 mt-1 max-w-md">{description}</p>}
      </div>
      {action}
    </div>
  );
}

// -----------------------------------------------------------------------------
// CopyButton
// -----------------------------------------------------------------------------

export function CopyButton({ value, label = 'Copiar', className = '' }) {
  const [copied, setCopied] = useState(false);
  return (
    <button
      type="button"
      onClick={async () => {
        try {
          await navigator.clipboard.writeText(value);
          setCopied(true);
          setTimeout(() => setCopied(false), 1500);
        } catch {
          /* clipboard rechazado: ignorar silenciosamente */
        }
      }}
      className={[
        'inline-flex items-center gap-1.5 text-xs text-ink-400 hover:text-ink-100 transition-colors',
        className,
      ].join(' ')}
    >
      {copied ? <Check className="w-3.5 h-3.5 text-success-500" /> : <Copy className="w-3.5 h-3.5" />}
      <span>{copied ? 'Copiado' : label}</span>
    </button>
  );
}

// -----------------------------------------------------------------------------
// MonoTag — pequeñas etiquetas técnicas (IDs, tokens cortos, claves)
// -----------------------------------------------------------------------------

export function MonoTag({ children, className = '' }) {
  return (
    <code
      className={[
        'mono inline-flex items-center px-1.5 py-0.5 rounded text-[11px] text-ink-200 bg-ink-800/80 border border-ink-700',
        className,
      ].join(' ')}
    >
      {children}
    </code>
  );
}
