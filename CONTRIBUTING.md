# Contributing

Questions and proposals go to the
[issue tracker](https://github.com/therealm-tech/convertisseur/issues). The
short version: install the tools below, keep `pre-commit` and the tests green,
and open a pull request.

## Development setup

The repository holds two projects: the Rust server in
[`convertisseur-api/`](convertisseur-api/) and the React webapp in
[`convertisseur-webapp/`](convertisseur-webapp/).

You need:

- **Rust**, through `rustup`; the toolchain is pinned in
  [rust-toolchain.toml](rust-toolchain.toml) and installed on first use:

  ```sh
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
  ```

- **Node.js** at the version in
  [convertisseur-webapp/.nvmrc](convertisseur-webapp/.nvmrc), with npm, for
  instance through [nvm](https://github.com/nvm-sh/nvm#installing-and-updating):

  ```sh
  nvm install "$(cat convertisseur-webapp/.nvmrc)"
  ```

- **Docker** with the Compose plugin, for PostgreSQL.
- **pre-commit** 4.x:

  ```sh
  pipx install pre-commit
  ```

- **hadolint** and **trivy**, which two hooks call from your `PATH`:

  **macOS**

  ```sh
  brew install hadolint trivy
  ```

  **Linux**: download the `hadolint` binary from its
  [releases](https://github.com/hadolint/hadolint/releases), and install
  `trivy` following its
  [installation guide](https://trivy.dev/latest/getting-started/installation/).

Then, from a fresh clone:

```sh
docker compose up -d --wait postgres
```

```sh
npm ci --prefix convertisseur-webapp
```

```sh
pre-commit install
```

`cargo run` and `cargo test` take their settings from
[.cargo/config.toml](.cargo/config.toml), which points them at the compose
PostgreSQL and sets a throwaway super admin; a variable already set in your
environment wins. Cargo applies them only to what it spawns, not to
`./target/debug/convertisseur-api` run by hand.

To work on the app with hot reload, run the server:

```sh
cargo run -- serve
```

and, in another terminal, the webapp's dev server, which proxies `/api` to it:

```sh
npm run dev --prefix convertisseur-webapp
```

then open the URL Vite prints. The server alone, on <http://localhost:3000>,
serves the last `npm run build` of the webapp. See the
[README](README.md#getting-started) for running the whole app in containers.

## Running the tests

The API tests need the compose PostgreSQL. Each `#[sqlx::test]` creates its
own database, applies the migrations and drops it afterwards, so tests never
see each other's data.

```sh
cargo test
```

One file, or one test:

```sh
cargo test --test bootstrap
```

```sh
cargo test --test bootstrap is_idempotent
```

The webapp tests run Vitest and React Testing Library on jsdom, with no server:

```sh
npm test --prefix convertisseur-webapp
```

```sh
npm test --prefix convertisseur-webapp -- src/__tests__/landing.test.tsx
```

New behaviour comes with tests, and a fix with a test that fails without it.

## Pre-commit hooks

The hooks in [.pre-commit-config.yaml](.pre-commit-config.yaml) must pass
before a commit is pushed.

```sh
pre-commit run --all-files
```

```sh
pre-commit run <hook-id> --all-files
```

| Hook | Needs | When it fails |
| --- | --- | --- |
| `cargo-fmt`, `biome-check` | the toolchain, `npm ci` | They rewrite the files; stage the result and commit again |
| `cargo-clippy` | the toolchain | Fix the lint; a lint that must be allowed gets a scoped `#[allow]` with the reason |
| `tsc` | `npm ci` | A type error in the webapp |
| `sqlfluff-lint` | nothing, pre-commit installs it | Reformat the SQL to the rule; [.sqlfluff](.sqlfluff) says which rules are off and why |
| `hadolint`, `trivy-dockerfile` | `hadolint`, `trivy` | A `Dockerfile` finding, fixed at the source |
| `no-co-authors` | nothing | The commit message carries a co-author or "generated with" line: remove it |

`--no-verify` and `SKIP=` are not the fix: a red hook is a real finding. If a
rule is wrong for this repository, change its configuration in the same pull
request and say why. CI runs the same hooks.

## Continuous integration

| Workflow | What it does | Reproduce locally |
| --- | --- | --- |
| [`quality.yaml`](.github/workflows/quality.yaml) | `pre-commit run --all-files`, the API tests against PostgreSQL, the webapp tests | `pre-commit run --all-files`, `cargo test`, `npm test --prefix convertisseur-webapp` |
| [`build.yaml`](.github/workflows/build.yaml) | Builds the image for `amd64` and `arm64` and fails on a fixable `HIGH` or `CRITICAL` vulnerability | `docker build -t convertisseur .` then `trivy image --severity HIGH,CRITICAL --ignore-unfixed convertisseur` |
| [`security.yaml`](.github/workflows/security.yaml) | Scans the lockfiles and the tree for vulnerabilities and secrets, and uploads the report to code scanning | `trivy fs --scanners vuln,secret .` |

`quality` and `security` run on every pull request and gate merging. `build`
runs only when a file that goes into the image changes, so it is not a
required check. `security` also runs daily: it can turn red with no change of
yours when a new vulnerability is published against a dependency.

[Dependabot](.github/dependabot.yaml) opens the dependency updates.

## Submitting a change

- Branch from `main`, one change per pull request.
- Commit subjects are imperative and lowercase, prefixed with the area they
  touch when it helps: `api: …`, `webapp: …`, `ci: …`. `git log --oneline`
  shows the style.
- A pull request is ready when CI is green and the documentation it affects
  (README, this file, [ARCHITECTURE.md](ARCHITECTURE.md)) is updated in the same
  pull request.
