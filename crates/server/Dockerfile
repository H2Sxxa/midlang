# syntax=docker/dockerfile:1

# Build context must be the workspace root: the server binary links the
# `midlang-core` crate and both are compiled from the same Cargo workspace.
#
#   docker build -f crates/server/Dockerfile -t midlang-server .

FROM rust:1.94-bookworm AS builder

WORKDIR /app

# Compile the dependency graph against stub sources first. This layer depends
# only on the manifests and the lockfile, so it is reused until a dependency
# changes and source-only edits stay fast to rebuild.
COPY Cargo.toml Cargo.lock ./
COPY crates/core/Cargo.toml crates/core/Cargo.toml
COPY crates/server/Cargo.toml crates/server/Cargo.toml
RUN mkdir -p crates/core/src crates/server/src \
    && printf '' > crates/core/src/lib.rs \
    && printf 'fn main() {}' > crates/server/src/main.rs \
    && cargo build --release -p midlang-server \
    && rm -rf crates/core/src crates/server/src

# Build the real workspace. Only the local crates recompile here.
COPY crates ./crates
RUN cargo build --release -p midlang-server

FROM debian:bookworm-slim AS runtime

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Run unprivileged and keep every mutable file under /data so it can be mounted
# as a volume: both translation.rdb and sqlite.db live there.
RUN useradd --create-home --uid 10001 midlang \
    && install -d -o midlang -g midlang /data

COPY --from=builder /app/target/release/midlang /usr/local/bin/midlang

USER midlang
WORKDIR /data
VOLUME ["/data"]

EXPOSE 4321

ENTRYPOINT ["midlang"]
CMD ["--host", "0.0.0.0", "--port", "4321", "--store", "redb", "--store-url", "/data/translation.rdb", "--sqlite-url", "/data/sqlite.db"]
