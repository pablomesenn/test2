import { config } from '../config.js';
import { getIdToken } from './firebase.js';

/**
 * Error de API enriquecido con statusCode para que la UI pueda diferenciar
 * 401/403/404/409, mostrar mensajes específicos y manejar el caso "endpoint
 * todavía no implementado en el Node API" (404 con código `not_implemented`).
 */
export class ApiError extends Error {
  constructor(message, statusCode, payload) {
    super(message);
    this.name = 'ApiError';
    this.statusCode = statusCode;
    this.payload = payload;
  }
}

async function request(path, { method = 'GET', body, authMode = 'user', extraHeaders } = {}) {
  const headers = {
    'Accept': 'application/json',
    ...(extraHeaders ?? {}),
  };

  if (body !== undefined) {
    headers['Content-Type'] = 'application/json';
  }

  if (authMode === 'user') {
    const token = await getIdToken();
    if (!token) {
      throw new ApiError('Sesión no iniciada', 401);
    }
    headers['Authorization'] = `Bearer ${token}`;
  }

  let response;
  try {
    response = await fetch(`${config.apiUrl}${path}`, {
      method,
      headers,
      body: body !== undefined ? JSON.stringify(body) : undefined,
    });
  } catch (e) {
    throw new ApiError(`No se pudo conectar al API (${config.apiUrl}): ${e.message}`, 0);
  }

  const text = await response.text();
  let payload = null;
  if (text) {
    try { payload = JSON.parse(text); } catch { payload = { raw: text }; }
  }

  if (!response.ok) {
    const message = payload?.error ?? payload?.message ?? response.statusText ?? 'Error del API';
    throw new ApiError(message, response.status, payload);
  }

  return payload;
}

// -----------------------------------------------------------------------------
// Endpoints que ya existen en el Node API
// -----------------------------------------------------------------------------

export const api = {
  // Salud
  health: () => request('/health', { authMode: 'none' }),

  // Auth (intercambio del ID token; opcional pero el Node API lo expone)
  authExchange: (idToken) => request('/auth/exchange', { method: 'POST', body: { idToken }, authMode: 'none' }),

  // Dominios
  listDomains: () => request('/domains'),
  createDomain: (domainName) => request('/domains', { method: 'POST', body: { domainName } }),
  getDomain: (id) => request(`/domains/${encodeURIComponent(id)}`),
  deleteDomain: (id) => request(`/domains/${encodeURIComponent(id)}`, { method: 'DELETE' }),
  verifyDomain: (id) => request(`/domains/${encodeURIComponent(id)}/verify`, { method: 'POST' }),

  // URLs de un dominio
  listUrls: (domainId) => request(`/domains/${encodeURIComponent(domainId)}/urls`),
  createUrl: (domainId, rule) => request(`/domains/${encodeURIComponent(domainId)}/urls`, { method: 'POST', body: rule }),
  updateUrl: (domainId, urlId, patch) => request(
    `/domains/${encodeURIComponent(domainId)}/urls/${encodeURIComponent(urlId)}`,
    { method: 'PUT', body: patch },
  ),
  deleteUrl: (domainId, urlId) => request(
    `/domains/${encodeURIComponent(domainId)}/urls/${encodeURIComponent(urlId)}`,
    { method: 'DELETE' },
  ),

  // API keys (dominio)
  listApiKeys: (domainId) => request(`/domains/${encodeURIComponent(domainId)}/api-keys`),
  createApiKey: (domainId, label) => request(
    `/domains/${encodeURIComponent(domainId)}/api-keys`,
    { method: 'POST', body: { label } },
  ),
  deleteApiKey: (domainId, keyId) => request(
    `/domains/${encodeURIComponent(domainId)}/api-keys/${encodeURIComponent(keyId)}`,
    { method: 'DELETE' },
  ),

  // -----------------------------------------------------------------------
  // Endpoints esperados (todavía no implementados en el Node API actual).
  // La UI llama estos endpoints; si responden 404, el componente lo muestra.
  // Documentación de contrato esperado más abajo, ver README.
  // -----------------------------------------------------------------------

  // Usuarios para auth user/password (a nivel de dominio).
  listUsers: (domainId) => request(`/domains/${encodeURIComponent(domainId)}/users`),
  createUser: (domainId, { username, password }) => request(
    `/domains/${encodeURIComponent(domainId)}/users`,
    { method: 'POST', body: { username, password } },
  ),
  updateUser: (domainId, userId, patch) => request(
    `/domains/${encodeURIComponent(domainId)}/users/${encodeURIComponent(userId)}`,
    { method: 'PUT', body: patch },
  ),
  deleteUser: (domainId, userId) => request(
    `/domains/${encodeURIComponent(domainId)}/users/${encodeURIComponent(userId)}`,
    { method: 'DELETE' },
  ),

  // Validación de credenciales para el formulario que invocan las cachés zonales.
  // El UI envía username/password y recibe { sessionToken, expiresAt }.
  zonalAuthValidate: ({ domainName, username, password }) => request(
    '/zonal-auth/validate',
    { method: 'POST', body: { domainName, username, password }, authMode: 'none' },
  ),
};
