# syntax=docker/dockerfile:1

ARG CODEX_RUST_IMAGE=rust:1.95-bookworm
FROM ${CODEX_RUST_IMAGE} AS builder

ARG CODEX_VERSION=0.155.1
ARG CARGO_BUILD_JOBS=6
ARG CARGO_PROFILE_RELEASE_LTO=false
ARG CARGO_PROFILE_RELEASE_CODEGEN_UNITS=16

ENV CARGO_BUILD_JOBS=${CARGO_BUILD_JOBS} \
    CARGO_PROFILE_RELEASE_LTO=${CARGO_PROFILE_RELEASE_LTO} \
    CARGO_PROFILE_RELEASE_CODEGEN_UNITS=${CARGO_PROFILE_RELEASE_CODEGEN_UNITS}

RUN apt-get update \
 && apt-get install -y --no-install-recommends \
      build-essential ca-certificates clang libcap-dev pkg-config protobuf-compiler \
 && rm -rf /var/lib/apt/lists/*

WORKDIR /src/codex
COPY . .

RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=/src/codex/codex-rs/target \
    test "$(grep -c '^version = \"0.0.0\"$' codex-rs/Cargo.toml)" -eq 1 \
 && sed -i "s/^version = \"0.0.0\"$/version = \"${CODEX_VERSION}\"/" \
      codex-rs/Cargo.toml codex-rs/Cargo.lock \
 && cd codex-rs \
 && cargo build --locked --release -p codex-app-server --bin codex-app-server \
 && install -D -m 0755 target/release/codex-app-server /rootfs/usr/local/bin/codex-app-server \
 && strip --strip-unneeded /rootfs/usr/local/bin/codex-app-server \
 && install -D -m 0644 ../LICENSE /rootfs/usr/share/licenses/codex/LICENSE \
 && install -D -m 0644 ../NOTICE /rootfs/usr/share/licenses/codex/NOTICE \
 && install -D -m 0644 ../integrations/ai-teaching/runtime/MODIFICATIONS.txt \
      /rootfs/usr/share/licenses/codex/MODIFICATIONS \
 && install -D -m 0644 ../integrations/ai-teaching/runtime/requirements.toml \
      /rootfs/etc/codex/requirements.toml \
 && mkdir -p /rootfs/usr/share/codex/ai-teaching-skills \
 && cp -R ../integrations/ai-teaching/skills/. /rootfs/usr/share/codex/ai-teaching-skills/

FROM scratch AS runtime
ARG CODEX_REVISION=unknown
ARG CODEX_VERSION=0.155.1
ARG CODEX_RUNTIME_RECIPE=2
LABEL org.opencontainers.image.source="https://github.com/NytePlus/codex" \
      org.opencontainers.image.revision="${CODEX_REVISION}" \
      org.opencontainers.image.version="${CODEX_VERSION}" \
      org.ai-teaching.codex.runtime-recipe="${CODEX_RUNTIME_RECIPE}"
COPY --from=builder /rootfs /rootfs
