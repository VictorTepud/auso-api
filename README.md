# AUSO (AURA SOCIAL) API

API de red social construida en Rust con Actix-Web.

## Requisitos en Linux Mint

```bash
# Instalar Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
source "$HOME/.cargo/env"

# Instalar FFmpeg (para procesamiento de video)
sudo apt update
sudo apt install -y ffmpeg

# Instalar SQLite3
sudo apt install -y sqlite3 libsqlite3-dev
```

## Instalación

```bash
# Descomprimir
unzip auso-api.zip
cd auso-api

# Compilar (la primera vez tarda unos minutos)
cargo build --release

# Ejecutar
./target/release/auso-api
```

El servidor arranca en `http://0.0.0.0:8080`

## Configuración (.env)

Edita el archivo `.env` antes de ejecutar:

```
DATABASE_URL=sqlite:./auso.db?mode=rwc
JWT_SECRET=cambia_esto_en_produccion
JWT_EXPIRATION_HOURS=168
SERVER_HOST=0.0.0.0
SERVER_PORT=8080
UPLOAD_DIR=./uploads
MAX_IMAGE_SIZE_MB=10
MAX_VIDEO_SIZE_MB=500
MAX_IMAGES_PER_PACK=15
HLS_SEGMENT_DURATION=6
VIDEO_TARGET_HEIGHT=360
```

## Endpoints principales

### Autenticación
- `POST /api/v1/auth/register` — Registro (email + contraseña + username)
- `POST /api/v1/auth/login` — Login

### Usuarios
- `GET /api/v1/users/me` — Perfil propio
- `PUT /api/v1/users/me` — Actualizar perfil
- `POST /api/v1/users/me/profile-photo` — Subir foto de perfil
- `POST /api/v1/users/me/cover-photo` — Subir foto de portada
- `GET /api/v1/users/search?q=termino` — Buscar usuarios
- `GET /api/v1/users/{username}` — Ver perfil de otro usuario

### Seguir
- `POST /api/v1/users/{user_id}/follow` — Follow/Unfollow
- `GET /api/v1/users/{user_id}/followers` — Seguidores
- `GET /api/v1/users/{user_id}/following` — Seguidos

### Posts
- `POST /api/v1/posts/text` — Post de texto (con fondo color/imagen)
- `POST /api/v1/posts/image` — Post con imagen
- `POST /api/v1/posts/image-pack` — Paquete de imágenes (carousel/grid)
- `POST /api/v1/posts/{id}/images` — Agregar imágenes a un post
- `POST /api/v1/posts/video` — Post con video (se transcodifica a HLS 360p)
- `POST /api/v1/posts/poll` — Post con encuesta
- `GET /api/v1/posts/feed` — Feed (paginado)
- `GET /api/v1/posts/{id}` — Ver post
- `DELETE /api/v1/posts/{id}` — Eliminar post
- `POST /api/v1/posts/{id}/like` — Like/Unlike

### Comentarios
- `POST /api/v1/posts/{post_id}/comments` — Comentar
- `GET /api/v1/posts/{post_id}/comments` — Listar comentarios
- `DELETE /api/v1/comments/{id}` — Eliminar comentario

### Encuestas
- `POST /api/v1/polls/{id}/vote` — Votar
- `GET /api/v1/polls/{id}/results` — Resultados

### Comunidades
- `POST /api/v1/communities` — Crear comunidad
- `GET /api/v1/communities` — Listar/buscar comunidades
- `GET /api/v1/communities/{id}` — Ver comunidad
- `PUT /api/v1/communities/{id}` — Editar comunidad
- `POST /api/v1/communities/{id}/join` — Unirse
- `POST /api/v1/communities/{id}/leave` — Salirse
- `GET /api/v1/communities/{id}/members` — Miembros
- `POST /api/v1/communities/{id}/cover-photo` — Portada

### Canales (dentro de comunidades)
- `POST /api/v1/communities/{community_id}/channels` — Crear canal
- `GET /api/v1/communities/{community_id}/channels` — Listar canales
- `GET /api/v1/channels/{id}` — Ver canal
- `PUT /api/v1/channels/{id}` — Editar canal
- `DELETE /api/v1/channels/{id}` — Eliminar canal

### Grupos
- `POST /api/v1/groups` — Crear grupo
- `GET /api/v1/groups` — Listar/buscar grupos
- `GET /api/v1/groups/{id}` — Ver grupo
- `PUT /api/v1/groups/{id}` — Editar grupo
- `POST /api/v1/groups/{id}/join` — Unirse
- `POST /api/v1/groups/{id}/leave` — Salirse
- `GET /api/v1/groups/{id}/members` — Miembros
- `POST /api/v1/groups/{id}/cover-photo` — Portada

### Video Streaming
- `GET /api/v1/stream/{video_id}/master.m3u8` — Playlist HLS
- `GET /api/v1/stream/{video_id}/{segment}.ts` — Segmentos de video
- `GET /api/v1/stream/{video_id}/thumbnail.jpg` — Thumbnail

## Ejemplos curl

```bash
# Registro
curl -X POST http://localhost:8080/api/v1/auth/register \
  -H "Content-Type: application/json" \
  -d '{"email":"user@auso.com","password":"123456","username":"miuser"}'

# Login
curl -X POST http://localhost:8080/api/v1/auth/login \
  -H "Content-Type: application/json" \
  -d '{"email":"user@auso.com","password":"123456"}'

# Post de texto con fondo de color
curl -X POST http://localhost:8080/api/v1/posts/text \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer TU_TOKEN" \
  -d '{"content":"Hola AUSO!","background_color":"#FF5733"}'

# Crear comunidad
curl -X POST http://localhost:8080/api/v1/communities \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer TU_TOKEN" \
  -d '{"name":"Mi Comunidad","description":"Descripcion"}'

# Crear encuesta
curl -X POST http://localhost:8080/api/v1/posts/poll \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer TU_TOKEN" \
  -d '{"question":"Que prefieres?","options":["Opcion A","Opcion B","Opcion C"]}'
```

## Migrar a PostgreSQL

Cuando estes listo para usar PostgreSQL, cambia el `.env`:

```
DATABASE_URL=postgres://usuario:contrasena@localhost:5432/auso
```

Y en `Cargo.toml`, agrega la feature `postgres` a sqlx:

```toml
sqlx = { version = "0.8", features = ["runtime-tokio", "postgres", "chrono", "uuid"] }
```
