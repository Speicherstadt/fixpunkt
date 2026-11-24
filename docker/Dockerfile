# syntax=docker/dockerfile:1
# Drei Stufen wie firsttris/haul docker/Dockerfile: UI (Bun) → Programmdatei (Rust) → Debian slim.

# ---- 1. Web-UI ---------------------------------------------------------------------------
FROM oven/bun:1.4.5 AS web
WORKDIR /src
COPY package.json bun.lock bunfig.toml ./
COPY ui ui
RUN bun install --frozen-lockfile && bun run build

# ---- 2. Programmdatei mit eingebetteter UI -----------------------------------------------
# cargo-chef: die Abhängigkeiten bauen in einer eigenen Schicht, die bis zur nächsten Änderung
# an Cargo.toml/Cargo.lock im Cache bleibt; bei Codeänderungen bauen nur die eigenen Crates.
FROM rust:1-bookworm AS chef
RUN cargo install cargo-chef --version 0.1.78 --locked
WORKDIR /src

FROM chef AS planner
COPY Cargo.toml Cargo.lock ./
COPY crates crates
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS server
COPY --from=planner /src/recipe.json recipe.json
RUN cargo chef cook --release -p fixpunkt-server --recipe-path recipe.json
COPY Cargo.toml Cargo.lock ./
COPY crates crates
COPY ui/messages ui/messages
COPY --from=web /src/ui/dist ui/dist
RUN cargo build --release -p fixpunkt-server && cp target/release/fixpunkt /fixpunkt

# ---- 3. Laufzeit -------------------------------------------------------------------------
FROM debian:bookworm-slim
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates tini \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --system --uid 1000 --home-dir /data --shell /usr/sbin/nologin fixpunkt \
    && mkdir -p /data && chown fixpunkt:fixpunkt /data
COPY --from=server /fixpunkt /usr/local/bin/fixpunkt

ENV FIXPUNKT_DATA_DIR=/data \
    FIXPUNKT_LISTEN=0.0.0.0:8080
USER fixpunkt
VOLUME ["/data"]
EXPOSE 8080
HEALTHCHECK --interval=30s --timeout=5s CMD ["/bin/bash", "-c", "exec 3<>/dev/tcp/127.0.0.1/8080 && printf 'GET /api/health HTTP/1.0\\r\\n\\r\\n' >&3 && grep -q ok <&3"]
ENTRYPOINT ["/usr/bin/tini", "--", "/usr/local/bin/fixpunkt"]
