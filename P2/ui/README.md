# UI · Proxy/Cache Zonal IC-7602

Consola de administración para el Proyecto 2 de **Redes (IC-7602)** del TEC. Permite registrar y verificar dominios, definir reglas de caché por URL, administrar API keys y usuarios, y expone el formulario de autenticación que invocan las cachés zonales.

Construida con **React 18 + Vite 5 + Tailwind 3 + Firebase Auth** y se conecta al Node API ya desplegado en Vercel.

---

## 1. Requisitos previos

- **Node.js ≥ 18.17** (Vite 5 lo exige) y npm ≥ 9.
- Acceso al proyecto de **Firebase** `ic7602-p2-79108` y a su consola para obtener la configuración pública del Web SDK.
- El **Node API** corriendo (en local o en Vercel). En este proyecto la URL pública es `https://2026-01-ic-7602.vercel.app/api`.

---

## 2. Configuración rápida

```bash
cd ui
cp .env.example .env.local
npm install
npm run dev          # http://localhost:5173
```

Llena `.env.local` con la configuración del Firebase Web SDK. Esos valores se obtienen en:
**Firebase Console → Project Settings → Tus apps → SDK setup and configuration**.

```env
VITE_API_URL=https://2026-01-ic-7602.vercel.app/api
VITE_FIREBASE_API_KEY=AIzaSy...
VITE_FIREBASE_AUTH_DOMAIN=ic7602-p2-79108.firebaseapp.com
VITE_FIREBASE_PROJECT_ID=ic7602-p2-79108
VITE_FIREBASE_APP_ID=1:1234:web:abcdef...
```

En Firebase Console, **habilita el proveedor "Email/Password"** en *Authentication → Sign-in method* antes de probar el registro.

---

## 3. Funcionalidad implementada

### Cuentas y sesión
- Registro y login con Firebase Auth (email + contraseña).
- El ID token de Firebase se envía como `Authorization: Bearer ...` al Node API.
- Cierre de sesión desde la barra superior.
- Errores de Firebase traducidos a español.

### Administración de dominios (`/dashboard`)
- Lista los dominios del usuario con estado verificado/pendiente.
- Crear un dominio: la UI muestra el registro **TXT** que debe agregarse al DNS (`_ic7602-verify.<dominio>` con el token devuelto por el API) y permite copiarlo.
- Eliminar dominio (borra también URLs, claves y usuarios asociados en cascada del lado del backend).

### Detalle de dominio (`/domains/:id`)
Tres pestañas:

1. **Reglas de URL** — crear/editar/eliminar reglas con:
   - Patrón (con soporte de wildcards mediante flag explícito).
   - Tamaño máximo de caché (entrada tipo `10 MB`, `1.5 GB`, etc.; se parsea a bytes).
   - TTL en segundos.
   - Política de reemplazo (`LRU`, `LFU`, `FIFO`, `MRU`, `RANDOM`).
   - Content-types con toggles.
   - Modo de autenticación (`Sin autenticación`, `API Key`, `Usuario y contraseña`).
2. **API Keys** — listar, crear, eliminar. El valor en claro se muestra **una sola vez** al crearla.
3. **Usuarios** — CRUD de usuarios para auth user/password (ver §5).

### Formulario para cachés zonales (`/auth/zonal`)
- Página pública que aceptan los redirects de las cachés.
- Acepta `?domain=...&return_to=...&origin=...`.
- Valida credenciales contra el Node API (ver §5).
- Tras autenticar redirige a `return_to` agregando `?token=<sessionToken>`.

### Notificaciones
- Sistema de toasts con éxito/error/info/warning, auto-dismiss.

---

## 4. Endpoints consumidos del Node API

La UI usa el cliente en `src/lib/api.js`. Todos los endpoints actualmente existentes en `2026-01-IC-7602/api/routes.js` están conectados:

| Método | Ruta | Uso en UI |
|---|---|---|
| `GET` | `/api/health` | (manual) |
| `POST` | `/api/auth/exchange` | (disponible, no se llama de momento) |
| `GET` | `/api/domains` | Dashboard |
| `POST` | `/api/domains` | Modal "Registrar dominio" |
| `GET` | `/api/domains/:id` | Detalle |
| `DELETE` | `/api/domains/:id` | Eliminar dominio |
| `POST` | `/api/domains/:id/verify` | Verificar TXT |
| `GET / POST / PUT / DELETE` | `/api/domains/:id/urls[/:urlId]` | Pestaña Reglas |
| `GET / POST / DELETE` | `/api/domains/:id/api-keys[/:keyId]` | Pestaña API Keys |

---

## 5. Endpoints esperados (todavía no existen en el Node API)

La UI ya consume estos endpoints; cuando responden 404/501 la UI lo informa amablemente. **Quien extienda el Node API debe implementarlos con este contrato**:

### Usuarios por dominio (auth `Bearer <Firebase ID Token>`)

```
GET    /api/domains/:id/users
       → { users: [{ id, username, createdAt }] }

POST   /api/domains/:id/users
       body: { username, password }
       → { user: { id, username, createdAt } }

PUT    /api/domains/:id/users/:userId
       body: { username?, password? }
       → { user: { id, username, updatedAt } }

DELETE /api/domains/:id/users/:userId
       → { ok: true }
```

Los passwords nunca se devuelven; en Firestore deben guardarse con hash + salt (Argon2id o bcrypt).

### Validación de credenciales para cachés zonales (sin Bearer)

```
POST   /api/zonal-auth/validate
       body: { domainName, username, password }
       → 200 { sessionToken, expiresAt }
       → 401 { error: 'invalid_credentials' }
```

El `sessionToken` se concatena a `return_to` como `?token=...` y la caché lo valida después (probablemente llamando a otro endpoint, por ejemplo `POST /api/zonal-auth/verify`).

> **Nota de seguridad**: este endpoint debería tener rate-limiting agresivo (intentos por IP por dominio) ya que es público.

---

## 6. Flujo de redirect desde la caché zonal

```
Cliente ── GET /protegido ─────────────►  Caché zonal
                                          (URL con authMode = user_password)
Caché ── 302 ──────────────────────────►  https://ui.../auth/zonal
                                          ?domain=midominio.test
                                          &return_to=https://cache.../resume?req=abc
                                          &origin=Caché%20CR

Usuario ── credenciales ───────────────►  UI
UI    ── POST /api/zonal-auth/validate ─►  Node API
UI    ── 302 a return_to ──────────────►  Caché zonal con ?token=...

Caché ── usa el token en subsecuentes peticiones del cliente ─►
```

---

## 7. Estructura del proyecto

```
ui/
├── public/favicon.svg
├── src/
│   ├── main.jsx
│   ├── App.jsx                 # Rutas
│   ├── config.js               # Lectura de variables VITE_
│   ├── index.css               # Tailwind + tokens visuales
│   ├── context/
│   │   ├── AuthContext.jsx     # Sesión Firebase
│   │   └── ToastContext.jsx
│   ├── lib/
│   │   ├── firebase.js         # Wrappers de Firebase Auth
│   │   ├── api.js              # Cliente del Node API
│   │   └── formatters.js       # Helpers (bytes, ttl, fechas)
│   ├── components/
│   │   ├── Layout.jsx          # Topbar + footer
│   │   ├── Modal.jsx
│   │   ├── ConfirmDialog.jsx
│   │   ├── Tabs.jsx
│   │   ├── ProtectedRoute.jsx
│   │   └── ui.jsx              # Primitivas: Button, Input, Badge, etc.
│   └── pages/
│       ├── LoginPage.jsx
│       ├── DashboardPage.jsx
│       ├── DomainDetailPage.jsx
│       ├── ZonalAuthPage.jsx
│       └── NotFoundPage.jsx
├── index.html
├── package.json
├── postcss.config.js
├── tailwind.config.js
├── vite.config.js
└── vercel.json                 # SPA rewrites
```

---

## 8. Desarrollo

```bash
npm run dev       # servidor Vite con HMR en :5173
npm run build     # compila a dist/
npm run preview   # sirve dist/ localmente para verificar el bundle
```

---

## 9. Despliegue en Vercel

La UI debe estar en un **proyecto Vercel separado** del Node API (o como sub-proyecto del monorepo apuntando a `ui/`).

1. **Importar repositorio** en Vercel y, en *Project Settings*, fijar **Root Directory = `ui`**.
2. **Framework Preset**: Vite (autodetectado).
3. **Build Command**: `npm run build` · **Output Directory**: `dist`.
4. **Environment Variables** (en *Project Settings → Environment Variables*):
   - `VITE_API_URL` → `https://2026-01-ic-7602.vercel.app/api`
   - `VITE_FIREBASE_API_KEY`, `VITE_FIREBASE_AUTH_DOMAIN`, `VITE_FIREBASE_PROJECT_ID`, `VITE_FIREBASE_APP_ID`
5. El `vercel.json` que viene en el repo se encarga del rewrite SPA (todas las rutas → `/index.html`).
6. Cada push a la rama principal dispara un deploy automático.

Después del primer despliegue, **agregar el dominio Vercel a los Authorized Domains de Firebase Auth** (Console → Authentication → Settings → Authorized domains), si no, el login fallará con `auth/unauthorized-domain`.

---

## 10. Decisiones de diseño

- **Stack mínimo, sin libs de estado**: solo React + Router + Firebase + lucide-react + Tailwind. Todo el fetching se hace con `fetch` nativo y `useEffect`.
- **Firebase modular** (`@firebase/app` + `@firebase/auth` en vez del meta-paquete) → bundle ~107 KB gzipped.
- **Estética de consola técnica**: tipografía serif italic (*Instrument Serif*) para títulos, sans con buen rendimiento (*IBM Plex Sans*) para cuerpo, monoespaciada (*JetBrains Mono*) para IDs/tokens. Paleta oscura con un único acento ámbar.
- **Manejo de endpoints faltantes**: cualquier ruta esperada pero todavía no implementada en el Node API muestra un panel informativo con el contrato esperado, en vez de un error críptico.
- **Tamaños y TTL en formatos humanos**: el formulario acepta `10 MB`, `1.5 GB`, `3600` y los convierte a bytes/segundos antes de enviar.
- **Las API keys solo se muestran en claro al crearlas** (alineado con cómo se guardan hasheadas en Firestore desde el Node API).

---

## 11. Cosas pendientes para completar el proyecto (no UI)

Estos puntos NO son parte de esta entrega de UI, pero se documentan para contexto del equipo:

1. **Node API**: implementar `/zonal-auth/validate` y CRUD de `/domains/:id/users` (contrato en §5).
2. **Zonal Cache (Rust)**: cuando una URL requiera `user_password`, devolver 302 hacia `<UI>/auth/zonal?domain=...&return_to=...` y honrar el `token` en peticiones siguientes.
3. **Firebase Auth**: habilitar Email/Password en consola y agregar dominios Vercel a Authorized domains.

---

_Estudiante: Sede TEC · Curso IC-7602 · Mayo 2026._
