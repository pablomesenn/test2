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
- Docker

## Levantar el stack completo

### 1. Colocar credenciales de Firebase

Copiar el archivo `firebase-service-account.json` en la carpeta `P2/` antes de continuar. Este archivo **no está en el repositorio** por razones de seguridad — solicitarlo al equipo.

### 2. Iniciar minikube

```bash
minikube start
```

### 3. Build de todas las imágenes

```bash
cd P2
docker build -t dns-interceptor:dev  ./dns-interceptor
docker build -t zonal-cache:dev      ./zonal-cache
docker build -t rest-api-java:0.1.0  ./rest-api-java
docker build -t apache-server:dev    ./apache-server
```

### 4. Descargar dependencias del umbrella chart

```bash
helm dependency update ./helm/umbrella
```

### 5. Instalar el stack completo

```bash
helm install p2 ./helm/umbrella \
  --set-file zonal-cache.firebaseServiceAccount=./firebase-service-account.json
```

### 6. Verificar que todos los pods estén Running

```bash
kubectl get pods
```

Resultado esperado:

```
NAME                                READY   STATUS    RESTARTS
p2-apache-server-...                1/1     Running   0
p2-cache-...                        1/1     Running   0
p2-dns-...                          1/1     Running   0
p2-rest-api-java-...                1/1     Running   0
```

### 7. Abrir port-forwards para pruebas

Abrir una terminal por cada servicio:

```bash
# Terminal 1 — Zonal Cache
kubectl port-forward svc/p2-cache 8080:8080

# Terminal 2 — Apache Server
kubectl port-forward svc/p2-apache-server 8082:80

# Terminal 3 — REST API Java
kubectl port-forward svc/p2-rest-api-java 8081:8081
```

---

## Ejecutar pruebas unitarias e integración

```bash
# Rust — DNS Interceptor
cd dns-interceptor && cargo test

# Rust — Zonal Cache (unit + integración)
cd zonal-cache && cargo test

# Java
cd rest-api-java && ./mvnw test
```

---

## Pruebas del DNS Interceptor

### Prerequisito

Obtener la IP de minikube:

```bash
minikube ip
# Ejemplo: 192.168.49.2
```

### Dominio gestionado → devuelve IP de la Zonal Cache

```bash
dig @192.168.49.2 -p 32647 midominio.test
```

Respuesta esperada:

```
;; ANSWER SECTION:
midominio.test.    30    IN    A    10.0.0.99
```

### Segundo dominio gestionado

```bash
dig @192.168.49.2 -p 32647 ejemplo.test
```

Respuesta esperada:

```
;; ANSWER SECTION:
ejemplo.test.    30    IN    A    10.0.0.99
```

### Dominio NO gestionado → forwarding a 8.8.8.8

```bash
dig @192.168.49.2 -p 32647 google.com
dig @192.168.49.2 -p 32647 cloudflare.com
```

Los dominios no gestionados deben resolverse con IPs reales desde Internet.

### Verificar logs del interceptor

```bash
kubectl logs deployment/p2-dns --tail=20
```

Salida esperada para cada query:

```
INFO dns_interceptor: Query de 10.244.0.1:XXXXX → 'midominio.test'
INFO dns_interceptor:   → dominio gestionado, zona: 10.0.0.99
INFO dns_interceptor: Query de 10.244.0.1:XXXXX → 'google.com'
INFO dns_interceptor:   → forwarding a 8.8.8.8:53
```

---

## Pruebas del Zonal Cache

> Todas las pruebas requieren el port-forward `kubectl port-forward svc/p2-cache 8080:8080` activo.

### Health y conectividad con Firebase

```bash
curl http://localhost:8080/health
```

Respuesta esperada:

```json
{"firestore":"connected","status":"ok"}
```

### Recargar configuración desde Firebase

```bash
curl http://localhost:8080/cache/reload
```

Respuesta esperada:

```json
{"max_size_bytes":104857600,"policy":"LRU","status":"reloaded","ttl_seconds":3600}
```

### Ver estadísticas del cache

```bash
curl http://localhost:8080/cache/stats
```

### Proxy con Apache Server — MISS y HIT

```bash
# Primera llamada: MISS (fetch al origen)
curl -v "http://localhost:8080/proxy?url=http://p2-apache-server:80" 2>&1 \
  | grep "x-cache\|x-cache-age\|x-origin\|< HTTP"

# Segunda llamada: HIT (desde disco)
curl -v "http://localhost:8080/proxy?url=http://p2-apache-server:80" 2>&1 \
  | grep "x-cache\|x-cache-age\|x-origin\|< HTTP"
```

Si se necesita reinicar el caché y tambien los fowarding:

```bash
kubectl exec deployment/p2-cache -- rm -rf /app/cache_data/*

kubectl rollout restart deployment/p2-cache
```

Respuesta esperada primera llamada:

```
< HTTP/1.1 200 OK
< x-cache: MISS
< x-cache-policy: LRU
< x-cache-domain: p2-apache-server
< x-origin: http://p2-apache-server:80
```

Respuesta esperada segunda llamada:

```
< HTTP/1.1 200 OK
< x-cache: HIT
< x-cache-policy: LRU
< x-cache-domain: p2-apache-server
< x-cache-age: 5
< x-origin: http://p2-apache-server:80
```

### Proxy con REST API Java — MISS y HIT

```bash
curl -v "http://localhost:8080/proxy?url=http://p2-rest-api-java:8081/api/items" 2>&1 \
  | grep "x-cache\|x-cache-age\|x-origin\|< HTTP"

curl -v "http://localhost:8080/proxy?url=http://p2-rest-api-java:8081/api/items" 2>&1 \
  | grep "x-cache\|x-cache-age\|x-origin\|< HTTP"
```

### Proxy con sitio externo HTTP — MISS y HIT

```bash
curl -v "http://localhost:8080/proxy?url=http://example.com" 2>&1 \
  | grep "x-cache\|x-cache-age\|x-origin\|< HTTP"

curl -v "http://localhost:8080/proxy?url=http://example.com" 2>&1 \
  | grep "x-cache\|x-cache-age\|x-origin\|< HTTP"
```

### Proxy con sitio externo HTTPS — MISS y HIT

```bash
curl -v "http://localhost:8080/proxy?url=https://example.com" 2>&1 \
  | grep "x-cache\|x-cache-age\|x-origin\|< HTTP"

curl -v "http://localhost:8080/proxy?url=https://example.com" 2>&1 \
  | grep "x-cache\|x-cache-age\|x-origin\|< HTTP"
```

### Autenticación con API Key válida

```bash
curl -v "http://localhost:8080/proxy?url=http://p2-apache-server:80" \
  -H "x-api-key: test-key-123" 2>&1 | grep "x-cache\|< HTTP"
```

Respuesta esperada: `HTTP/1.1 200 OK`

### Autenticación con API Key inválida

```bash
curl -v "http://localhost:8080/proxy?url=http://p2-apache-server:80" \
  -H "x-api-key: clave-incorrecta" 2>&1 | grep "< HTTP"
```

### Login con credenciales inválidas

```bash
curl -X POST http://localhost:8080/auth/login \
  -H "Content-Type: application/json" \
  -d '{"user":"admin","password":"wrong"}'
```

Respuesta esperada: `Invalid credentials`

### URL con esquema inválido

```bash
curl -v "http://localhost:8080/proxy?url=ftp://example.com" 2>&1 | grep "< HTTP"
```

Respuesta esperada: `HTTP/1.1 400 Bad Request`

### Stats finales

```bash
curl http://localhost:8080/cache/stats
```

Debe mostrar los dominios cacheados con sus tamaños y número de entradas.

---

## Pruebas del Apache Server

```bash
# Directo al servidor
curl http://localhost:8082

# Vía Zonal Cache
curl -v "http://localhost:8080/proxy?url=http://p2-apache-server:80" 2>&1 \
  | grep "x-cache\|< HTTP"
```

---

## Pruebas del REST API Java

```bash
# Health
curl http://localhost:8081/actuator/health

# GET todos los items
curl http://localhost:8081/api/items

# POST crear item
curl -X POST http://localhost:8081/api/items \
  -H "Content-Type: application/json" \
  -d '{"name":"Test Item","description":"Descripción de prueba"}'

# PUT actualizar item (reemplazar {id} con el ID del item creado)
curl -X PUT http://localhost:8081/api/items/{id} \
  -H "Content-Type: application/json" \
  -d '{"name":"Item Actualizado","description":"Nueva descripción"}'

# DELETE eliminar item
curl -X DELETE http://localhost:8081/api/items/{id}
```

---

## Node.js API

La carpeta [node-api/](node-api/) contiene la API REST que conecta la UI y las cachés zonales con Firebase / Firestore. Este componente se despliega en Vercel con despliegues automáticos desde GitHub.

### Responsabilidades

- Autenticación de usuarios con Firebase Auth usando `idToken` en `Authorization: Bearer ...`.
- Registro de cuentas con Firebase Auth y administración del usuario autenticado: consulta, actualización de correo/contraseña/nombre y eliminación de la cuenta.
- Gestión de dominios en Firestore.
- Verificación de dominios mediante registro TXT.
- Creación, edición y eliminación de reglas de URL por dominio.
- Creación y eliminación de API keys para consumo desde las cachés zonales.
- Exposición de `GET /api/cache-config/:domainName` para que la caché zonal obtenga su configuración.

### Variables de entorno

Configurar estas variables en local y en Vercel:

- `FIREBASE_PROJECT_ID`
- `FIREBASE_SERVICE_ACCOUNT_BASE64` o `FIREBASE_SERVICE_ACCOUNT_JSON`
- `CACHE_SERVICE_API_KEY`
- `NODE_API_URL`

Recomendación para Vercel: usar `FIREBASE_SERVICE_ACCOUNT_BASE64`.

### Ejecución local

```bash
cd node-api
npm install
npm run dev
```

### Verificación local

```bash
curl http://localhost:3001/api/health
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
- `GET /api/cache-config/:domainName`

### Verificación en Vercel

```bash
curl https://2026-01-ic-7602.vercel.app/api/health
```

O ingresar directamente en el navegador: https://2026-01-ic-7602.vercel.app/api/health

---

## Desinstalar

```bash
helm uninstall p2
```

> El PVC del Zonal Cache se conserva intencionalmente (`helm.sh/resource-policy: keep`). Para eliminarlo manualmente:
> ```bash
> kubectl delete pvc p2-cache-pvc
> ```