import { useState } from 'react';
import { Navigate, useLocation, useNavigate } from 'react-router-dom';
import { useAuth } from '../context/AuthContext.jsx';
import { useToast } from '../context/ToastContext.jsx';
import { signIn, signUp, describeAuthError } from '../lib/firebase.js';
import { isFirebaseConfigured } from '../config.js';
import { Button, Field, Input } from '../components/ui.jsx';
import { Network, ShieldAlert, ArrowRight } from 'lucide-react';

export function LoginPage() {
  const { user, ready, firebaseConfigured } = useAuth();
  const location = useLocation();
  const navigate = useNavigate();
  const toast = useToast();

  const [mode, setMode] = useState('signin');
  const [email, setEmail] = useState('');
  const [password, setPassword] = useState('');
  const [displayName, setDisplayName] = useState('');
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState('');

  if (!firebaseConfigured) {
    return <FirebaseNotConfigured />;
  }

  if (ready && user) {
    const from = location.state?.from?.pathname ?? '/dashboard';
    return <Navigate to={from} replace />;
  }

  const onSubmit = async (e) => {
    e.preventDefault();
    setError('');
    setLoading(true);
    try {
      if (mode === 'signin') {
        await signIn(email, password);
        toast({ kind: 'success', title: 'Sesión iniciada' });
      } else {
        await signUp({ email, password, displayName: displayName.trim() || undefined });
        toast({ kind: 'success', title: 'Cuenta creada', message: 'Te dimos la bienvenida.' });
      }
      navigate('/dashboard', { replace: true });
    } catch (err) {
      setError(describeAuthError(err));
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="min-h-screen flex">
      {/* Lado decorativo */}
      <div className="hidden lg:flex relative w-1/2 bg-grid border-r border-ink-800 overflow-hidden">
        <div className="absolute inset-0 bg-gradient-to-br from-accent-500/10 via-transparent to-primary-500/5" />
        <div className="relative m-auto p-12 max-w-md space-y-8">
          <div className="flex items-center gap-2.5">
            <div className="w-9 h-9 rounded-md border border-accent-500/40 bg-accent-500/10 flex items-center justify-center">
              <Network className="w-4 h-4 text-accent-500" />
            </div>
            <span className="text-sm tracking-[0.2em] text-ink-300 uppercase">Proyecto 2 · IC-7602</span>
          </div>

          <h1 className="heading-serif italic text-5xl leading-tight text-ink-100">
            Consola para <span className="text-accent-500 not-italic mono text-3xl align-middle">proxy/cache</span> zonal.
          </h1>

          <p className="text-ink-400 leading-relaxed">
            Administra dominios, reglas por URL, políticas de reemplazo, API keys y usuarios para las cachés
            zonales. Verifica la propiedad de tus dominios mediante registros TXT.
          </p>

          <ul className="space-y-2 text-sm text-ink-300">
            {[
              'Verificación de dominio por DNS TXT',
              'Wildcards y políticas LRU · LFU · FIFO · MRU · Random',
              'API keys y usuarios por dominio',
              'Integración con Firebase Auth + Node API',
            ].map((line) => (
              <li key={line} className="flex items-center gap-2">
                <span className="w-1 h-1 rounded-full bg-accent-500" />
                {line}
              </li>
            ))}
          </ul>
        </div>
      </div>

      {/* Formulario */}
      <div className="flex-1 flex items-center justify-center p-6">
        <div className="w-full max-w-sm space-y-6 animate-fadeIn">
          <div className="space-y-1">
            <p className="text-[11px] uppercase tracking-[0.2em] text-ink-500">
              {mode === 'signin' ? 'Acceso' : 'Crear cuenta'}
            </p>
            <h2 className="heading-serif text-3xl text-ink-100">
              {mode === 'signin' ? 'Iniciar sesión.' : 'Crear cuenta.'}
            </h2>
            <p className="text-sm text-ink-400">
              {mode === 'signin'
                ? 'Ingresa con la cuenta que registro en Firebase.'
                : 'La cuenta se crea en Firebase Auth.'}
            </p>
          </div>

          <div className="inline-flex p-1 bg-ink-900 border border-ink-800 rounded-md">
            <TabBtn active={mode === 'signin'} onClick={() => setMode('signin')}>
              Iniciar sesión
            </TabBtn>
            <TabBtn active={mode === 'signup'} onClick={() => setMode('signup')}>
              Crear cuenta
            </TabBtn>
          </div>

          <form onSubmit={onSubmit} className="space-y-4">
            {mode === 'signup' && (
              <Field label="Nombre (opcional)" htmlFor="displayName">
                <Input
                  id="displayName"
                  type="text"
                  autoComplete="name"
                  value={displayName}
                  onChange={(e) => setDisplayName(e.target.value)}
                  placeholder="Ana Pérez"
                />
              </Field>
            )}

            <Field label="Email" htmlFor="email">
              <Input
                id="email"
                type="email"
                required
                autoComplete="email"
                value={email}
                onChange={(e) => setEmail(e.target.value)}
                placeholder="alguien@correo.com"
              />
            </Field>

            <Field label="Contraseña" htmlFor="password" hint={mode === 'signup' ? 'Mínimo 6 caracteres.' : undefined}>
              <Input
                id="password"
                type="password"
                required
                autoComplete={mode === 'signin' ? 'current-password' : 'new-password'}
                value={password}
                onChange={(e) => setPassword(e.target.value)}
                placeholder="••••••••"
              />
            </Field>

            {error && (
              <div className="rounded-md border border-danger-500/30 bg-danger-500/5 px-3 py-2 text-xs text-danger-400">
                {error}
              </div>
            )}

            <Button type="submit" size="lg" loading={loading} className="w-full">
              {mode === 'signin' ? 'Entrar' : 'Crear cuenta'}
              <ArrowRight className="w-4 h-4" />
            </Button>
          </form>

          <p className="text-xs text-ink-500 text-center">
            Proyecto IC-7602.
          </p>
        </div>
      </div>
    </div>
  );
}

function TabBtn({ active, onClick, children }) {
  return (
    <button
      type="button"
      onClick={onClick}
      className={[
        'px-3 py-1.5 text-xs rounded transition-colors',
        active ? 'bg-ink-800 text-ink-100' : 'text-ink-400 hover:text-ink-200',
      ].join(' ')}
    >
      {children}
    </button>
  );
}

function FirebaseNotConfigured() {
  return (
    <div className="min-h-screen flex items-center justify-center p-6">
      <div className="max-w-lg panel p-8 space-y-4">
        <div className="flex items-center gap-3">
          <div className="p-2 rounded-md bg-danger-500/10 text-danger-500 border border-danger-500/30">
            <ShieldAlert className="w-5 h-5" />
          </div>
          <h1 className="heading-serif text-3xl">Firebase no está configurado</h1>
        </div>
        <p className="text-sm text-ink-300">
          Para que la UI funcione necesitas definir las variables públicas de Firebase Web SDK en{' '}
          <code className="mono bg-ink-800 px-1.5 py-0.5 rounded text-xs">.env.local</code>:
        </p>
        <pre className="mono text-xs bg-ink-900 border border-ink-800 rounded-md p-4 overflow-x-auto text-ink-300">
{`VITE_FIREBASE_API_KEY=...
VITE_FIREBASE_AUTH_DOMAIN=...
VITE_FIREBASE_PROJECT_ID=...
VITE_FIREBASE_APP_ID=...`}
        </pre>
        <p className="text-xs text-ink-400">
          Estos valores se obtienen en <strong>Firebase Console → Project Settings → SDK setup</strong>. Luego
          reinicia el servidor de desarrollo.
        </p>
      </div>
    </div>
  );
}
