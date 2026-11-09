# The chess server's image. Only the backend is built; the desktop app is not part of it.

FROM rust:slim-trixie AS build

WORKDIR /build

RUN apt-get update \
    && apt-get install -y --no-install-recommends pkg-config cmake \
    && rm -rf /var/lib/apt/lists/*

COPY . .
# sqlx macros read .sqlx rather than a live database.
ENV SQLX_OFFLINE=true
# The cache mounts keep the downloaded crates and compiled dependencies between builds; the
# finished program is copied out of the cache before the layer ends.
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/build/target \
    cargo build --release --locked -p chess-backend \
    && install -D target/release/chess-backend /out/chess-backend \
    && strip /out/chess-backend

FROM gcr.io/distroless/cc-debian13:nonroot

COPY --from=build /out/chess-backend /usr/local/bin/chess-backend

USER nonroot:nonroot
EXPOSE 8080

ENTRYPOINT ["/usr/local/bin/chess-backend"]
