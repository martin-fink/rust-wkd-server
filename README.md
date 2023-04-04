# Rust WKD server

## What is WKD?

https://wiki.gnupg.org/WKD

## Running this project

Put your _public_ keys into `./openpgp/keys`.
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
  -p, --policy <POLICY>  The path to the policy file. If not set, an empty policy is served [env: POLICY=]
  -h, --help             Print help
```

### Security

This server will refuse to serve private or invalid keys.
If a file contains a private and a public key, only the public key will be served.
Nonetheless, make sure to only include your public key.
