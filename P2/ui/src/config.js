// Configuración pública leída desde variables de entorno Vite (prefijo VITE_).
// En desarrollo: configurar en .env.local
// En producción: configurar en Vercel > Project Settings > Environment Variables

const env = import.meta.env;

export const config = {
  apiUrl: (env.VITE_API_URL ?? 'http://localhost:3001/api').replace(/\/$/, ''),
  firebase: {
    apiKey: env.VITE_FIREBASE_API_KEY ?? '',
    authDomain: env.VITE_FIREBASE_AUTH_DOMAIN ?? '',
    projectId: env.VITE_FIREBASE_PROJECT_ID ?? '',
    appId: env.VITE_FIREBASE_APP_ID ?? '',
  },
};

export function isFirebaseConfigured() {
  const { apiKey, authDomain, projectId, appId } = config.firebase;
  return Boolean(apiKey && authDomain && projectId && appId);
}
