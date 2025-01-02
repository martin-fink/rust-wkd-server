FROM rust:latest as build

WORKDIR /build

COPY Cargo.toml Cargo.lock ./

RUN mkdir src && echo "fn main() {}" > src/main.rs

# cache dependencies
RUN cargo build --release

COPY src ./src

# make sure main.rs is rebuilt
RUN touch src/main.rs
RUN cargo build --release

# Create a minimal docker image
FROM cgr.dev/chainguard/glibc-dynamic

ENV RUST_LOG="error,wkd_server=info"
COPY --from=build --chown=nonroot:nonroot /build/target/release/wkd-server /usr/local/bin/

EXPOSE 8080

VOLUME /openpgp-keys
CMD ["/usr/local/bin/wkd-server", "/openpgp-keys"]
