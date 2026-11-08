# The Convertisseur image: the Rust server, which serves both the JSON API and
# the built webapp.

# renovate: datasource=crate depName=cargo-chef
ARG CARGO_CHEF_VERSION=0.1.78

FROM node:26.13.0-alpine3.24@sha256:8996bb5821d62a8fd9bb27c1cb07462f3fdce6e47d640c6a881ead0b064ce216 AS webapp-builder
WORKDIR /usr/local/src/convertisseur/convertisseur-webapp
COPY convertisseur-webapp/package.json convertisseur-webapp/package-lock.json ./
RUN npm ci
COPY assets/ ../assets/
COPY convertisseur-webapp/ ./
RUN npm run build

FROM rust:1.99.1-alpine3.24@sha256:44819b27be4c97b0e4d7203bd2e02c4820f97700211b6a37fb4c246a2de59383 AS chef
ARG CARGO_CHEF_VERSION
RUN cargo install cargo-chef --locked --version "${CARGO_CHEF_VERSION}"
WORKDIR /usr/local/src/convertisseur

FROM chef AS planner
COPY Cargo.toml Cargo.lock ./
COPY convertisseur-api/ convertisseur-api/
RUN cargo chef prepare --recipe-path recipe.json

# The recipe ignores the package version, so a release bump keeps the cooked
# dependency layer cached.
FROM chef AS builder
COPY --from=planner /usr/local/src/convertisseur/recipe.json recipe.json
RUN cargo chef cook --release --locked --recipe-path recipe.json
COPY Cargo.toml Cargo.lock ./
COPY convertisseur-api/ convertisseur-api/
RUN cargo build --release --locked --bin convertisseur-api

# musl links statically: the binary needs no libc, only the CA roots and the
# nonroot user (65532) the distroless image provides.
FROM gcr.io/distroless/static-debian13:nonroot@sha256:25ac8f0c3b2334a9eb05452266bd4697864a8c8ef0ef71e5e6480d7f92868329
COPY --from=builder /usr/local/src/convertisseur/target/release/convertisseur-api /usr/local/bin/convertisseur-api
COPY --from=webapp-builder /usr/local/src/convertisseur/convertisseur-webapp/dist /usr/share/convertisseur/webapp
USER 65532:65532
EXPOSE 3000
HEALTHCHECK CMD ["/usr/local/bin/convertisseur-api", "healthcheck"]
ENTRYPOINT ["/usr/local/bin/convertisseur-api"]
CMD ["serve"]
