FROM rust:alpine as build

RUN apk add --no-cache musl-dev openssl-dev pkgconfig perl make
RUN export OPENSSL_LIB_DIR="/usr/lib/x86_64-linux-gnu"; export OPENSSL_INCLUDE_DIR="/usr/include/openssl"
WORKDIR /app

COPY Cargo.lock .
COPY Cargo.toml .

COPY main/Cargo.toml main/Cargo.toml
RUN mkdir ./main/src && echo 'fn main() { println!("Dummy!"); }' > ./main/src/main.rs
RUN echo 'pub mod test;' > ./main/src/lib.rs
RUN touch ./main/src/test.rs

COPY crd/Cargo.toml crd/Cargo.toml
RUN mkdir ./crd/src && echo 'fn main() { println!("Dummy!"); }' > ./crd/src/main.rs
RUN echo 'pub mod test;' > ./crd/src/lib.rs
RUN touch ./crd/src/test.rs

COPY utils/Cargo.toml utils/Cargo.toml
RUN mkdir ./utils/src && echo 'pub mod test;' > ./utils/src/lib.rs
RUN touch ./utils/src/test.rs

COPY plugins/Cargo.toml plugins/Cargo.toml
RUN mkdir ./plugins/src && echo 'pub mod test;' > ./plugins/src/lib.rs
RUN touch ./plugins/src/test.rs

COPY k8s/Cargo.toml k8s/Cargo.toml
RUN mkdir ./k8s/src && echo 'pub mod test;' > ./k8s/src/lib.rs
RUN touch ./k8s/src/test.rs

RUN cargo build --release 

RUN rm -rf ./main
RUN rm -rf ./crd
RUN rm -rf ./utils
RUN rm -rf ./plugins
RUN rm -rf ./k8s
COPY crd crd
RUN touch -a -m ./crd/src/main.rs
RUN touch -a -m ./crd/src/lib.rs
COPY main main
RUN touch -a -m ./main/src/main.rs
RUN touch -a -m ./main/src/lib.rs
COPY utils utils
RUN touch -a -m ./utils/src/lib.rs
COPY plugins plugins
RUN touch -a -m ./plugins/src/lib.rs
COPY k8s k8s
RUN touch -a -m ./k8s/src/lib.rs
RUN cargo build --release 

FROM alpine:3.20
RUN apk --update add ca-certificates

WORKDIR /app

COPY --from=build /app/target/release/controller /app

CMD ["./controller"]