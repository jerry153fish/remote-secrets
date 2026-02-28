FROM rust:alpine AS build

RUN apk add --no-cache musl-dev openssl-dev pkgconfig perl make
WORKDIR /app

# 1) Cache deps: copy manifests only + stub sources
COPY Cargo.toml Cargo.lock ./
COPY main/Cargo.toml main/Cargo.toml
COPY crd/Cargo.toml crd/Cargo.toml
COPY utils/Cargo.toml utils/Cargo.toml
COPY plugins/Cargo.toml plugins/Cargo.toml
COPY k8s/Cargo.toml k8s/Cargo.toml

RUN mkdir -p main/src crd/src utils/src plugins/src k8s/src \
 && printf 'fn main() { println!("dummy"); }\n' > main/src/main.rs \
 && printf 'pub mod dummy;\n' > main/src/lib.rs \
 && touch main/src/dummy.rs \
 && printf 'fn main() { println!("dummy"); }\n' > crd/src/main.rs \
 && printf 'pub mod dummy;\n' > crd/src/lib.rs \
 && touch crd/src/dummy.rs \
 && printf 'pub mod dummy;\n' > utils/src/lib.rs \
 && touch utils/src/dummy.rs \
 && printf 'pub mod dummy;\n' > plugins/src/lib.rs \
 && touch plugins/src/dummy.rs \
 && printf 'pub mod dummy;\n' > k8s/src/lib.rs \
 && touch k8s/src/dummy.rs

RUN cargo build --release

# 2) Copy real sources (this should be the frequently-changing layer)
COPY crd crd
COPY main main
COPY utils utils
COPY plugins plugins
COPY k8s k8s

RUN cargo build --release

FROM alpine:3.23
RUN apk --no-cache add ca-certificates
WORKDIR /app
COPY --from=build /app/target/release/controller /app/controller
CMD ["./controller"]
