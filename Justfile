alias t := test
alias b := build

build:
    cargo fmt
    cargo clippy
    cargo b

test:
    cargo fmt
    cargo nextest run -j 1 --retries 2
