# Rust WKD server

## What is WKD?

https://wiki.gnupg.org/WKD

## Running this project

Put your **public** keys into `./openpgp/keys`.
The name of the file does not matter, this service extracts all user ids and serves them.
By default, every key in the directory is loaded and all associated user IDs are made available via the API.
You can restrict responses to a specific user ID by enabling the `--split-keys` option or setting the `SPLIT_KEYS=true`
environment variable. In this mode, each request will only return the matching user ID and its corresponding key.

Optionally, put your policy into a text file in `./openpgp`.

```shell
cargo build --release
./target/release/wkd-server ./openpgp/keys
```

### Usage

```
Usage: wkd-server [OPTIONS] <KEYS_PATH>

Arguments:
  <KEYS_PATH>  The path where the GPG keys are stored

Options:
      --address <ADDRESS>  Address to bind the HTTP server to. Defaults to 0.0.0.0 to listen on all interfaces [env: ADDRESS=] [default: 0.0.0.0]
      --port <PORT>        Port to bind the HTTP server to. Defaults to 8080 [env: PORT=] [default: 8080]
  -p, --policy <POLICY>    The path to the policy directory. If not set, an empty policy is served [env: POLICY=]
      --split-keys         Split certificate into individual user IDs. If set, only the requested user ID and corresponding key will be returned from the certificate. Otherwise, the response will include all user IDs and keys found in the file [env: SPLIT_KEYS=]
  -h, --help               Print help
```

### Policy

The policy directory can contain the following files:
- `default`: This is the default policy served for all domains, if no more specific policy can be found.
- `$domain`: This is the policy that should be served for a specific domain. Example: `example.com`.

### Security

This server will refuse to serve private or invalid keys.
If a file contains a private and a public key, only the public key will be served.
Nonetheless, make sure to only include your public key.

### Deployment

You can use this `docker-compose.yaml` example file as a starting off point for your
deployment. Make sure to add your public keys as a volume.

```yaml
services:
  wkd-server:
    image: ghcr.io/martin-fink/rust-wkd-server:latest
    volumes:
      - ./keys:/openpgp-keys:ro
    ports:
      - 127.0.0.1:8080:8080
    environment:
      - RUST_LOG=error,wkd_server=info # change this to trace for debugging
    labels:
      - "traefik.enable=true"
      - "traefik.http.routers.rust-wkd-server.rule=(Host(`<your-domain>`) && PathPrefix(`/.well-known/openpgpkey`))"
      - "traefik.http.routers.rust-wkd-server.entrypoints=<your-https-entrypoint>"
      - "traefik.http.routers.rust-wkd-server.tls=true"
      - "traefik.http.routers.rust-wkd-server.tls.certResolver=<your-certResolver>"
      - "traefik.services.rust-wkd-server.loadbalancer.passHostHeader=true"
```

#### Reverse proxy setup

You probably want to move this behind a reverse proxy such as nginx in order for it to handle https.
You can use the following snippets for nginx.

##### Advanced method

Set up a subdomain `openpgpkey.`, e.g., `openpgpkey.example.org`;
The WKD client will try to access `https://openpgpkey.example.org/.well-known/openpgpkey/example.org/hu/{hash}`.

```nginx
server_name openpgpkey.example.org;

location ^~ /.well-known/openpgpkey {
    resolver 127.0.0.11 valid=5s;
    set $upstream_endpoint http://address:port;
    proxy_pass $upstream_endpoint;
    proxy_http_version 1.1;
    proxy_set_header X-Forwarded-Host $host;
}
```

##### Direct method

The important bit is to set the `X-Forwarded-Host` header, as that header is used to differentiate domains.
In this case, the WKD client will try to access `https://example.org/.well-known/openpgpkey/hu/{hash}`

```nginx
server_name example.org;

location ^~ /.well-known/openpgpkey {
    resolver 127.0.0.11 valid=5s;
    set $upstream_endpoint http://address:port;
    proxy_pass $upstream_endpoint;
    proxy_http_version 1.1;
    proxy_set_header X-Forwarded-Host $host;
}
```
