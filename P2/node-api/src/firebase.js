import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { config, defaultServiceAccountPaths, requireEnv } from './config.js';

function parseServiceAccountJson(rawValue) {
  const parsed = JSON.parse(rawValue);
  if (parsed.private_key) parsed.private_key = parsed.private_key.replace(/\\n/g, '\n');
  return parsed;
}

function resolveServiceAccountPath() {
  if (process.env.GOOGLE_APPLICATION_CREDENTIALS) {
    return process.env.GOOGLE_APPLICATION_CREDENTIALS;
  }

  if (config.firebaseServiceAccountPath) {
    return config.firebaseServiceAccountPath;
  }

  for (const candidate of defaultServiceAccountPaths) {
    if (fs.existsSync(candidate)) {
      return candidate;
    }
  }

  if (config.firebaseServiceAccountJson) {
    try {
      const parsed = parseServiceAccountJson(config.firebaseServiceAccountJson);
      const tmpDir = os.tmpdir();
      const tmpPath = path.join(tmpDir, `firebase-service-account-${parsed.project_id}.json`);
      if (!fs.existsSync(tmpPath)) {
        fs.writeFileSync(tmpPath, JSON.stringify(parsed), { encoding: 'utf8', mode: 0o600 });
      }
      return tmpPath;
    } catch (e) {
      console.warn('firebase.js: failed to write temp service account for ADC:', e?.message ?? e);
    }
  }

  return '';
}

const resolvedServiceAccountPath = resolveServiceAccountPath();

// If the service account path was resolved, expose it for ADC-based libraries.
if (resolvedServiceAccountPath && !process.env.GOOGLE_APPLICATION_CREDENTIALS) {
  try {
    process.env.GOOGLE_APPLICATION_CREDENTIALS = resolvedServiceAccountPath;
  } catch (e) {
    console.warn('firebase.js: failed to set GOOGLE_APPLICATION_CREDENTIALS:', e?.message ?? e);
  }
}

import admin from 'firebase-admin';

function loadServiceAccount() {
  if (config.firebaseServiceAccountBase64) {
    const decoded = Buffer.from(config.firebaseServiceAccountBase64, 'base64').toString('utf8');
    return parseServiceAccountJson(decoded);
  }

  if (config.firebaseServiceAccountJson) {
    return parseServiceAccountJson(config.firebaseServiceAccountJson);
  }

  if (config.firebaseServiceAccountPath) {
    const raw = fs.readFileSync(config.firebaseServiceAccountPath, 'utf8');
    const parsed = JSON.parse(raw);
    if (parsed.private_key) {
      parsed.private_key = parsed.private_key.replace(/\\n/g, '\n');
    }
    return parsed;
  }

  throw new Error('Define FIREBASE_SERVICE_ACCOUNT_JSON o FIREBASE_SERVICE_ACCOUNT_PATH');
}

function hasApplicationDefaultCredentials() {
  return Boolean(process.env.GOOGLE_APPLICATION_CREDENTIALS);
}

function resolveProjectId() {
  if (config.firebaseProjectId) {
    return config.firebaseProjectId;
  }

  if (resolvedServiceAccountPath && fs.existsSync(resolvedServiceAccountPath)) {
    const raw = fs.readFileSync(resolvedServiceAccountPath, 'utf8');
    const parsed = JSON.parse(raw);
    if (parsed.project_id) {
      return parsed.project_id;
    }
  }

  return '';
}

export function getAdminApp() {
  if (admin.apps.length > 0) {
    return admin.app();
  }

  const credential = hasApplicationDefaultCredentials()
    ? admin.credential.applicationDefault()
    : admin.credential.cert(loadServiceAccount());

  console.debug('firebase.getAdminApp: credentialSource=' + (hasApplicationDefaultCredentials() ? 'applicationDefault' : 'cert'));
  console.debug('firebase.getAdminApp: GOOGLE_APPLICATION_CREDENTIALS=' + (process.env.GOOGLE_APPLICATION_CREDENTIALS ?? '<unset>'));
  return admin.initializeApp({
    credential,
    projectId: requireEnv('FIREBASE_PROJECT_ID', resolveProjectId()),
  });
}

export function getFirestore() {
  getAdminApp();

  return admin.firestore();
}

export async function verifyIdToken(idToken, checkRevoked = false) {
  getAdminApp();
  return admin.auth().verifyIdToken(idToken, checkRevoked);
}

export function getFieldValue() {
  return admin.firestore.FieldValue;
}
