import { useEffect, useMemo, useState } from 'react';
import { useSearchParams } from 'react-router-dom';
import { LogIn, ShieldCheck, AlertTriangle, ArrowRight, Lock } from 'lucide-react';
import { api, ApiError } from '../lib/api.js';
import { Button, Field, Input, MonoTag } from '../components/ui.jsx';

/**
 * Página de autenticación invocada por las cachés zonales.
 *
 * Query params:
 *   - domain     (obligatorio): nombre del dominio cuyas credenciales se validan.
 *   - return_to  (opcional)   : URL a la que se debe redirigir tras autenticarse.
 *   - origin     (opcional)   : nombre legible del origen (mostrado al usuario).
 *
 * Flujo:
 *   1. La caché recibe una petición que requiere user/password.
 *   2. La caché responde con 401 + redirección a esta URL.
 *   3. El usuario ingresa credenciales y la UI llama a POST /api/zonal-auth/validate.
 *   4. La UI redirige a `return_to` con `?token=<sessionToken>` para que la caché
 *      lo use en peticiones subsecuentes.
 */
export function ZonalAuthPage() {
  const [params] = useSearchParams();
  const domainName = params.get('domain') ?? '';
  const returnTo = params.get('return_to') ?? '';
  const originName = params.get('origin') ?? '';

  const [username, setUsername] = useState('');
  const [password, setPassword] = useState('');
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState('');
  const [success, setSuccess] = useState(null);

  const returnHost = useMemo(() => {
    try {
      return returnTo ? new URL(returnTo).host : '';
    } catch {
      return '';
    }
  }, [returnTo]);

  const missingParams = !domainName;

  const onSubmit = async (e) => {
    e.preventDefault();
    setError('');
    if (!username || !password) {
      setError('Usuario y contraseña son obligatorios.');
      return;
    }
    setLoading(true);
    try {
      const res = await api.zonalAuthValidate({ domainName, username, password });
      setSuccess(res);

      if (returnTo) {
        const sep = returnTo.includes('?') ? '&' : '?';
        const target = `${returnTo}${sep}token=${encodeURIComponent(res.sessionToken ?? res.token ?? '')}`;
        // Pequeña pausa para que el usuario vea que fue exitoso
        setTimeout(() => {
          window.location.href = target;
        }, 900);
      }
    } catch (err) {
      if (err instanceof ApiError && (err.statusCode === 404 || err.statusCode === 501)) {
        setError(
          'El endpoint POST /api/zonal-auth/validate aún no está implementado en el Node API. La UI ya está lista; falta el backend.',
        );
      } else if (err instanceof ApiError && err.statusCode === 401) {
        setError('Credenciales inválidas para este dominio.');
      } else {
        setError(err.message);
      }
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="min-h-screen flex items-center justify-center p-6 bg-grid">
      <div className="w-full max-w-md panel p-8 shadow-soft animate-fadeIn">
        <div className="flex items-center gap-2 mb-6">
          <div className="p-2 rounded-md border border-accent-500/40 bg-accent-500/10 text-accent-500">
            <Lock className="w-4 h-4" />
          </div>
          <div>
            <p className="text-[10px] uppercase tracking-[0.18em] text-ink-500">Autenticación de caché zonal</p>
            <h1 className="heading-serif italic text-2xl text-ink-100 leading-tight">
              Acceso protegido
            </h1>
          </div>
        </div>

        {missingParams ? (
          <ParamError />
        ) : success && !returnTo ? (
          <SuccessNoReturn token={success.sessionToken ?? success.token} />
        ) : success && returnTo ? (
          <RedirectingNotice host={returnHost} />
        ) : (
          <>
            <div className="rounded-md border border-ink-800 bg-ink-900/60 px-3 py-2.5 mb-5 text-xs space-y-1">
              <div className="flex items-center justify-between gap-2">
                <span className="text-ink-500">Dominio</span>
                <MonoTag>{domainName}</MonoTag>
              </div>
              {originName && (
                <div className="flex items-center justify-between gap-2">
                  <span className="text-ink-500">Origen</span>
                  <span className="text-ink-200">{originName}</span>
                </div>
              )}
              {returnHost && (
                <div className="flex items-center justify-between gap-2">
                  <span className="text-ink-500">Volverá a</span>
                  <MonoTag>{returnHost}</MonoTag>
                </div>
              )}
            </div>

            <form onSubmit={onSubmit} className="space-y-4">
              <Field label="Usuario">
                <Input
                  value={username}
                  onChange={(e) => setUsername(e.target.value)}
                  placeholder="usuario-del-dominio"
                  autoFocus
                  required
                  mono
                />
              </Field>
              <Field label="Contraseña">
                <Input
                  type="password"
                  value={password}
                  onChange={(e) => setPassword(e.target.value)}
                  placeholder="••••••••"
                  required
                />
              </Field>

              {error && (
                <div className="rounded-md border border-danger-500/30 bg-danger-500/5 px-3 py-2 text-xs text-danger-400 flex items-start gap-2">
                  <AlertTriangle className="w-3.5 h-3.5 mt-0.5 shrink-0" />
                  <span>{error}</span>
                </div>
              )}

              <Button type="submit" size="lg" loading={loading} className="w-full">
                <LogIn className="w-4 h-4" />
                Autenticar
              </Button>
            </form>
          </>
        )}

        <p className="text-[11px] text-ink-500 text-center mt-6">
          Este formulario es invocado por una caché zonal del proxy IC-7602.
        </p>
      </div>
    </div>
  );
}

function ParamError() {
  return (
    <div className="rounded-md border border-danger-500/30 bg-danger-500/5 px-4 py-3 text-sm text-danger-400 flex gap-2 items-start">
      <AlertTriangle className="w-4 h-4 mt-0.5 shrink-0" />
      <div>
        <p>Falta el parámetro <code className="mono">domain</code> en la URL.</p>
        <p className="text-xs text-ink-400 mt-1">
          Esperado: <code className="mono">/auth/zonal?domain=ejemplo.test&return_to=...</code>
        </p>
      </div>
    </div>
  );
}

function SuccessNoReturn({ token }) {
  return (
    <div className="space-y-3">
      <div className="rounded-md border border-success-500/30 bg-success-500/5 px-4 py-3 text-sm text-success-400 flex gap-2 items-center">
        <ShieldCheck className="w-4 h-4" />
        <span>Autenticación exitosa.</span>
      </div>
      <div className="rounded-md border border-ink-800 bg-ink-900 p-3">
        <p className="text-[10px] uppercase tracking-[0.18em] text-ink-500 mb-1">Session token</p>
        <code className="mono text-xs text-ink-100 break-all">{token}</code>
      </div>
      <p className="text-xs text-ink-500">
        No se proporcionó <code className="mono">return_to</code>. Copia el token manualmente si lo necesitas.
      </p>
    </div>
  );
}

function RedirectingNotice({ host }) {
  return (
    <div className="space-y-3">
      <div className="rounded-md border border-success-500/30 bg-success-500/5 px-4 py-3 text-sm text-success-400 flex gap-2 items-center">
        <ShieldCheck className="w-4 h-4" />
        <span>Autenticación exitosa. Redirigiendo…</span>
      </div>
      <p className="text-xs text-ink-400 flex items-center gap-1">
        Volviendo a <MonoTag>{host}</MonoTag> <ArrowRight className="w-3 h-3" />
      </p>
    </div>
  );
}
