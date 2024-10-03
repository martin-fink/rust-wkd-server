FROM rust:alpine as build-stage

WORKDIR /build

RUN apk add --no-cache musl-dev

COPY Cargo.toml Cargo.lock /build/

RUN mkdir /build/src && echo "fn main() {}" > /build/src/main.rs

# cache dependencies
RUN cargo build --release

COPY src ./src

# make sure main.rs is rebuilt
RUN touch /build/src/main.rs
RUN cargo build --release

# Create a minimal docker image
FROM alpine

ENV RUST_LOG="error,wkd_server=info"
COPY --from=build-stage /build/target/release/wkd-server /wkd-server

EXPOSE 8080

VOLUME /openpgp-keys
CMD ["/wkd-server", "/openpgp-keys"]
