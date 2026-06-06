export function formatBytes(bytes) {
  if (bytes === null || bytes === undefined || bytes === 0) return '0 B';
  const units = ['B', 'KB', 'MB', 'GB', 'TB'];
  const i = Math.min(Math.floor(Math.log(bytes) / Math.log(1024)), units.length - 1);
  const value = bytes / Math.pow(1024, i);
  return `${value.toFixed(value >= 100 || i === 0 ? 0 : 1)} ${units[i]}`;
}

export function formatTtl(seconds) {
  if (!seconds || seconds <= 0) return '—';
  if (seconds < 60) return `${seconds}s`;
  if (seconds < 3600) return `${Math.round(seconds / 60)}m`;
  if (seconds < 86400) return `${Math.round(seconds / 3600)}h`;
  return `${Math.round(seconds / 86400)}d`;
}

export function formatTimestamp(value) {
  if (!value) return '—';
  // Firestore timestamps llegan como { _seconds, _nanoseconds } al pasar por JSON.
  let date;
  if (typeof value === 'object' && value._seconds) {
    date = new Date(value._seconds * 1000);
  } else if (typeof value === 'object' && typeof value.seconds === 'number') {
    date = new Date(value.seconds * 1000);
  } else {
    date = new Date(value);
  }
  if (Number.isNaN(date.getTime())) return '—';
  return new Intl.DateTimeFormat('es-CR', {
    dateStyle: 'medium',
    timeStyle: 'short',
  }).format(date);
}

export function truncateMiddle(str, max = 18) {
  if (!str || str.length <= max) return str ?? '';
  const half = Math.floor((max - 1) / 2);
  return `${str.slice(0, half)}…${str.slice(-half)}`;
}

export function parseBytesInput(text) {
  // Acepta "10 MB", "1.5GB", "1024", "500 KB"
  if (text == null) return 0;
  const trimmed = String(text).trim();
  const match = trimmed.match(/^(\d+(?:\.\d+)?)\s*(B|KB|MB|GB)?$/i);
  if (!match) return Number.NaN;
  const [, numStr, unitRaw] = match;
  const num = Number(numStr);
  const unit = (unitRaw ?? 'B').toUpperCase();
  const mult = { B: 1, KB: 1024, MB: 1024 ** 2, GB: 1024 ** 3 }[unit] ?? 1;
  return Math.round(num * mult);
}

export const REPLACEMENT_POLICIES = ['LRU', 'LFU', 'FIFO', 'MRU', 'RANDOM'];
export const AUTH_MODES = [
  { value: 'none', label: 'Sin autenticación' },
  { value: 'api_key', label: 'API Key (x-api-key)' },
  { value: 'user_password', label: 'Usuario y contraseña' },
];
export const COMMON_CONTENT_TYPES = ['html', 'css', 'js', 'json', 'png', 'jpg', 'svg', 'webp', 'gif', 'pdf', 'txt', 'xml'];
