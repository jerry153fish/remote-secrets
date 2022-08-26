FROM ekidd/rust-musl-builder:stable as build

WORKDIR /app

COPY crd crd
COPY main main
COPY utils utils
COPY plugins plugins
COPY Cargo.lock .
COPY Cargo.toml .

RUN cargo build --release 

FROM alpine:latest as certs
RUN apk --update add ca-certificates

FROM scratch

WORKDIR /app

COPY --from=build /app/target/x86_64-unknown-linux-musl/release/controller /app
COPY --from=certs /etc/ssl/certs/ca-certificates.crt /etc/ssl/certs/ca-certificates.crt

CMD ["./controller"]