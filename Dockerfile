ARG VERSION=0.1.0
FROM alpine:3.20 AS fetcher
RUN apk add --no-cache curl ca-certificates
ARG VERSION
RUN curl -fsSL -o /sanctifier \
    "https://github.com/HyperSafeD/Sanctifier/releases/download/v${VERSION}/sanctifier-linux-amd64-musl" \
    && chmod +x /sanctifier \
    && curl -fsSL -o /sanctifier.sha256 \
    "https://github.com/HyperSafeD/Sanctifier/releases/download/v${VERSION}/sanctifier-linux-amd64-musl.sha256" \
    && cd / && sha256sum -c sanctifier.sha256

FROM scratch
COPY --from=fetcher /sanctifier /usr/local/bin/sanctifier
WORKDIR /workspace
ENTRYPOINT ["/usr/local/bin/sanctifier"]
CMD ["--help"]
