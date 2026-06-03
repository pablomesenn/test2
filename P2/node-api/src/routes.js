import dns from 'node:dns/promises';
import { createHash, randomUUID } from 'node:crypto';
import { config } from './config.js';
import { handleCorsPreflight, readJson, sendError, sendJson, getPathname } from './http.js';
import {
  createApiKey,
  createDomainRecord,
  createUrlRule,
  deleteApiKey,
  deleteDomain,
  deleteUrlRule,
  findDomainByName,
  findDomainByDomainName,
  getDomainById,
  listApiKeys,
  listDomains,
  listUrlRules,
  updateDomain,
  updateUrlRule,
} from './store.js';
import { verifyIdToken, getAdminApp } from './firebase.js';

function getBearerToken(req) {
  const header = req.headers.authorization ?? req.headers.Authorization;
  if (!header || !header.startsWith('Bearer ')) {
    return '';
  }

  return header.slice('Bearer '.length).trim();
}

async function requireUser(req) {
  const token = getBearerToken(req);
  if (!token) {
    throw Object.assign(new Error('Falta el token Bearer'), { statusCode: 401 });
  }

  return verifyIdToken(token, true);
}

async function requireServiceKey(req) {
  const apiKey = req.headers['x-api-key'];
  if (!config.cacheServiceApiKey) {
    throw Object.assign(new Error('CACHE_SERVICE_API_KEY no está configurada'), { statusCode: 500 });
  }

  if (apiKey !== config.cacheServiceApiKey) {
    throw Object.assign(new Error('API key inválida'), { statusCode: 401 });
  }
}

function normalizeDomainName(domainName) {
  return String(domainName ?? '').trim().toLowerCase();
}

function assertDomainName(domainName) {
  if (!domainName || !/^[a-z0-9.-]+\.[a-z]{2,}$/i.test(domainName)) {
    throw Object.assign(new Error('Dominio inválido'), { statusCode: 400 });
  }
}

function assertRulePayload(payload) {
  if (!payload.pattern) {
    throw Object.assign(new Error('Falta pattern'), { statusCode: 400 });
  }

  if (payload.ttlSeconds !== undefined && Number.isNaN(Number(payload.ttlSeconds))) {
    throw Object.assign(new Error('ttlSeconds debe ser numérico'), { statusCode: 400 });
  }
}

function buildVerificationHost(domainName) {
  return `_ic7602-verify.${domainName}`;
}

function buildVerificationToken() {
  return `ic7602-${randomUUID().replace(/-/g, '')}`;
}

function hashSecret(secret) {
  return createHash('sha256').update(secret).digest('hex');
}

function serializeFirebaseUser(userRecord) {
  return {
    uid: userRecord.uid,
    email: userRecord.email ?? null,
    displayName: userRecord.displayName ?? null,
    photoURL: userRecord.photoURL ?? null,
    emailVerified: Boolean(userRecord.emailVerified),
    disabled: Boolean(userRecord.disabled),
    metadata: {
      creationTime: userRecord.metadata?.creationTime ?? null,
      lastSignInTime: userRecord.metadata?.lastSignInTime ?? null,
    },
  };
}

function mapFirebaseAuthError(error) {
  switch (error?.code) {
    case 'auth/email-already-exists':
      return [409, 'El correo ya está registrado'];
    case 'auth/user-not-found':
      return [404, 'Usuario no encontrado'];
    case 'auth/invalid-email':
      return [400, 'Correo inválido'];
    case 'auth/invalid-password':
    case 'auth/weak-password':
      return [400, 'La contraseña debe tener al menos 6 caracteres'];
    case 'auth/operation-not-allowed':
      return [403, 'Operación no permitida'];
    default:
      return [500, 'Error al procesar autenticación'];
  }
}

function firebaseAuthErrorDetails(error) {
  return {
    code: error?.code ?? null,
    message: error?.message ?? null,
  };
}

function assertEmailPasswordPayload(body, { requirePassword = true } = {}) {
  if (!body || typeof body !== 'object' || Array.isArray(body)) {
    throw Object.assign(new Error('Cuerpo JSON inválido'), { statusCode: 400 });
  }

  if (!body.email || typeof body.email !== 'string') {
    throw Object.assign(new Error('Falta email'), { statusCode: 400 });
  }

  if (requirePassword && (!body.password || typeof body.password !== 'string')) {
    throw Object.assign(new Error('Falta password'), { statusCode: 400 });
  }

  if (body.password !== undefined && String(body.password).length < 6) {
    throw Object.assign(new Error('La contraseña debe tener al menos 6 caracteres'), { statusCode: 400 });
  }
}

async function verifyTxtRecord(domainName, expectedToken) {
  const records = await dns.resolveTxt(buildVerificationHost(domainName));
  const values = records.flat().map((entry) => String(entry).trim());
  return values.includes(expectedToken);
}

async function handleHealth(req, res) {
  sendJson(res, 200, {
    ok: true,
    service: 'node-api',
    nodeApiUrl: config.nodeApiUrl || null,
  });
}

async function handleSignupUser(req, res) {
  const body = await readJson(req);
  assertEmailPasswordPayload(body);

  try {
    const userRecord = await getAdminApp().auth().createUser({
      email: String(body.email).trim(),
      password: String(body.password),
      displayName: body.displayName !== undefined ? String(body.displayName).trim() : undefined,
    });

    return sendJson(res, 201, {
      user: serializeFirebaseUser(userRecord),
    });
  } catch (error) {
    const [statusCode, message] = mapFirebaseAuthError(error);
    return sendError(res, statusCode, message, firebaseAuthErrorDetails(error));
  }
}

async function handleAuthExchange(req, res) {
  const body = await readJson(req);
  if (!body.idToken) {
    return sendError(res, 400, 'Falta idToken');
  }

  const decoded = await verifyIdToken(body.idToken);
  return sendJson(res, 200, {
    uid: decoded.uid,
    email: decoded.email ?? null,
    name: decoded.name ?? null,
  });
}

async function handleGetCurrentUser(req, res, user) {
  try {
    const userRecord = await getAdminApp().auth().getUser(user.uid);
    return sendJson(res, 200, {
      user: serializeFirebaseUser(userRecord),
    });
  } catch (error) {
    const [statusCode, message] = mapFirebaseAuthError(error);
    return sendError(res, statusCode, message, firebaseAuthErrorDetails(error));
  }
}

async function handleUpdateCurrentUser(req, res, user) {
  const body = await readJson(req);
  if (!body || typeof body !== 'object' || Array.isArray(body)) {
    return sendError(res, 400, 'Cuerpo JSON inválido');
  }

  const updatePayload = {};
  if (body.email !== undefined) {
    updatePayload.email = String(body.email).trim();
  }
  if (body.password !== undefined) {
    updatePayload.password = String(body.password);
  }
  if (body.displayName !== undefined) {
    updatePayload.displayName = String(body.displayName).trim();
  }

  if (Object.keys(updatePayload).length === 0) {
    return sendError(res, 400, 'No hay campos para actualizar');
  }

  if (updatePayload.password !== undefined && updatePayload.password.length < 6) {
    return sendError(res, 400, 'La contraseña debe tener al menos 6 caracteres');
  }

  try {
    const userRecord = await getAdminApp().auth().updateUser(user.uid, updatePayload);
    return sendJson(res, 200, {
      user: serializeFirebaseUser(userRecord),
    });
  } catch (error) {
    const [statusCode, message] = mapFirebaseAuthError(error);
    return sendError(res, statusCode, message, firebaseAuthErrorDetails(error));
  }
}

async function handleDeleteCurrentUser(req, res, user) {
  try {
    await getAdminApp().auth().revokeRefreshTokens(user.uid);
    await getAdminApp().auth().deleteUser(user.uid);
    return sendJson(res, 200, { ok: true });
  } catch (error) {
    const [statusCode, message] = mapFirebaseAuthError(error);
    return sendError(res, statusCode, message, firebaseAuthErrorDetails(error));
  }
}

async function handleLogout(req, res, user) {
  try {
    await getAdminApp().auth().revokeRefreshTokens(user.uid);
    return sendJson(res, 200, { ok: true });
  } catch (error) {
    const [statusCode, message] = mapFirebaseAuthError(error);
    return sendError(res, statusCode, message);
  }
}

async function handleListDomains(req, res, user) {
  const domains = await listDomains(user.uid);
  return sendJson(res, 200, { domains });
}

async function handleCreateDomain(req, res, user) {
  const body = await readJson(req);
  const domainName = normalizeDomainName(body.domainName);
  assertDomainName(domainName);

  const existing = await findDomainByName(user.uid, domainName);
  if (existing) {
    return sendError(res, 409, 'El dominio ya existe para esta cuenta');
  }

  const verificationToken = buildVerificationToken();
  const created = await createDomainRecord({
    ownerUid: user.uid,
    domainName,
    verified: false,
    verificationToken,
  });

  return sendJson(res, 201, {
    id: created.id,
    domainName,
    verified: false,
    dnsVerification: {
      host: buildVerificationHost(domainName),
      value: verificationToken,
    },
  });
}

async function handleGetDomain(req, res, user, domainId) {
  const domain = await getDomainById(domainId);
  if (!domain) {
    return sendError(res, 404, 'Dominio no encontrado');
  }

  if (domain.ownerUid !== user.uid) {
    return sendError(res, 403, 'No autorizado');
  }

  return sendJson(res, 200, { domain });
}

async function handleVerifyDomain(req, res, user, domainId) {
  const domain = await getDomainById(domainId);
  if (!domain) {
    return sendError(res, 404, 'Dominio no encontrado');
  }

  if (domain.ownerUid !== user.uid) {
    return sendError(res, 403, 'No autorizado');
  }

  const ok = await verifyTxtRecord(domain.domainName, domain.verificationToken);
  if (!ok) {
    return sendError(res, 400, 'Todavía no existe el TXT esperado');
  }

  await updateDomain(domainId, { verified: true, verifiedAt: new Date().toISOString() });
  return sendJson(res, 200, { ok: true, verified: true });
}

async function handleDeleteDomain(req, res, user, domainId) {
  const domain = await getDomainById(domainId);
  if (!domain) {
    return sendError(res, 404, 'Dominio no encontrado');
  }

  if (domain.ownerUid !== user.uid) {
    return sendError(res, 403, 'No autorizado');
  }

  await deleteDomain(domainId);
  return sendJson(res, 200, { ok: true });
}

async function handleListUrls(req, res, user, domainId) {
  const domain = await getDomainById(domainId);
  if (!domain) {
    return sendError(res, 404, 'Dominio no encontrado');
  }

  if (domain.ownerUid !== user.uid) {
    return sendError(res, 403, 'No autorizado');
  }

  const urls = await listUrlRules(domainId);
  return sendJson(res, 200, { urls });
}

async function handleCreateUrl(req, res, user, domainId) {
  const domain = await getDomainById(domainId);
  if (!domain) {
    return sendError(res, 404, 'Dominio no encontrado');
  }

  if (domain.ownerUid !== user.uid) {
    return sendError(res, 403, 'No autorizado');
  }

  const body = await readJson(req);
  assertRulePayload(body);

  const created = await createUrlRule(domainId, {
    pattern: String(body.pattern).trim(),
    cacheMaxSizeBytes: Number(body.cacheMaxSizeBytes ?? 0),
    ttlSeconds: Number(body.ttlSeconds ?? 0),
    replacementPolicy: String(body.replacementPolicy ?? 'LRU').toUpperCase(),
    contentTypes: Array.isArray(body.contentTypes) ? body.contentTypes : [],
    authMode: String(body.authMode ?? 'none'),
    wildcard: Boolean(body.wildcard ?? false),
  });

  return sendJson(res, 201, { id: created.id });
}

async function handleUpdateUrl(req, res, user, domainId, urlId) {
  const domain = await getDomainById(domainId);
  if (!domain) {
    return sendError(res, 404, 'Dominio no encontrado');
  }

  if (domain.ownerUid !== user.uid) {
    return sendError(res, 403, 'No autorizado');
  }

  const body = await readJson(req);
  assertRulePayload({ pattern: body.pattern ?? 'ok', ttlSeconds: body.ttlSeconds });

  await updateUrlRule(domainId, urlId, {
    pattern: body.pattern !== undefined ? String(body.pattern).trim() : undefined,
    cacheMaxSizeBytes: body.cacheMaxSizeBytes !== undefined ? Number(body.cacheMaxSizeBytes) : undefined,
    ttlSeconds: body.ttlSeconds !== undefined ? Number(body.ttlSeconds) : undefined,
    replacementPolicy: body.replacementPolicy ? String(body.replacementPolicy).toUpperCase() : undefined,
    contentTypes: Array.isArray(body.contentTypes) ? body.contentTypes : undefined,
    authMode: body.authMode !== undefined ? String(body.authMode) : undefined,
    wildcard: body.wildcard !== undefined ? Boolean(body.wildcard) : undefined,
  });

  return sendJson(res, 200, { ok: true });
}

async function handleDeleteUrl(req, res, user, domainId, urlId) {
  const domain = await getDomainById(domainId);
  if (!domain) {
    return sendError(res, 404, 'Dominio no encontrado');
  }

  if (domain.ownerUid !== user.uid) {
    return sendError(res, 403, 'No autorizado');
  }

  await deleteUrlRule(domainId, urlId);
  return sendJson(res, 200, { ok: true });
}

async function handleListApiKeys(req, res, user, domainId) {
  const domain = await getDomainById(domainId);
  if (!domain) {
    return sendError(res, 404, 'Dominio no encontrado');
  }

  if (domain.ownerUid !== user.uid) {
    return sendError(res, 403, 'No autorizado');
  }

  const apiKeys = await listApiKeys(domainId);
  return sendJson(res, 200, { apiKeys });
}

async function handleCreateApiKey(req, res, user, domainId) {
  const domain = await getDomainById(domainId);
  if (!domain) {
    return sendError(res, 404, 'Dominio no encontrado');
  }

  if (domain.ownerUid !== user.uid) {
    return sendError(res, 403, 'No autorizado');
  }

  const body = await readJson(req);
  const label = String(body.label ?? 'api-key').trim();
  const rawApiKey = `key_${randomUUID().replace(/-/g, '')}`;
  const keyHash = hashSecret(rawApiKey);

  const created = await createApiKey(domainId, {
    label,
    keyHash,
    active: true,
  });

  return sendJson(res, 201, {
    id: created.id,
    label,
    apiKey: rawApiKey,
  });
}

async function handleDeleteApiKey(req, res, user, domainId, keyId) {
  const domain = await getDomainById(domainId);
  if (!domain) {
    return sendError(res, 404, 'Dominio no encontrado');
  }

  if (domain.ownerUid !== user.uid) {
    return sendError(res, 403, 'No autorizado');
  }

  await deleteApiKey(domainId, keyId);
  return sendJson(res, 200, { ok: true });
}

async function handleCacheConfig(req, res, domainName) {
  await requireServiceKey(req);

  const normalizedDomain = normalizeDomainName(domainName);
  const domain = await findDomainByDomainName(normalizedDomain);
  if (!domain) {
    return sendError(res, 404, 'Dominio no encontrado');
  }

  if (!domain.verified) {
    return sendError(res, 403, 'El dominio todavía no está verificado');
  }

  const urls = await listUrlRules(domain.id);
  return sendJson(res, 200, {
    domain,
    urls,
  });
}

export async function handleRequest(req, res) {
  try {
    if (handleCorsPreflight(req, res)) {
      return;
    }

    const pathname = getPathname(req);
    const method = (req.method ?? 'GET').toUpperCase();
    const segments = pathname.split('/').filter(Boolean);

    if (method === 'GET' && pathname === '/health') {
      return await handleHealth(req, res);
    }

    if (method === 'POST' && pathname === '/auth/signup') {
      return await handleSignupUser(req, res);
    }

    if (method === 'POST' && pathname === '/auth/exchange') {
      return await handleAuthExchange(req, res);
    }

    if (pathname === '/auth/me') {
      const user = await requireUser(req);
      if (method === 'GET') {
        return await handleGetCurrentUser(req, res, user);
      }
      if (method === 'PUT') {
        return await handleUpdateCurrentUser(req, res, user);
      }
      if (method === 'DELETE') {
        return await handleDeleteCurrentUser(req, res, user);
      }
    }

    if (method === 'POST' && pathname === '/auth/logout') {
      const user = await requireUser(req);
      return await handleLogout(req, res, user);
    }

    if (segments[0] === 'domains' && segments.length === 1) {
      const user = await requireUser(req);
      if (method === 'GET') {
        return await handleListDomains(req, res, user);
      }
      if (method === 'POST') {
        return await handleCreateDomain(req, res, user);
      }
    }

    if (segments[0] === 'domains' && segments.length >= 2) {
      const user = await requireUser(req);
      const domainId = segments[1];

      if (segments.length === 2) {
        if (method === 'GET') {
          return await handleGetDomain(req, res, user, domainId);
        }
        if (method === 'DELETE') {
          return await handleDeleteDomain(req, res, user, domainId);
        }
      }

      if (segments.length === 3 && segments[2] === 'verify' && method === 'POST') {
        return await handleVerifyDomain(req, res, user, domainId);
      }

      if (segments.length === 3 && segments[2] === 'urls' && method === 'GET') {
        return await handleListUrls(req, res, user, domainId);
      }

      if (segments.length === 3 && segments[2] === 'urls' && method === 'POST') {
        return await handleCreateUrl(req, res, user, domainId);
      }

      if (segments.length === 4 && segments[2] === 'urls') {
        const urlId = segments[3];
        if (method === 'PUT') {
          return await handleUpdateUrl(req, res, user, domainId, urlId);
        }
        if (method === 'DELETE') {
          return await handleDeleteUrl(req, res, user, domainId, urlId);
        }
      }

      if (segments.length === 3 && segments[2] === 'api-keys' && method === 'GET') {
        return await handleListApiKeys(req, res, user, domainId);
      }

      if (segments.length === 3 && segments[2] === 'api-keys' && method === 'POST') {
        return await handleCreateApiKey(req, res, user, domainId);
      }

      if (segments.length === 4 && segments[2] === 'api-keys' && method === 'DELETE') {
        return await handleDeleteApiKey(req, res, user, domainId, segments[3]);
      }
    }

    if (segments[0] === 'cache-config' && segments.length === 2 && method === 'GET') {
      return await handleCacheConfig(req, res, segments[1]);
    }

    return sendError(res, 404, 'Ruta no encontrada');
  } catch (error) {
    const statusCode = error.statusCode ?? 500;
    const message = error.message ?? 'Error inesperado';
    return sendError(res, statusCode, message);
  }
}
