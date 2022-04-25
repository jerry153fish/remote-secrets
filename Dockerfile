FROM ekidd/rust-musl-builder:stable as build

WORKDIR /app

COPY src src
COPY Cargo.lock .
COPY Cargo.toml .

RUN cargo build --release 

FROM scratch

WORKDIR /app

COPY --from=build /app/target/x86_64-unknown-linux-musl/release/controller /app

CMD ["./controller"]