import { initializeApp, getApp, getApps } from '@firebase/app';
import {
  getAuth,
  signInWithEmailAndPassword,
  createUserWithEmailAndPassword,
  signOut as fbSignOut,
  onAuthStateChanged,
  updateProfile,
} from '@firebase/auth';
import { config, isFirebaseConfigured } from '../config.js';

function getFirebaseApp() {
  if (getApps().length > 0) return getApp();
  if (!isFirebaseConfigured()) {
    throw new Error(
      'Firebase no está configurado. Define VITE_FIREBASE_API_KEY, VITE_FIREBASE_AUTH_DOMAIN, VITE_FIREBASE_PROJECT_ID y VITE_FIREBASE_APP_ID en .env.local.',
    );
  }
  return initializeApp(config.firebase);
}

export function getFirebaseAuth() {
  return getAuth(getFirebaseApp());
}

export async function signIn(email, password) {
  const auth = getFirebaseAuth();
  const cred = await signInWithEmailAndPassword(auth, email, password);
  return cred.user;
}

export async function signUp({ email, password, displayName }) {
  const auth = getFirebaseAuth();
  const cred = await createUserWithEmailAndPassword(auth, email, password);
  if (displayName && cred.user) {
    await updateProfile(cred.user, { displayName });
  }
  return cred.user;
}

export async function signOut() {
  const auth = getFirebaseAuth();
  await fbSignOut(auth);
}

export function watchAuth(callback) {
  const auth = getFirebaseAuth();
  return onAuthStateChanged(auth, callback);
}

export async function getIdToken() {
  const auth = getFirebaseAuth();
  if (!auth.currentUser) return null;
  return auth.currentUser.getIdToken();
}

/** Traduce errores típicos de Firebase Auth a mensajes en español. */
export function describeAuthError(err) {
  const code = err?.code ?? '';
  switch (code) {
    case 'auth/invalid-email': return 'El email no tiene un formato válido.';
    case 'auth/missing-password': return 'La contraseña es obligatoria.';
    case 'auth/weak-password': return 'La contraseña es muy débil (mínimo 6 caracteres).';
    case 'auth/email-already-in-use': return 'Ya existe una cuenta con ese email.';
    case 'auth/user-not-found':
    case 'auth/wrong-password':
    case 'auth/invalid-credential': return 'Credenciales inválidas.';
    case 'auth/network-request-failed': return 'Sin conexión con Firebase.';
    case 'auth/too-many-requests': return 'Demasiados intentos. Espera unos minutos.';
    default: return err?.message ?? 'Error de autenticación.';
  }
}
