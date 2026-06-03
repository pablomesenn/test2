import { useEffect, useState } from 'react';
import { useNavigate, useParams } from 'react-router-dom';
import {
  ArrowLeft, ShieldCheck, ShieldAlert, Trash2, RefreshCw, Plus, Edit3,
  Globe, KeyRound, Users, Pencil, Wand2,
} from 'lucide-react';
import { api, ApiError } from '../lib/api.js';
import { useToast } from '../context/ToastContext.jsx';
import {
  Button, Card, Field, Input, Select, Spinner, Badge, EmptyState, MonoTag, CopyButton,
} from '../components/ui.jsx';
import { Modal } from '../components/Modal.jsx';
import { ConfirmDialog } from '../components/ConfirmDialog.jsx';
import { Tabs } from '../components/Tabs.jsx';
import {
  formatBytes, formatTtl, formatTimestamp, parseBytesInput, truncateMiddle,
  REPLACEMENT_POLICIES, AUTH_MODES, COMMON_CONTENT_TYPES,
} from '../lib/formatters.js';

export function DomainDetailPage() {
  const { id } = useParams();
  const navigate = useNavigate();
  const toast = useToast();

  const [domain, setDomain] = useState(null);
  const [loading, setLoading] = useState(true);
  const [tab, setTab] = useState('urls');

  const [urls, setUrls] = useState([]);
  const [apiKeys, setApiKeys] = useState([]);
  const [users, setUsers] = useState([]);
  const [usersUnavailable, setUsersUnavailable] = useState(false);

  const [verifying, setVerifying] = useState(false);
  const [confirmDelete, setConfirmDelete] = useState(false);
  const [deleting, setDeleting] = useState(false);

  const loadAll = async () => {
    setLoading(true);
    try {
      // El Node API a veces no expone GET /domains/:id correctamente.
      // Si falla con 404, caemos de regreso a listDomains y buscamos por id.
      const domainPromise = api.getDomain(id).catch(async (err) => {
        if (err instanceof ApiError && err.statusCode === 404) {
          const list = await api.listDomains();
          const found = (list.domains ?? []).find((d) => d.id === id);
          if (found) return { domain: found };
          throw err;
        }
        throw err;
      });

      const [domainRes, urlsRes, keysRes] = await Promise.all([
        domainPromise,
        api.listUrls(id).catch(() => ({ urls: [] })),
        api.listApiKeys(id).catch(() => ({ apiKeys: [] })),
      ]);
      setDomain(domainRes.domain ?? domainRes);
      setUrls(urlsRes.urls ?? []);
      setApiKeys(keysRes.apiKeys ?? []);

      // Endpoint de usuarios: puede no existir en el Node API actual.
      try {
        const usersRes = await api.listUsers(id);
        setUsers(usersRes.users ?? []);
        setUsersUnavailable(false);
      } catch (err) {
        if (err instanceof ApiError && (err.statusCode === 404 || err.statusCode === 501)) {
          setUsersUnavailable(true);
          setUsers([]);
        } else {
          throw err;
        }
      }
    } catch (err) {
      toast({ kind: 'error', title: 'Error al cargar el dominio', message: err.message });
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    loadAll();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [id]);

  const onVerify = async () => {
    setVerifying(true);
    try {
      await api.verifyDomain(id);
      toast({ kind: 'success', title: 'Dominio verificado' });
      await loadAll();
    } catch (err) {
      toast({
        kind: 'error',
        title: 'No se pudo verificar',
        message: err.message + ' Asegúrate de que el TXT esté propagado.',
      });
    } finally {
      setVerifying(false);
    }
  };

  const onDelete = async () => {
    setDeleting(true);
    try {
      await api.deleteDomain(id);
      toast({ kind: 'success', title: 'Dominio eliminado' });
      navigate('/dashboard', { replace: true });
    } catch (err) {
      toast({ kind: 'error', title: 'No se pudo eliminar', message: err.message });
    } finally {
      setDeleting(false);
      setConfirmDelete(false);
    }
  };

  if (loading && !domain) {
    return (
      <div className="py-12">
        <Spinner label="Cargando dominio…" />
      </div>
    );
  }

  if (!domain) {
    return (
      <EmptyState
        title="Dominio no encontrado"
        description="Puede haber sido eliminado o no tienes acceso."
        action={<Button onClick={() => navigate('/dashboard')}>Volver</Button>}
      />
    );
  }

  return (
    <div className="space-y-8">
      <div>
        <button
          onClick={() => navigate('/dashboard')}
          className="text-xs text-ink-400 hover:text-ink-100 inline-flex items-center gap-1 transition-colors"
        >
          <ArrowLeft className="w-3 h-3" /> Volver a dominios
        </button>
      </div>

      <header className="flex flex-wrap items-start justify-between gap-4">
        <div className="space-y-1.5">
          <p className="text-[11px] uppercase tracking-[0.2em] text-ink-500">Dominio</p>
          <h1 className="heading-serif text-4xl mono not-italic text-ink-100 break-all">{domain.domainName}</h1>
          <div className="flex items-center gap-2 flex-wrap">
            {domain.verified ? (
              <Badge variant="success">
                <ShieldCheck className="w-3 h-3" /> Verificado
              </Badge>
            ) : (
              <Badge variant="warning">
                <ShieldAlert className="w-3 h-3" /> Pendiente de verificación
              </Badge>
            )}
            <Badge variant="neutral">ID: <code className="mono ml-1">{truncateMiddle(domain.id, 20)}</code></Badge>
            <Badge variant="neutral">Creado: {formatTimestamp(domain.createdAt)}</Badge>
          </div>
        </div>
        <div className="flex items-center gap-2">
          <Button variant="secondary" onClick={loadAll}>
            <RefreshCw className="w-3.5 h-3.5" /> Refrescar
          </Button>
          <Button variant="danger-outline" onClick={() => setConfirmDelete(true)}>
            <Trash2 className="w-3.5 h-3.5" /> Eliminar
          </Button>
        </div>
      </header>

      {!domain.verified && (
        <VerificationPanel domain={domain} onVerify={onVerify} verifying={verifying} />
      )}

      <Tabs
        tabs={[
          { value: 'urls', label: 'Reglas de URL', icon: Globe, count: urls.length },
          { value: 'keys', label: 'API Keys', icon: KeyRound, count: apiKeys.length },
          { value: 'users', label: 'Usuarios', icon: Users, count: usersUnavailable ? '—' : users.length },
        ]}
        current={tab}
        onChange={setTab}
      />

      <div className="animate-fadeIn">
        {tab === 'urls' && (
          <UrlRulesTab domainId={id} urls={urls} reload={loadAll} toast={toast} />
        )}
        {tab === 'keys' && (
          <ApiKeysTab domainId={id} keys={apiKeys} reload={loadAll} toast={toast} />
        )}
        {tab === 'users' && (
          <UsersTab
            domainId={id}
            users={users}
            unavailable={usersUnavailable}
            reload={loadAll}
            toast={toast}
          />
        )}
      </div>

      <ConfirmDialog
        open={confirmDelete}
        onClose={() => setConfirmDelete(false)}
        onConfirm={onDelete}
        loading={deleting}
        title="Eliminar dominio"
        description={`Esto eliminará "${domain.domainName}" y toda su configuración (URLs, claves, usuarios). No se puede deshacer.`}
      />
    </div>
  );
}

// -----------------------------------------------------------------------------
// Panel: verificación de TXT
// -----------------------------------------------------------------------------

function VerificationPanel({ domain, onVerify, verifying }) {
  const host = `_ic7602-verify.${domain.domainName}`;
  return (
    <Card className="border-accent-500/30">
      <div className="flex flex-col md:flex-row gap-4 md:items-center">
        <div className="p-2.5 rounded-md bg-accent-500/10 border border-accent-500/30 text-accent-500 shrink-0">
          <ShieldAlert className="w-5 h-5" />
        </div>
        <div className="flex-1 space-y-2">
          <p className="text-sm text-ink-100">
            <span className="heading-serif italic text-lg text-ink-100">Falta verificar la propiedad.</span>
            <span className="block text-ink-400 text-sm mt-1">
              Crea el siguiente registro en DNS y luego presiona verificar. Puede tardar unos minutos en
              propagarse.
            </span>
          </p>
          <div className="rounded-md border border-ink-800 bg-ink-900 p-3 grid grid-cols-1 md:grid-cols-3 gap-3 text-xs">
            <KV label="Tipo" value="TXT" />
            <KV label="Host" value={host} copyable />
            <KV label="Valor" value={domain.verificationToken ?? '—'} copyable />
          </div>
        </div>
        <Button onClick={onVerify} loading={verifying}>
          <ShieldCheck className="w-3.5 h-3.5" /> Verificar ahora
        </Button>
      </div>
    </Card>
  );
}

function KV({ label, value, copyable = false }) {
  return (
    <div className="space-y-1 min-w-0">
      <p className="text-[10px] tracking-[0.18em] uppercase text-ink-500">{label}</p>
      <div className="flex items-center gap-2 min-w-0">
        <code className="mono text-ink-100 truncate" title={value}>
          {value}
        </code>
        {copyable && <CopyButton value={value} label="" />}
      </div>
    </div>
  );
}

// =============================================================================
// Tab: URLs
// =============================================================================

const DEFAULT_RULE = {
  pattern: '/',
  cacheMaxSizeBytes: 10 * 1024 * 1024,
  cacheMaxSizeInput: '10 MB',
  ttlSeconds: 3600,
  replacementPolicy: 'LRU',
  contentTypes: ['html', 'css', 'js'],
  authMode: 'none',
  wildcard: false,
};

function UrlRulesTab({ domainId, urls, reload, toast }) {
  const [openForm, setOpenForm] = useState(false);
  const [editing, setEditing] = useState(null);
  const [pendingDelete, setPendingDelete] = useState(null);
  const [deleting, setDeleting] = useState(false);

  const onCreate = () => {
    setEditing(null);
    setOpenForm(true);
  };

  const onEdit = (rule) => {
    setEditing(rule);
    setOpenForm(true);
  };

  const onDelete = async () => {
    if (!pendingDelete) return;
    setDeleting(true);
    try {
      await api.deleteUrl(domainId, pendingDelete.id);
      toast({ kind: 'success', title: 'Regla eliminada' });
      await reload();
    } catch (err) {
      toast({ kind: 'error', title: 'No se pudo eliminar', message: err.message });
    } finally {
      setDeleting(false);
      setPendingDelete(null);
    }
  };

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between">
        <p className="text-sm text-ink-400">
          Define el patrón de URL, política de caché, content-types permitidos y el modo de autenticación.
          Soporta wildcards.
        </p>
        <Button onClick={onCreate}>
          <Plus className="w-4 h-4" /> Nueva regla
        </Button>
      </div>

      {urls.length === 0 ? (
        <EmptyState
          icon={Globe}
          title="Sin reglas todavía"
          description="Cuando agregues reglas, las cachés zonales sabrán cómo tratar cada URL."
          action={<Button onClick={onCreate}><Plus className="w-4 h-4" /> Nueva regla</Button>}
        />
      ) : (
        <div className="panel divide-y divide-ink-800">
          {urls.map((rule) => (
            <UrlRow
              key={rule.id}
              rule={rule}
              onEdit={() => onEdit(rule)}
              onDelete={() => setPendingDelete(rule)}
            />
          ))}
        </div>
      )}

      <UrlRuleForm
        open={openForm}
        editing={editing}
        domainId={domainId}
        onClose={() => setOpenForm(false)}
        onSaved={async () => { setOpenForm(false); await reload(); }}
        toast={toast}
      />

      <ConfirmDialog
        open={!!pendingDelete}
        onClose={() => setPendingDelete(null)}
        onConfirm={onDelete}
        loading={deleting}
        title="Eliminar regla"
        description={pendingDelete ? `Eliminar la regla "${pendingDelete.pattern}". ¿Continuar?` : ''}
      />
    </div>
  );
}

function UrlRow({ rule, onEdit, onDelete }) {
  const authMeta = AUTH_MODES.find((a) => a.value === rule.authMode) ?? AUTH_MODES[0];
  return (
    <div className="px-5 py-4 grid grid-cols-1 md:grid-cols-12 gap-3 items-center">
      <div className="md:col-span-4 min-w-0">
        <div className="flex items-center gap-2 flex-wrap">
          <code className="mono text-sm text-ink-100 break-all">{rule.pattern}</code>
          {rule.wildcard && (
            <Badge variant="info">
              <Wand2 className="w-3 h-3" /> wildcard
            </Badge>
          )}
        </div>
        {rule.contentTypes?.length > 0 && (
          <div className="flex items-center gap-1 mt-1 flex-wrap">
            {rule.contentTypes.slice(0, 6).map((ct) => (
              <MonoTag key={ct} className="!text-[10px]">{ct}</MonoTag>
            ))}
            {rule.contentTypes.length > 6 && (
              <span className="text-[10px] text-ink-500">+{rule.contentTypes.length - 6}</span>
            )}
          </div>
        )}
      </div>
      <Cell label="Tamaño" value={formatBytes(rule.cacheMaxSizeBytes)} />
      <Cell label="TTL" value={formatTtl(rule.ttlSeconds)} />
      <Cell label="Política" value={<MonoTag>{rule.replacementPolicy}</MonoTag>} />
      <Cell label="Auth" value={<Badge variant="neutral">{authMeta.label}</Badge>} />
      <div className="md:col-span-1 flex items-center justify-end gap-1">
        <Button variant="ghost" size="sm" onClick={onEdit} aria-label="Editar">
          <Edit3 className="w-3.5 h-3.5" />
        </Button>
        <Button variant="ghost" size="sm" onClick={onDelete} aria-label="Eliminar">
          <Trash2 className="w-3.5 h-3.5" />
        </Button>
      </div>
    </div>
  );
}

function Cell({ label, value }) {
  return (
    <div className="md:col-span-2 min-w-0">
      <p className="text-[10px] uppercase tracking-[0.16em] text-ink-500">{label}</p>
      <div className="mt-0.5 text-sm text-ink-200">{value}</div>
    </div>
  );
}

function UrlRuleForm({ open, editing, domainId, onClose, onSaved, toast }) {
  const [form, setForm] = useState(DEFAULT_RULE);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState('');

  useEffect(() => {
    if (open) {
      if (editing) {
        setForm({
          pattern: editing.pattern ?? '/',
          cacheMaxSizeBytes: editing.cacheMaxSizeBytes ?? 0,
          cacheMaxSizeInput: formatBytes(editing.cacheMaxSizeBytes ?? 0),
          ttlSeconds: editing.ttlSeconds ?? 0,
          replacementPolicy: editing.replacementPolicy ?? 'LRU',
          contentTypes: editing.contentTypes ?? [],
          authMode: editing.authMode ?? 'none',
          wildcard: Boolean(editing.wildcard),
        });
      } else {
        setForm(DEFAULT_RULE);
      }
      setError('');
    }
  }, [open, editing]);

  const update = (patch) => setForm((c) => ({ ...c, ...patch }));
  const toggleType = (t) =>
    update({
      contentTypes: form.contentTypes.includes(t)
        ? form.contentTypes.filter((x) => x !== t)
        : [...form.contentTypes, t],
    });

  const onSubmit = async (e) => {
    e.preventDefault();
    setError('');
    const bytes = parseBytesInput(form.cacheMaxSizeInput);
    if (Number.isNaN(bytes) || bytes < 0) {
      setError('Tamaño de caché inválido. Usa por ejemplo "10 MB" o "1024".');
      return;
    }
    const payload = {
      pattern: form.pattern.trim(),
      cacheMaxSizeBytes: bytes,
      ttlSeconds: Number(form.ttlSeconds) || 0,
      replacementPolicy: form.replacementPolicy,
      contentTypes: form.contentTypes,
      authMode: form.authMode,
      wildcard: form.wildcard,
    };
    setSaving(true);
    try {
      if (editing) {
        await api.updateUrl(domainId, editing.id, payload);
        toast({ kind: 'success', title: 'Regla actualizada' });
      } else {
        await api.createUrl(domainId, payload);
        toast({ kind: 'success', title: 'Regla creada' });
      }
      await onSaved();
    } catch (err) {
      setError(err.message);
    } finally {
      setSaving(false);
    }
  };

  return (
    <Modal
      open={open}
      onClose={onClose}
      title={editing ? 'Editar regla de URL' : 'Nueva regla de URL'}
      description={editing ? editing.pattern : 'Definí cómo se debe cachear esta URL.'}
      size="lg"
      footer={
        <>
          <Button variant="ghost" onClick={onClose} disabled={saving}>
            Cancelar
          </Button>
          <Button onClick={onSubmit} loading={saving}>
            {editing ? 'Guardar cambios' : 'Crear regla'}
          </Button>
        </>
      }
    >
      <form onSubmit={onSubmit} className="space-y-5">
        <Field label="Patrón de URL" hint="Ejemplo: /api/*, /static/**/*.png, /">
          <Input
            value={form.pattern}
            onChange={(e) => update({ pattern: e.target.value })}
            placeholder="/api/*"
            required
            mono
          />
        </Field>

        <div className="grid grid-cols-1 sm:grid-cols-2 gap-4">
          <Field label="Tamaño máximo de caché" hint="Acepta B, KB, MB, GB.">
            <Input
              value={form.cacheMaxSizeInput}
              onChange={(e) => update({ cacheMaxSizeInput: e.target.value })}
              placeholder="10 MB"
              mono
            />
          </Field>
          <Field label="TTL (segundos)" hint="0 = sin expiración por tiempo">
            <Input
              type="number"
              min={0}
              value={form.ttlSeconds}
              onChange={(e) => update({ ttlSeconds: e.target.value })}
            />
          </Field>
        </div>

        <div className="grid grid-cols-1 sm:grid-cols-2 gap-4">
          <Field label="Política de reemplazo">
            <Select
              value={form.replacementPolicy}
              onChange={(e) => update({ replacementPolicy: e.target.value })}
            >
              {REPLACEMENT_POLICIES.map((p) => (
                <option key={p} value={p}>{p}</option>
              ))}
            </Select>
          </Field>
          <Field label="Mecanismo de autenticación">
            <Select value={form.authMode} onChange={(e) => update({ authMode: e.target.value })}>
              {AUTH_MODES.map((a) => (
                <option key={a.value} value={a.value}>{a.label}</option>
              ))}
            </Select>
          </Field>
        </div>

        <Field label="Content-types" hint="Tipos de archivos cacheables. Selecciona los que apliquen.">
          <div className="flex flex-wrap gap-2 pt-1">
            {COMMON_CONTENT_TYPES.map((t) => {
              const active = form.contentTypes.includes(t);
              return (
                <button
                  type="button"
                  key={t}
                  onClick={() => toggleType(t)}
                  className={[
                    'mono text-xs px-2.5 py-1 rounded border transition-colors',
                    active
                      ? 'border-accent-500 bg-accent-500/10 text-accent-400'
                      : 'border-ink-800 text-ink-400 hover:border-ink-700 hover:text-ink-200',
                  ].join(' ')}
                >
                  {t}
                </button>
              );
            })}
          </div>
        </Field>

        <label className="flex items-center gap-2 select-none cursor-pointer">
          <input
            type="checkbox"
            checked={form.wildcard}
            onChange={(e) => update({ wildcard: e.target.checked })}
            className="accent-accent-500"
          />
          <span className="text-sm text-ink-200">Habilitar wildcard</span>
          <span className="text-xs text-ink-500">— interpretar <code className="mono">*</code> en el patrón</span>
        </label>

        {error && (
          <div className="rounded-md border border-danger-500/30 bg-danger-500/5 px-3 py-2 text-xs text-danger-400">
            {error}
          </div>
        )}
      </form>
    </Modal>
  );
}

// =============================================================================
// Tab: API Keys
// =============================================================================

function ApiKeysTab({ domainId, keys, reload, toast }) {
  const [openCreate, setOpenCreate] = useState(false);
  const [label, setLabel] = useState('cache-zonal');
  const [creating, setCreating] = useState(false);
  const [createdKey, setCreatedKey] = useState(null);

  const [pendingDelete, setPendingDelete] = useState(null);
  const [deleting, setDeleting] = useState(false);

  const onCreate = async (e) => {
    e.preventDefault();
    setCreating(true);
    try {
      const res = await api.createApiKey(domainId, label.trim() || 'api-key');
      setCreatedKey(res);
      setLabel('cache-zonal');
      await reload();
      toast({ kind: 'success', title: 'API key creada' });
    } catch (err) {
      toast({ kind: 'error', title: 'No se pudo crear la API key', message: err.message });
    } finally {
      setCreating(false);
    }
  };

  const onDelete = async () => {
    if (!pendingDelete) return;
    setDeleting(true);
    try {
      await api.deleteApiKey(domainId, pendingDelete.id);
      toast({ kind: 'success', title: 'API key eliminada' });
      await reload();
    } catch (err) {
      toast({ kind: 'error', title: 'No se pudo eliminar', message: err.message });
    } finally {
      setDeleting(false);
      setPendingDelete(null);
    }
  };

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between">
        <p className="text-sm text-ink-400">
          Estas API keys se envían en el header <MonoTag>x-api-key</MonoTag> a las cachés zonales para
          autenticar peticiones en URLs que requieran este modo.
        </p>
        <Button onClick={() => { setOpenCreate(true); setCreatedKey(null); }}>
          <Plus className="w-4 h-4" /> Nueva API key
        </Button>
      </div>

      {keys.length === 0 ? (
        <EmptyState
          icon={KeyRound}
          title="Sin API keys"
          description="Crea una para que las cachés zonales puedan autenticarse en URLs configuradas con API Key."
          action={
            <Button onClick={() => { setOpenCreate(true); setCreatedKey(null); }}>
              <Plus className="w-4 h-4" /> Nueva API key
            </Button>
          }
        />
      ) : (
        <div className="panel divide-y divide-ink-800">
          {keys.map((k) => (
            <div key={k.id} className="px-5 py-4 flex items-center justify-between gap-3">
              <div className="min-w-0">
                <p className="text-sm text-ink-100">{k.label}</p>
                <div className="flex items-center gap-2 mt-1 flex-wrap text-xs text-ink-500">
                  <MonoTag>{truncateMiddle(k.keyHash ?? k.id, 28)}</MonoTag>
                  <span>·</span>
                  <span>{formatTimestamp(k.createdAt)}</span>
                  {k.active === false && <Badge variant="danger">Inactiva</Badge>}
                </div>
              </div>
              <Button variant="ghost" size="sm" onClick={() => setPendingDelete(k)}>
                <Trash2 className="w-3.5 h-3.5" />
                Eliminar
              </Button>
            </div>
          ))}
        </div>
      )}

      <Modal
        open={openCreate}
        onClose={() => { setOpenCreate(false); setCreatedKey(null); }}
        title={createdKey ? 'API key creada' : 'Nueva API key'}
        description={
          createdKey
            ? 'Copia el valor ahora. No volveremos a mostrarlo.'
            : 'Etiqueta esta API key para identificarla después.'
        }
        size="md"
        footer={
          createdKey ? (
            <Button onClick={() => { setOpenCreate(false); setCreatedKey(null); }}>Listo</Button>
          ) : null
        }
      >
        {createdKey ? (
          <div className="space-y-3">
            <div className="rounded-md border border-accent-500/30 bg-accent-500/5 p-4">
              <p className="text-[10px] uppercase tracking-[0.18em] text-accent-500 mb-2">Valor (una sola vez)</p>
              <div className="flex items-center gap-2">
                <code className="mono text-sm text-ink-100 break-all flex-1">{createdKey.apiKey}</code>
                <CopyButton value={createdKey.apiKey} />
              </div>
            </div>
            <p className="text-xs text-ink-400">
              Guárdala en un lugar seguro. Si la pierdes, deberás crear una nueva.
            </p>
          </div>
        ) : (
          <form onSubmit={onCreate} className="space-y-4">
            <Field label="Etiqueta">
              <Input
                value={label}
                onChange={(e) => setLabel(e.target.value)}
                placeholder="cache-zonal-cr"
                autoFocus
              />
            </Field>
            <div className="flex items-center justify-end gap-2 pt-2">
              <Button type="button" variant="ghost" onClick={() => setOpenCreate(false)} disabled={creating}>
                Cancelar
              </Button>
              <Button type="submit" loading={creating}>
                Crear API key
              </Button>
            </div>
          </form>
        )}
      </Modal>

      <ConfirmDialog
        open={!!pendingDelete}
        onClose={() => setPendingDelete(null)}
        onConfirm={onDelete}
        loading={deleting}
        title="Eliminar API key"
        description={
          pendingDelete
            ? `Esta acción es permanente. Las cachés que usen "${pendingDelete.label}" dejarán de autenticar.`
            : ''
        }
      />
    </div>
  );
}

// =============================================================================
// Tab: Users
// =============================================================================

function UsersTab({ domainId, users, unavailable, reload, toast }) {
  const [openForm, setOpenForm] = useState(false);
  const [editing, setEditing] = useState(null);
  const [pendingDelete, setPendingDelete] = useState(null);
  const [deleting, setDeleting] = useState(false);

  if (unavailable) {
    return (
      <Card>
        <div className="flex items-start gap-3">
          <div className="p-2 rounded-md bg-accent-500/10 border border-accent-500/30 text-accent-500 shrink-0">
            <Users className="w-4 h-4" />
          </div>
          <div className="space-y-2">
            <p className="heading-serif italic text-xl text-ink-100">Endpoints de usuarios pendientes.</p>
            <p className="text-sm text-ink-300">
              El Node API todavía no expone los endpoints para administrar usuarios por dominio. La UI ya está
              lista para consumirlos cuando se agreguen. Contrato esperado:
            </p>
            <pre className="mono text-xs bg-ink-900 border border-ink-800 rounded-md p-3 overflow-x-auto text-ink-300">
{`GET    /api/domains/:id/users
POST   /api/domains/:id/users          { username, password }
PUT    /api/domains/:id/users/:userId  { username?, password? }
DELETE /api/domains/:id/users/:userId`}
            </pre>
          </div>
        </div>
      </Card>
    );
  }

  const onDelete = async () => {
    if (!pendingDelete) return;
    setDeleting(true);
    try {
      await api.deleteUser(domainId, pendingDelete.id);
      toast({ kind: 'success', title: 'Usuario eliminado' });
      await reload();
    } catch (err) {
      toast({ kind: 'error', title: 'No se pudo eliminar', message: err.message });
    } finally {
      setDeleting(false);
      setPendingDelete(null);
    }
  };

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between">
        <p className="text-sm text-ink-400">
          Estas credenciales se usan cuando una URL tiene el modo <strong>Usuario y contraseña</strong>. La
          caché redirige al formulario de la UI para validarlas.
        </p>
        <Button onClick={() => { setEditing(null); setOpenForm(true); }}>
          <Plus className="w-4 h-4" /> Nuevo usuario
        </Button>
      </div>

      {users.length === 0 ? (
        <EmptyState
          icon={Users}
          title="Sin usuarios"
          description="Crea uno para autenticar URLs que usan usuario/contraseña."
          action={
            <Button onClick={() => { setEditing(null); setOpenForm(true); }}>
              <Plus className="w-4 h-4" /> Nuevo usuario
            </Button>
          }
        />
      ) : (
        <div className="panel divide-y divide-ink-800">
          {users.map((u) => (
            <div key={u.id} className="px-5 py-4 flex items-center justify-between gap-3">
              <div className="min-w-0">
                <p className="mono text-sm text-ink-100">{u.username}</p>
                <p className="text-xs text-ink-500 mt-0.5">
                  Creado {formatTimestamp(u.createdAt)}
                </p>
              </div>
              <div className="flex items-center gap-1">
                <Button variant="ghost" size="sm" onClick={() => { setEditing(u); setOpenForm(true); }}>
                  <Pencil className="w-3.5 h-3.5" /> Editar
                </Button>
                <Button variant="ghost" size="sm" onClick={() => setPendingDelete(u)}>
                  <Trash2 className="w-3.5 h-3.5" /> Eliminar
                </Button>
              </div>
            </div>
          ))}
        </div>
      )}

      <UserForm
        open={openForm}
        editing={editing}
        domainId={domainId}
        onClose={() => setOpenForm(false)}
        onSaved={async () => { setOpenForm(false); await reload(); }}
        toast={toast}
      />

      <ConfirmDialog
        open={!!pendingDelete}
        onClose={() => setPendingDelete(null)}
        onConfirm={onDelete}
        loading={deleting}
        title="Eliminar usuario"
        description={pendingDelete ? `Eliminar "${pendingDelete.username}". ¿Continuar?` : ''}
      />
    </div>
  );
}

function UserForm({ open, editing, domainId, onClose, onSaved, toast }) {
  const [username, setUsername] = useState('');
  const [password, setPassword] = useState('');
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState('');

  useEffect(() => {
    if (open) {
      setUsername(editing?.username ?? '');
      setPassword('');
      setError('');
    }
  }, [open, editing]);

  const onSubmit = async (e) => {
    e.preventDefault();
    setError('');
    setSaving(true);
    try {
      if (editing) {
        const patch = {};
        if (username && username !== editing.username) patch.username = username;
        if (password) patch.password = password;
        if (Object.keys(patch).length === 0) {
          setError('Sin cambios.');
          setSaving(false);
          return;
        }
        await api.updateUser(domainId, editing.id, patch);
        toast({ kind: 'success', title: 'Usuario actualizado' });
      } else {
        if (!username || !password) {
          setError('Usuario y contraseña son obligatorios.');
          setSaving(false);
          return;
        }
        await api.createUser(domainId, { username, password });
        toast({ kind: 'success', title: 'Usuario creado' });
      }
      await onSaved();
    } catch (err) {
      setError(err.message);
    } finally {
      setSaving(false);
    }
  };

  return (
    <Modal
      open={open}
      onClose={onClose}
      title={editing ? 'Editar usuario' : 'Nuevo usuario'}
      size="md"
      footer={
        <>
          <Button variant="ghost" onClick={onClose} disabled={saving}>Cancelar</Button>
          <Button onClick={onSubmit} loading={saving}>{editing ? 'Guardar' : 'Crear'}</Button>
        </>
      }
    >
      <form onSubmit={onSubmit} className="space-y-4">
        <Field label="Usuario">
          <Input
            value={username}
            onChange={(e) => setUsername(e.target.value)}
            placeholder="ana"
            autoFocus
            mono
          />
        </Field>
        <Field label={editing ? 'Nueva contraseña (opcional)' : 'Contraseña'}>
          <Input
            type="password"
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
      </form>
    </Modal>
  );
}