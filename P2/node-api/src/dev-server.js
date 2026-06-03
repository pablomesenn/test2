import 'dotenv/config';
import http from 'node:http';
import { handleRequest } from './routes.js';

const port = Number(process.env.PORT ?? 3001);

http.createServer((req, res) => {
  handleRequest(req, res);
}).listen(port, () => {
  console.log(`Node API listening on http://localhost:${port}`);
});
