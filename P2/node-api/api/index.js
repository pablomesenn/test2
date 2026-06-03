import { handleRequest } from '../src/routes.js';

export default async function handler(req, res) {
  await handleRequest(req, res);
}
