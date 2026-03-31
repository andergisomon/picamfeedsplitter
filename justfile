alias b := build
alias s := send

build:
    docker run --rm -t \
    -v "$(pwd)":/app \
    -v ~/.cargo/registry:/root/.cargo/registry \
    -v ~/.cargo/git:/root/.cargo/git \
    -w /app pi-builder \
    cargo build --target aarch64-unknown-linux-gnu --release --color=always

send:
    rsync -avz -e "ssh -i ~/gipop_plc" /Users/ander/Documents/proj/palanuk_splitter_audit/splitter/target/aarch64-unknown-linux-gnu/release/splitter pi@raspberrypi.local:/home/pi/palanuk/anc/splitter
