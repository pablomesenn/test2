# IC7602 — Proyecto 2

## Componentes
| Componente | Tecnología | Deploy |
|---|---|---|
| DNS Interceptor | Rust | Kubernetes (Helm) |
| Zonal Cache | Rust | Kubernetes (Helm) |
| REST API | Java | Kubernetes (Helm) |
| Apache Server | Apache | Kubernetes (Helm) |
| Node.js API | Node.js | Vercel |
| UI | React + Vite | Vercel |

## Requisitos
- minikube >= 1.32
- kubectl >= 1.29
- Helm >= 3.14
- Rust >= 1.77
- Java >= 21
- Node.js >= 20

## Levantar el stack completo
```bash
minikube start
helm install p2 ./helm/umbrella
```

## Ejecutar pruebas
```bash
# Rust
cd dns-interceptor && cargo test
cd zonal-cache && cargo test

# Java
cd rest-api-java && ./mvnw test
```

## Node.js API
La carpeta [node-api/](node-api/) contiene la API REST que conecta la UI y las cachés zonales con Firebase / Firestore. Este componente se despliega en Vercel con despliegues automáticos desde GitHub.

### Responsabilidades
- Autenticación de usuarios con Firebase Auth usando `idToken` en `Authorization: Bearer ...`.
- Registro de cuentas con Firebase Auth y administración del usuario autenticado: consulta, actualización de correo/contraseña/nombre y eliminación de la cuenta.
- Gestión de dominios en Firestore.
- Verificación de dominios mediante registro TXT.
- Creación, edición y eliminación de reglas de URL por dominio.
- Creación y eliminación de API keys para consumo desde las cachés zonales.
- Administración de usuarios usuario/contraseña por dominio (crear, listar, modificar, eliminar) para las URLs protegidas con ese modo de autenticación. Las contraseñas se guardan hasheadas con `scrypt` (`scrypt$salt$hash`) y nunca se devuelven.
- Validación de credenciales usuario/contraseña para el formulario de las cachés zonales (`/api/zonal-auth/validate`) y verificación posterior de la sesión emitida (`/api/zonal-auth/verify`).
- Exposición de `GET /api/cache-config/:domainName` para que la caché zonal obtenga su configuración.

### Variables de entorno
Configura estas variables en local y en Vercel:

- `FIREBASE_PROJECT_ID`
- `FIREBASE_SERVICE_ACCOUNT_BASE64` o `FIREBASE_SERVICE_ACCOUNT_JSON`
- `CACHE_SERVICE_API_KEY`
- `NODE_API_URL`
- `ZONAL_SESSION_TTL_SECONDS` (opcional, default `3600`): vigencia del token de sesión emitido por `/api/zonal-auth/validate`.

Recomendación para Vercel: usar `FIREBASE_SERVICE_ACCOUNT_BASE64`.

### Ejecución local
```bash
cd node-api
npm install
npm run dev
```

### Verificación local
```powershell
Invoke-RestMethod -Method Get -Uri "http://localhost:3001/api/health"
```

### Endpoints principales
- `GET /api/health`
- `POST /api/auth/signup`
- `POST /api/auth/exchange`
- `GET /api/auth/me`
- `PUT /api/auth/me`
- `DELETE /api/auth/me`
- `POST /api/auth/logout`
- `GET /api/domains`
- `POST /api/domains`
- `POST /api/domains/:id/verify`
- `GET /api/domains/:id/urls`
- `POST /api/domains/:id/urls`
- `PUT /api/domains/:id/urls/:urlId`
- `DELETE /api/domains/:id/urls/:urlId`
- `GET /api/domains/:id/api-keys`
- `POST /api/domains/:id/api-keys`
- `DELETE /api/domains/:id/api-keys/:keyId`
- `GET /api/domains/:id/users`
- `POST /api/domains/:id/users` — body `{ username, password }`
- `PUT /api/domains/:id/users/:userId` — body `{ username?, password? }`
- `DELETE /api/domains/:id/users/:userId`
- `POST /api/zonal-auth/validate` — público; body `{ domainName, username, password }` → `{ sessionToken, expiresAt }` | `401 invalid_credentials`
- `POST /api/zonal-auth/verify` — requiere `x-api-key`; body `{ sessionToken }` → `{ valid, domainName?, username?, expiresAt? }`
- `GET /api/cache-config/:domainName`

### Verificación en Vercel
Ingresar en el navegador la url:
https://2026-01-ic-7602.vercel.app/api/health

# para deploy