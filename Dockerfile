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

# Wake-on-LAN 走 UDP 广播，运行时需宿主网络（docker run --net=host）。
# 监听 UDP 9 属于特权端口，因此默认以 root 运行；如需以非 root 运行，
# 请通过 --user 指定用户并授予 CAP_NET_BIND_SERVICE。
EXPOSE 9/udp
ENTRYPOINT [ "/usr/local/bin/wol-relay" ]
