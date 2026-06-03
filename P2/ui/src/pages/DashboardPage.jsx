import { useEffect, useState } from 'react';
import { Link } from 'react-router-dom';
import { Globe, Plus, RefreshCw, ShieldCheck, ShieldAlert, ArrowRight, Copy as CopyIcon } from 'lucide-react';
import { api, ApiError } from '../lib/api.js';
import { useToast } from '../context/ToastContext.jsx';
import { Button, Card, EmptyState, Field, Input, Spinner, Badge, MonoTag, CopyButton } from '../components/ui.jsx';
import { Modal } from '../components/Modal.jsx';
import { formatTimestamp } from '../lib/formatters.js';

export function DashboardPage() {
  const toast = useToast();
  const [domains, setDomains] = useState([]);
  const [loading, setLoading] = useState(true);
  const [refreshing, setRefreshing] = useState(false);

  // Modal de creación
  const [createOpen, setCreateOpen] = useState(false);
  const [newDomainName, setNewDomainName] = useState('');
  const [creating, setCreating] = useState(false);
  const [createdInfo, setCreatedInfo] = useState(null);

  const loadDomains = async (silent = false) => {
    if (!silent) setLoading(true);
    else setRefreshing(true);
    try {
      const res = await api.listDomains();
      setDomains(res.domains ?? []);
    } catch (err) {
      toast({ kind: 'error', title: 'No se pudieron cargar los dominios', message: err.message });
    } finally {
      setLoading(false);
      setRefreshing(false);
    }
  };

  useEffect(() => {
    loadDomains();
  }, []);

  const onCreate = async (e) => {
    e.preventDefault();
    const name = newDomainName.trim().toLowerCase();
    if (!name) return;
    setCreating(true);
    try {
      const res = await api.createDomain(name);
      setCreatedInfo(res);
      setNewDomainName('');
      await loadDomains(true);
      toast({ kind: 'success', title: 'Dominio registrado', message: 'Ahora verifícalo con el registro TXT.' });
    } catch (err) {
      const msg =
        err instanceof ApiError && err.statusCode === 409
          ? 'Ya tienes ese dominio registrado.'
          : err.message;
      toast({ kind: 'error', title: 'No se pudo registrar el dominio', message: msg });
    } finally {
      setCreating(false);
    }
  };

  const verified = domains.filter((d) => d.verified).length;
  const pending = domains.length - verified;

  return (
    <div className="space-y-8">
      <header className="flex flex-wrap items-end justify-between gap-4">
        <div>
          <p className="text-[11px] uppercase tracking-[0.2em] text-ink-500">Inicio</p>
          <h1 className="heading-serif text-4xl text-ink-100 mt-1">Dominios</h1>
          <p className="text-sm text-ink-400 mt-1">
            Registra dominios, verifica propiedad con TXT y configura reglas por URL para la caché.
          </p>
        </div>
        <div className="flex items-center gap-2">
          <Button
            variant="secondary"
            size="md"
            onClick={() => loadDomains(true)}
            loading={refreshing}
            disabled={loading}
          >
            <RefreshCw className="w-3.5 h-3.5" />
            Refrescar
          </Button>
          <Button onClick={() => setCreateOpen(true)} size="md">
            <Plus className="w-4 h-4" />
            Registrar dominio
          </Button>
        </div>
      </header>

      <div className="grid grid-cols-1 sm:grid-cols-3 gap-4">
        <StatCard label="Dominios" value={domains.length} />
        <StatCard
          label="Verificados"
          value={verified}
          accent="success"
          icon={<ShieldCheck className="w-3.5 h-3.5" />}
        />
        <StatCard
          label="Pendientes"
          value={pending}
          accent="warning"
          icon={<ShieldAlert className="w-3.5 h-3.5" />}
        />
      </div>

      {loading ? (
        <Card>
          <Spinner label="Cargando dominios…" />
        </Card>
      ) : domains.length === 0 ? (
        <EmptyState
          icon={Globe}
          title="Aún no tienes dominios"
          description="Registra tu primer dominio para empezar a configurar reglas de caché, API keys y usuarios."
          action={
            <Button onClick={() => setCreateOpen(true)}>
              <Plus className="w-4 h-4" /> Registrar dominio
            </Button>
          }
        />
      ) : (
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
          {domains.map((d) => (
            <DomainCard key={d.id} domain={d} />
          ))}
        </div>
      )}

      {/* Modal de crear dominio */}
      <Modal
        open={createOpen}
        onClose={() => {
          setCreateOpen(false);
          setCreatedInfo(null);
        }}
        title={createdInfo ? 'Verificar propiedad' : 'Registrar un dominio'}
        description={
          createdInfo
            ? 'Crea este registro TXT en tu DNS y luego abre el dominio para verificarlo.'
            : 'Indica el nombre del dominio que quieres administrar.'
        }
        size="md"
        footer={
          createdInfo ? (
            <Button onClick={() => { setCreateOpen(false); setCreatedInfo(null); }}>Listo</Button>
          ) : null
        }
      >
        {createdInfo ? (
          <div className="space-y-4">
            <div className="rounded-md border border-ink-800 bg-ink-900 p-4 space-y-3">
              <div>
                <p className="text-[10px] tracking-[0.18em] uppercase text-ink-500">Host</p>
                <div className="flex items-center justify-between gap-2 mt-1">
                  <code className="mono text-sm text-ink-100 break-all">
                    {createdInfo.dnsVerification.host}
                  </code>
                  <CopyButton value={createdInfo.dnsVerification.host} />
                </div>
              </div>
              <div className="divider" />
              <div>
                <p className="text-[10px] tracking-[0.18em] uppercase text-ink-500">Tipo</p>
                <p className="mono text-sm text-ink-100 mt-1">TXT</p>
              </div>
              <div className="divider" />
              <div>
                <p className="text-[10px] tracking-[0.18em] uppercase text-ink-500">Valor</p>
                <div className="flex items-center justify-between gap-2 mt-1">
                  <code className="mono text-sm text-ink-100 break-all">
                    {createdInfo.dnsVerification.value}
                  </code>
                  <CopyButton value={createdInfo.dnsVerification.value} />
                </div>
              </div>
            </div>
            <p className="text-xs text-ink-400">
              La verificación puede tardar unos minutos en propagarse. Después, abre el dominio y presiona{' '}
              <strong className="text-ink-200">Verificar</strong>.
            </p>
          </div>
        ) : (
          <form onSubmit={onCreate} className="space-y-4">
            <Field label="Nombre del dominio" hint="Por ejemplo: midominio.test, sitio.example.com">
              <Input
                value={newDomainName}
                onChange={(e) => setNewDomainName(e.target.value)}
                placeholder="midominio.test"
                autoFocus
                required
                mono
              />
            </Field>
            <div className="flex items-center justify-end gap-2 pt-2">
              <Button type="button" variant="ghost" onClick={() => setCreateOpen(false)} disabled={creating}>
                Cancelar
              </Button>
              <Button type="submit" loading={creating}>
                Registrar
              </Button>
            </div>
          </form>
        )}
      </Modal>
    </div>
  );
}

function StatCard({ label, value, accent = 'neutral', icon }) {
  const accentClass = {
    neutral: 'text-ink-300',
    success: 'text-success-500',
    warning: 'text-accent-500',
  }[accent];
  return (
    <div className="panel p-5 flex items-center justify-between">
      <div>
        <p className="text-[11px] uppercase tracking-[0.18em] text-ink-500">{label}</p>
        <p className={`text-3xl mt-1 ${accentClass}`}>{value}</p>
      </div>
      {icon && (
        <div className={`p-2.5 rounded-md border border-ink-800 bg-ink-900 ${accentClass}`}>{icon}</div>
      )}
    </div>
  );
}

function DomainCard({ domain }) {
  return (
    <Link
      to={`/domains/${domain.id}`}
      className="panel panel-hover p-5 flex flex-col gap-4 group"
    >
      <div className="flex items-start justify-between gap-2">
        <div className="min-w-0">
          <p className="text-xs text-ink-500 tracking-wide uppercase">Dominio</p>
          <h3 className="mono text-base text-ink-100 break-all mt-0.5 group-hover:text-accent-500 transition-colors">
            {domain.domainName}
          </h3>
        </div>
        {domain.verified ? (
          <Badge variant="success">
            <ShieldCheck className="w-3 h-3" /> Verificado
          </Badge>
        ) : (
          <Badge variant="warning">
            <ShieldAlert className="w-3 h-3" /> Pendiente
          </Badge>
        )}
      </div>

      <div className="text-xs text-ink-500 space-y-1">
        <div className="flex items-center justify-between">
          <span>Creado</span>
          <span className="text-ink-300">{formatTimestamp(domain.createdAt)}</span>
        </div>
        {domain.verifiedAt && (
          <div className="flex items-center justify-between">
            <span>Verificado</span>
            <span className="text-ink-300">{formatTimestamp(domain.verifiedAt)}</span>
          </div>
        )}
        <div className="flex items-center justify-between">
          <span>ID</span>
          <MonoTag>{domain.id.slice(0, 12)}…</MonoTag>
        </div>
      </div>

      <div className="mt-auto flex items-center justify-between text-xs text-ink-400 pt-3 divider">
        <span>Administrar</span>
        <ArrowRight className="w-3.5 h-3.5 transition-transform group-hover:translate-x-0.5" />
      </div>
    </Link>
  );
}