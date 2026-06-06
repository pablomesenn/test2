export const config = {
  firebaseProjectId: process.env.FIREBASE_PROJECT_ID ?? '',
  firebaseServiceAccountJson: process.env.FIREBASE_SERVICE_ACCOUNT_JSON ?? '',
  firebaseServiceAccountBase64: process.env.FIREBASE_SERVICE_ACCOUNT_BASE64 ?? '',
  firebaseServiceAccountPath: process.env.FIREBASE_SERVICE_ACCOUNT_PATH ?? '',
  cacheServiceApiKey: process.env.CACHE_SERVICE_API_KEY ?? '',
  nodeApiUrl: process.env.NODE_API_URL ?? '',
};

export const defaultServiceAccountPaths = [
  './firebase-key.json',
  '../firebase-service-account.json',
  './firebase-service-account.json',
];

export function requireEnv(name, value) {
  if (!value) {
    throw new Error(`Missing required environment variable: ${name}`);
  }

  return value;
}
