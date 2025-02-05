alias t := test
alias b := build

build:
    cargo fmt
    cargo b

test:
    cargo fmt
    cargo nextest run -j 1
