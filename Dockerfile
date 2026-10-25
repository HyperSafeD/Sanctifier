# syntax=docker/dockerfile:1.7
#
# Production image for the Sanctifier CLI.
#
#   docker build -t sanctifier .
#   docker run --rm --read-only --cap-drop ALL -v "$PWD:/workspace:ro" sanctifier analyze /workspace
#
# The builder stage carries the full Rust toolchain. The final stage is
# distroless/static: no shell, no package manager, no libc — just CA
# certificates, tzdata, /etc/passwd and the statically linked (musl) binary,
# running as the `nonroot` user (uid/gid 65532). See docs/docker.md.
#
# Dockerfile.ci is the separate build-tooling image and is not affected.

ARG RUST_VERSION=1.98
ARG DEBIAN_RELEASE=bookworm
ARG CARGO_AUDITABLE_VERSION=0.7.7

# ── Builder ───────────────────────────────────────────────────────────────────
FROM rust:${RUST_VERSION}-${DEBIAN_RELEASE} AS builder
ARG CARGO_AUDITABLE_VERSION

# Builder-only package that never reaches the final image; pinning the Debian
# revision would break the build on every point release.
# hadolint ignore=DL3008
RUN apt-get update \
    && apt-get install -y --no-install-recommends musl-tools \
    && rm -rf /var/lib/apt/lists/*

# `cargo auditable` embeds the dependency tree in the binary so image scanners
# (Trivy) can report vulnerable crates, not just OS packages.
RUN cargo install --locked "cargo-auditable@${CARGO_AUDITABLE_VERSION}"

WORKDIR /app

# Cargo needs every workspace member's manifest, so the whole (dockerignored)
# context is copied. sanctifier-cli does not enable sanctifier-core's `smt`
# feature, so no libz3 is required.
COPY . .

RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=/app/target \
    TARGET="$(uname -m)-unknown-linux-musl" \
    && rustup target add "$TARGET" \
    && cargo auditable build --release --locked -p sanctifier-cli --target "$TARGET" \
    && install -D -m 0755 "target/$TARGET/release/sanctifier" /out/sanctifier

# ── Runtime ───────────────────────────────────────────────────────────────────
FROM gcr.io/distroless/static-debian12:nonroot

LABEL org.opencontainers.image.title="sanctifier" \
      org.opencontainers.image.description="Stellar Soroban security analyzer" \
      org.opencontainers.image.source="https://github.com/HyperSafeD/Sanctifier" \
      org.opencontainers.image.licenses="MIT OR Apache-2.0"

COPY --from=builder --chown=root:root /out/sanctifier /usr/local/bin/sanctifier

USER nonroot:nonroot
WORKDIR /workspace

# `sanctifier serve` listens here when started with `--bind 0.0.0.0`.
EXPOSE 9100

ENTRYPOINT ["/usr/local/bin/sanctifier"]
CMD ["--help"]
