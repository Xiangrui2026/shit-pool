FROM rust:1.98-bookworm AS build
WORKDIR /src
COPY . .
RUN cargo build --locked --release

FROM docker:29-cli AS docker-cli

FROM debian:bookworm-slim AS runtime
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates curl \
    && rm -rf /var/lib/apt/lists/* \
    && install -d -o 10001 -g 10001 /data/images /data/render
COPY --from=build /src/target/release/shit-pool-api /usr/local/bin/shit-pool-api
USER 10001:10001
ENV API_ADDR=0.0.0.0:3000 SHIT_POOL_IMAGE_DIR=/data/images
EXPOSE 3000
ENTRYPOINT ["/usr/local/bin/shit-pool-api"]

FROM runtime AS worker
COPY --from=docker-cli /usr/local/bin/docker /usr/local/bin/docker
CMD ["render-worker"]

FROM runtime AS api
