import { getFieldValue, getFirestore } from './firebase.js';

const domainsCollection = 'domains';

function collectionPath(domainId, childCollection) {
  return getFirestore().collection(domainsCollection).doc(domainId).collection(childCollection);
}

export async function createDomainRecord(data) {
  const db = getFirestore();
  const ref = await db.collection(domainsCollection).add({
    ...data,
    createdAt: getFieldValue().serverTimestamp(),
    updatedAt: getFieldValue().serverTimestamp(),
  });

  return { id: ref.id };
}

export async function listDomains(ownerUid) {
  const snapshot = await getFirestore()
    .collection(domainsCollection)
    .where('ownerUid', '==', ownerUid)
    .orderBy('createdAt', 'desc')
    .get();

  return snapshot.docs.map((doc) => ({ id: doc.id, ...doc.data() }));
}

export async function getDomainById(domainId) {
  const doc = await getFirestore().collection(domainsCollection).doc(domainId).get();
  if (!doc.exists) {
    return null;
  }

  return { id: doc.id, ...doc.data() };
}

export async function findDomainByName(ownerUid, domainName) {
  const snapshot = await getFirestore()
    .collection(domainsCollection)
    .where('ownerUid', '==', ownerUid)
    .where('domainName', '==', domainName)
    .limit(1)
    .get();

  if (snapshot.empty) {
    return null;
  }

  const doc = snapshot.docs[0];
  return { id: doc.id, ...doc.data() };
}

export async function findDomainByDomainName(domainName) {
  const snapshot = await getFirestore()
    .collection(domainsCollection)
    .where('domainName', '==', domainName)
    .limit(1)
    .get();

  if (snapshot.empty) {
    return null;
  }

  const doc = snapshot.docs[0];
  return { id: doc.id, ...doc.data() };
}

export async function updateDomain(domainId, patch) {
  await getFirestore().collection(domainsCollection).doc(domainId).update({
    ...patch,
    updatedAt: getFieldValue().serverTimestamp(),
  });
}

export async function deleteDomain(domainId) {
  await getFirestore().collection(domainsCollection).doc(domainId).delete();
}

export async function createUrlRule(domainId, data) {
  const ref = await collectionPath(domainId, 'urls').add({
    ...data,
    createdAt: getFieldValue().serverTimestamp(),
    updatedAt: getFieldValue().serverTimestamp(),
  });

  return { id: ref.id };
}

export async function listUrlRules(domainId) {
  const snapshot = await collectionPath(domainId, 'urls').orderBy('createdAt', 'desc').get();
  return snapshot.docs.map((doc) => ({ id: doc.id, ...doc.data() }));
}

export async function updateUrlRule(domainId, urlId, patch) {
  await collectionPath(domainId, 'urls').doc(urlId).update({
    ...patch,
    updatedAt: getFieldValue().serverTimestamp(),
  });
}

export async function deleteUrlRule(domainId, urlId) {
  await collectionPath(domainId, 'urls').doc(urlId).delete();
}

export async function createApiKey(domainId, data) {
  const ref = await collectionPath(domainId, 'apiKeys').add({
    ...data,
    createdAt: getFieldValue().serverTimestamp(),
    updatedAt: getFieldValue().serverTimestamp(),
  });

  return { id: ref.id };
}

export async function listApiKeys(domainId) {
  const snapshot = await collectionPath(domainId, 'apiKeys').orderBy('createdAt', 'desc').get();
  return snapshot.docs.map((doc) => ({ id: doc.id, ...doc.data() }));
}

export async function deleteApiKey(domainId, keyId) {
  await collectionPath(domainId, 'apiKeys').doc(keyId).delete();
}

// -----------------------------------------------------------------------------
// Usuarios por dominio (auth user/contraseña para URLs protegidas).
// El password se guarda hasheado (passwordHash); nunca en claro.
// -----------------------------------------------------------------------------

export async function createUser(domainId, data) {
  const ref = await collectionPath(domainId, 'users').add({
    ...data,
    createdAt: getFieldValue().serverTimestamp(),
    updatedAt: getFieldValue().serverTimestamp(),
  });

  return { id: ref.id };
}

export async function listUsers(domainId) {
  const snapshot = await collectionPath(domainId, 'users').orderBy('createdAt', 'desc').get();
  return snapshot.docs.map((doc) => ({ id: doc.id, ...doc.data() }));
}

export async function getUserById(domainId, userId) {
  const doc = await collectionPath(domainId, 'users').doc(userId).get();
  if (!doc.exists) {
    return null;
  }

  return { id: doc.id, ...doc.data() };
}

export async function findUserByUsername(domainId, username) {
  const snapshot = await collectionPath(domainId, 'users')
    .where('username', '==', username)
    .limit(1)
    .get();

  if (snapshot.empty) {
    return null;
  }

  const doc = snapshot.docs[0];
  return { id: doc.id, ...doc.data() };
}

export async function updateUser(domainId, userId, patch) {
  await collectionPath(domainId, 'users').doc(userId).update({
    ...patch,
    updatedAt: getFieldValue().serverTimestamp(),
  });
}

export async function deleteUser(domainId, userId) {
  await collectionPath(domainId, 'users').doc(userId).delete();
}

// -----------------------------------------------------------------------------
// Sesiones emitidas por el formulario de autenticación de las cachés zonales.
// Se guarda solo el hash del token; la caché lo verifica en peticiones siguientes.
// -----------------------------------------------------------------------------

const sessionsCollection = 'sessions';

export async function createSession(data) {
  const ref = await getFirestore().collection(sessionsCollection).add({
    ...data,
    createdAt: getFieldValue().serverTimestamp(),
  });

  return { id: ref.id };
}

export async function findSessionByTokenHash(tokenHash) {
  const snapshot = await getFirestore()
    .collection(sessionsCollection)
    .where('tokenHash', '==', tokenHash)
    .limit(1)
    .get();

  if (snapshot.empty) {
    return null;
  }

  const doc = snapshot.docs[0];
  return { id: doc.id, ...doc.data() };
}

export async function deleteSession(sessionId) {
  await getFirestore().collection(sessionsCollection).doc(sessionId).delete();
}