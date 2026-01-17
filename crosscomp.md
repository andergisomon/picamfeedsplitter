set env vars for libcamera build scripts to work for cross comp:

can't seem to get zigbuild to work properly. managed to get libcamera to build with these env vars setup, but iceroxy2 has trouble with posix APIs.

gave up making zigbuild and cross-rs work, so I used an LLM to create a docker image

Build docker image:

```bash
docker build -t pi-builder -f Dockerfile.cross .
```

Run and build:

```bash
docker run --rm -t -v "$(pwd)":/app -w /app pi-builder \
    cargo build --target aarch64-unknown-linux-gnu --release --color=always
```

```bash
docker run --rm -t \
    -v "$(pwd)":/app \
    -v ~/.cargo/registry:/root/.cargo/registry \
    -v ~/.cargo/git:/root/.cargo/git \
    -w /app pi-builder \
    cargo build --target aarch64-unknown-linux-gnu --release --color=always
```
