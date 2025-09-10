FROM --platform=$TARGETOS/$TARGETARCH rust:alpine AS builder

COPY ./ /work
WORKDIR /work

RUN apk add --update --no-cache musl-dev \
        && cargo build --release

FROM --platform=$TARGETOS/$TARGETARCH alpine

COPY --from=builder /work/target/release/cc-pre-proxy /usr/local/bin/pre-proxy

RUN apk add --update --no-cache tini
ENTRYPOINT ["/sbin/tini"]

WORKDIR /app
CMD ["/usr/local/bin/pre-proxy"]
