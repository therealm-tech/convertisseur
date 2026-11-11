<p align="center">
  <img src="assets/logo.svg" alt="Convertisseur logo" width="200">
</p>

# convertisseur

A gamified, mobile-first web app for learning the basics of Islam (Sunni,
Maliki school) and Arabic, classical and Algerian darija, through mini-games.

## Description

Convertisseur is a progressive web app with a French interface, installable on
a phone's home screen. It ships as a single container image: a Rust server that
answers the JSON API under `/api` and serves the React webapp for every other
path, backed by PostgreSQL and an S3-compatible object store for uploaded
media.

Registration is open on the Internet: an account is a username and a password,
with no email address. Accounts have one of three roles: `user`, `admin` or
`super_admin`. There is always exactly one super admin, the account named in
the server's configuration. Every account studies the two tracks, `arabe` and
`islam`: their units, their lessons, and each lesson's cards one at a time,
with the transliteration, the meaning and the recording on demand. Only what
an admin has validated, together with everything above it, is shown. In the
admin area, admins write and review the
learning content at `/admin/contenu`: units, lessons and cards, with audio
uploaded or recorded from the device's microphone, each "to review" until an
admin validates it. At `/admin/utilisateurs`, they search
accounts, reset forgotten passwords and delete accounts, and the super admin
also promotes users to admin and back.

See [ARCHITECTURE.md](ARCHITECTURE.md) for the design and the reasoning behind
it.

## Getting started

### Prerequisites

- Docker ≥ 25 with the Compose plugin

### Installation

```sh
git clone https://github.com/therealm-tech/convertisseur.git
```

```sh
cd convertisseur
```

### Configuration

The server reads its configuration from flags or environment variables;
`convertisseur-api serve --help` lists both.

| Variable | Required | Default | Description |
| --- | --- | --- | --- |
| `DATABASE_URL` | yes | — | PostgreSQL connection string |
| `SUPER_ADMIN_USERNAME` | yes | — | Username of the super admin, created or updated on every start-up |
| `SUPER_ADMIN_PASSWORD` | yes | — | Password of the super admin, at least 12 characters, applied on every start-up |
| `S3_ENDPOINT` | yes | — | URL of the S3-compatible object store, e.g. `https://s3.eu-west-3.amazonaws.com` |
| `S3_BUCKET` | yes | — | Bucket holding uploaded media; created on start-up when it does not exist |
| `S3_ACCESS_KEY_ID` | yes | — | Access key ID of the object store's credentials |
| `S3_SECRET_ACCESS_KEY` | yes | — | Secret access key of the object store's credentials |
| `S3_REGION` | no | `us-east-1` | Region of the bucket; a store with no regions accepts any |
| `S3_FORCE_PATH_STYLE` | no | `false` | Name the bucket in the URL's path rather than in its host name; most self-hosted stores, SeaweedFS and MinIO among them, need `true` |
| `BIND_ADDR` | no | `0.0.0.0:3000` | Address the HTTP server listens on |
| `DB_MAX_CONNECTIONS` | no | `10` | Size of the database connection pool |
| `WEBAPP_DIR` | no | `/usr/share/convertisseur/webapp` | Directory holding the built webapp |
| `LOG_FILTER` | no | `info` | [`tracing` filter directive](https://docs.rs/tracing-subscriber/latest/tracing_subscriber/filter/struct.EnvFilter.html#directives), e.g. `info,convertisseur_api=debug` |
| `LOG_FORMAT` | no | `text` | Log format: `text` or `json` |
| `COOKIE_SECURE` | no | `true` | Mark the session cookie `Secure`, so that browsers only send it over HTTPS; set `false` only to serve plain HTTP in development |
| `CLIENT_IP_HEADER` | no | — | Header in which the reverse proxy in front of the server passes the client's IP, e.g. `X-Forwarded-For`; its last address is used for rate limiting. Leave unset when clients connect directly, since they can forge it |
| `VITE_LOG_LEVEL` | no | `debug` in dev, `warn` in builds | Browser console verbosity (`debug`, `info`, `warn`, `error`, `silent`), read when the webapp is built |

Changing the super admin username demotes the previous super admin to `admin`
on the next start-up. Changing its password logs it out everywhere.

The object store's credentials need to read, write and delete objects in the
bucket and to look the bucket up; they need to create buckets only when the
bucket does not exist yet.

### Usage

Start PostgreSQL, SeaweedFS as the object store, and the app:

```sh
docker compose --profile app up --build
```

Then open <http://localhost:3000>, and register or log in. The local super
admin is `admin`, with the password set in
[docker-compose.yaml](docker-compose.yaml), which also turns off the `Secure`
cookie flag so that the session works over plain HTTP, and holds the
development-only credentials of SeaweedFS. The microphone works there because
browsers treat `http://localhost` as secure; from another device on the
network, over plain HTTP, the admin area can upload audio files but not
record.

| Path | What answers |
| --- | --- |
| `/api/…` | The JSON API |
| `/docs` | The API reference, rendered from `/openapi.json` |
| `/healthz` | Liveness probe: `200` while the process serves HTTP |
| `/readyz` | Readiness probe: `200` when PostgreSQL and the object store answer, `503` otherwise |
| anything else | The webapp |

### Deployment

Build the image:

```sh
docker build -t convertisseur .
```

Run it against a PostgreSQL database and an S3-compatible object store; it
applies the migrations and creates the bucket on start-up:

```sh
docker run --rm -p 3000:3000 \
  -e DATABASE_URL=postgres://user:password@db.example.com/convertisseur \
  -e SUPER_ADMIN_USERNAME=admin \
  -e SUPER_ADMIN_PASSWORD \
  -e S3_ENDPOINT=https://s3.eu-west-3.amazonaws.com \
  -e S3_REGION=eu-west-3 \
  -e S3_BUCKET=convertisseur-media \
  -e S3_ACCESS_KEY_ID \
  -e S3_SECRET_ACCESS_KEY \
  convertisseur
```

`-e SUPER_ADMIN_PASSWORD` with no value passes the variable from your shell,
so the password stays out of the command line; the same goes for the object
store's keys.

Back up the database and the bucket together: rows point at objects, and a
restore of one without the other leaves broken references or orphaned files.

Serve it over HTTPS: the session cookie is `Secure`, so browsers do not send it
back over plain HTTP, and browsers open the microphone, which the admin area
records audio with, only on HTTPS pages. Behind a reverse proxy, set `CLIENT_IP_HEADER` to the
header the proxy writes the client's address into, or every client shares the
proxy's rate limits.

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md).

## License

Apache-2.0 — see [LICENSE](LICENSE).
