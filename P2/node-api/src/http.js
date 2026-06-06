import { URL } from 'node:url';

function setCorsHeaders(res) {
  res.setHeader('Access-Control-Allow-Origin', '*');
  res.setHeader('Access-Control-Allow-Methods', 'GET,POST,PUT,DELETE,OPTIONS');
  res.setHeader('Access-Control-Allow-Headers', 'Content-Type, Authorization, x-api-key');
}

export function getPathname(req) {
  const requestUrl = new URL(req.url ?? '/', 'http://localhost');
  return requestUrl.pathname.replace(/^\/api/, '') || '/';
}

export function applyCors(res) {
  setCorsHeaders(res);
}

export function handleCorsPreflight(req, res) {
  if ((req.method ?? 'GET').toUpperCase() !== 'OPTIONS') {
    return false;
  }

  setCorsHeaders(res);
  res.statusCode = 204;
  res.end();
  return true;
}

export async function readJson(req) {
  if (req.body && typeof req.body === 'object') {
    return req.body;
  }

  const chunks = [];

  for await (const chunk of req) {
    chunks.push(chunk);
  }

  if (chunks.length === 0) {
    return {};
  }

  const raw = Buffer.concat(chunks).toString('utf8').trim();
  if (!raw) {
    return {};
  }

  return JSON.parse(raw);
}

export function sendJson(res, statusCode, payload) {
  res.statusCode = statusCode;
  setCorsHeaders(res);
  res.setHeader('Content-Type', 'application/json; charset=utf-8');
  res.end(JSON.stringify(payload));
}

export function sendError(res, statusCode, message, details = undefined) {
  const payload = { error: message };

  if (details) {
    payload.details = details;
  }

  sendJson(res, statusCode, payload);
}
