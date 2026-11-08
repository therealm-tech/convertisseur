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
path, backed by PostgreSQL.

Registration is open on the Internet. Accounts have one of three roles: `user`,
`admin` or `super_admin`. There is always exactly one super admin, the account
named in the server's configuration.

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
| `CONVERTISSEUR_SUPER_ADMIN_USERNAME` | yes | — | Username of the super admin, created or updated on every start-up |
| `CONVERTISSEUR_SUPER_ADMIN_PASSWORD` | yes | — | Password of the super admin, at least 12 characters, applied on every start-up |
| `CONVERTISSEUR_BIND_ADDR` | no | `0.0.0.0:3000` | Address the HTTP server listens on |
| `CONVERTISSEUR_DB_MAX_CONNECTIONS` | no | `10` | Size of the database connection pool |
| `CONVERTISSEUR_WEBAPP_DIR` | no | `/usr/share/convertisseur/webapp` | Directory holding the built webapp |
| `CONVERTISSEUR_LOG_FILTER` | no | `info` | [`tracing` filter directive](https://docs.rs/tracing-subscriber/latest/tracing_subscriber/filter/struct.EnvFilter.html#directives), e.g. `info,convertisseur_api=debug` |
| `CONVERTISSEUR_LOG_FORMAT` | no | `text` | Log format: `text` or `json` |
| `VITE_LOG_LEVEL` | no | `debug` in dev, `warn` in builds | Browser console verbosity (`debug`, `info`, `warn`, `error`, `silent`), read when the webapp is built |

Changing the super admin username demotes the previous super admin to `admin`
on the next start-up.

### Usage

Start PostgreSQL and the app:

```sh
docker compose --profile app up --build
```

Then open <http://localhost:3000>. The local super admin is `admin`, with the
password set in [docker-compose.yaml](docker-compose.yaml).

| Path | What answers |
| --- | --- |
| `/api/…` | The JSON API |
| `/docs` | The API reference, rendered from `/openapi.json` |
| `/healthz` | Liveness probe: `200` while the process serves HTTP |
| `/readyz` | Readiness probe: `200` when PostgreSQL answers, `503` otherwise |
| anything else | The webapp |

### Deployment

Build the image:

```sh
docker build -t convertisseur .
```

Run it against a PostgreSQL database; it applies the migrations on start-up:

```sh
docker run --rm -p 3000:3000 \
  -e DATABASE_URL=postgres://user:password@db.example.com/convertisseur \
  -e CONVERTISSEUR_SUPER_ADMIN_USERNAME=admin \
  -e CONVERTISSEUR_SUPER_ADMIN_PASSWORD \
  convertisseur
```

`-e CONVERTISSEUR_SUPER_ADMIN_PASSWORD` with no value passes the variable from
your shell, so the password stays out of the command line.

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md).

## License

Apache-2.0 — see [LICENSE](LICENSE).
