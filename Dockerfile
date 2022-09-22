FROM rust:buster as build

WORKDIR /app

COPY crd crd
COPY main main
COPY utils utils
COPY plugins plugins
COPY k8s k8s
COPY Cargo.lock .
COPY Cargo.toml .

RUN cargo build --release 

FROM debian:buster-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=build /app/target/release/controller /app

CMD ["./controller"]