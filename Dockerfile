# syntax=docker/dockerfile:1@sha256:4edf897a3ffa55b89f906fc8cc78afdb3f1834cc9c7083565e611a8a7d5fe99e

# --------------------------------------------------------------------------------------
# Chef — Rust toolchain plus cargo-chef, shared by the planner and build stages.
# --------------------------------------------------------------------------------------

FROM rust:1-alpine@sha256:b1d5dd5a22a09404d3e798c0cf051b689077360374eb505fa5bb93c479693d87 AS chef

# musl-dev: linker and headers for a static musl target
# file: asserts the built binary is static (scratch ships no dynamic loader)
RUN apk add --no-cache musl-dev file \
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
# 断言二进制是静态链接：动态二进制在 scratch 运行时只会报
# "exec /app/wol-relay: no such file or directory"，难以定位。
RUN cargo build --release \
    && if file target/release/wol-relay | grep -q interpreter; then \
        echo "error: wol-relay is dynamically linked, but the runtime image has no loader" >&2; \
        exit 1; \
    fi

# --------------------------------------------------------------------------------------
# Run — minimal runtime image; the binary is fully static.
# --------------------------------------------------------------------------------------

FROM scratch

COPY --from=build /app/target/release/wol-relay /app/wol-relay

# Wake-on-LAN over UDP; relaying requires host networking (`docker run --net=host`)
EXPOSE 9/udp

ENTRYPOINT [ "/app/wol-relay" ]
