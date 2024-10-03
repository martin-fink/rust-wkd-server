FROM debian:slim

ENV RUST_LOG="error,wkd_server=info"
COPY artifacts/wkd-server /wkd-server

VOLUME /openpgp-keys
CMD ["/wkd-server", "/openpgp-keys"]
