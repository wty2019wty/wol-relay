# syntax=docker/dockerfile:1@sha256:4edf897a3ffa55b89f906fc8cc78afdb3f1834cc9c7083565e611a8a7d5fe99e

# --------------------------------------------------------------------------------------
# Chef — Rust toolchain plus cargo-chef, shared by the planner and build stages.
#
# rust:1-alpine 没有 i386 变体：x86（linux/386）构建时由 CI 传入
# BUILDER_IMAGE=alpine:3.24，Rust 工具链改由 apk 安装（见 publish-ghcr 工作流）。
# --------------------------------------------------------------------------------------

ARG BUILDER_IMAGE=rust:1-alpine@sha256:b1d5dd5a22a09404d3e798c0cf051b689077360374eb505fa5bb93c479693d87

FROM ${BUILDER_IMAGE} AS chef

# musl-dev: linker and headers for a static musl target
# rust/cargo: plain alpine builders (x86) do not ship a Rust toolchain
RUN if ! command -v cargo >/dev/null 2>&1; then apk add --no-cache rust cargo; fi \
    && apk add --no-cache musl-dev \
    && cargo install cargo-chef --locked

WORKDIR /app

# --------------------------------------------------------------------------------------
# Planner — extract a dependency-only recipe from the manifests.
# --------------------------------------------------------------------------------------

FROM chef AS planner

COPY . .
RUN cargo chef prepare --recipe-path recipe.json

# --------------------------------------------------------------------------------------
# Build — compile a static musl binary. Dependencies are built from the recipe first, so
# that layer stays cached until the dependencies change.
# --------------------------------------------------------------------------------------

FROM chef AS build

COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json

COPY . .
RUN cargo build --release

# --------------------------------------------------------------------------------------
# Run — minimal runtime image; the binary is fully static.
# --------------------------------------------------------------------------------------

FROM scratch

COPY --from=build /app/target/release/wol-relay /app/wol-relay

# Wake-on-LAN over UDP; relaying requires host networking (`docker run --net=host`)
EXPOSE 9/udp

ENTRYPOINT [ "/app/wol-relay" ]
