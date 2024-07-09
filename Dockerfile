# Setup
FROM rust:alpine AS chef
ENV RUSTFLAGS="-C target-feature=-crt-static"
ENV PKG_CONFIG_ALLOW_CROSS=1
ENV PKG_CONFIG_PATH=/usr/lib/pkgconfig:/usr/share/pkgconfig

RUN apk update && apk add --no-cache openssl-dev pkgconfig libc-dev
RUN cargo install cargo-chef
WORKDIR /midnight


FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json


FROM chef AS builder
COPY --from=planner /midnight/recipe.json recipe.json
# Build dependencies
RUN cargo chef cook --release --recipe-path recipe.json
# Build application
COPY . .
RUN cargo build --release --bin bot


# Run on clean alpine
FROM alpine:3 AS runtime
RUN apk update && apk add --no-cache pkgconfig libgcc
WORKDIR /midnight
COPY --from=builder /midnight/target/release/bot .
COPY --from=builder /midnight/patterns.json .
ENTRYPOINT ["/midnight/bot"]
