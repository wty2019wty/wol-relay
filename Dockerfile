FROM rust:1-alpine AS build

RUN apk add --no-cache musl-dev

RUN --mount=type=bind,source=.,target=/code \
    cargo build \
        --manifest-path=/code/Cargo.toml \
        --target-dir /target \
        --release \
        --locked

# -------------------------------------------------

FROM alpine:3.22

COPY --from=build /target/release/wol-relay /usr/local/bin/wol-relay

# Wake-on-LAN 走 UDP 广播，运行时需宿主网络（docker run --net=host）
ENTRYPOINT [ "/usr/local/bin/wol-relay" ]
