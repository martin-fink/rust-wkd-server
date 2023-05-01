FROM rust:latest as build-stage

COPY Cargo.lock .
COPY Cargo.toml .

RUN apt-get -y update && apt-get -y install clang llvm pkg-config nettle-dev

COPY src ./src
RUN set -x && cargo build --release

# Create a minimal docker image 
FROM debian:bullseye-slim

ENV RUST_LOG="error,wkd_server=info"
COPY --from=build-stage /target/release/wkd-server /wkd-server

EXPOSE 8080

VOLUME /openpgp-keys
CMD ["/wkd-server", "/openpgp-keys"]
