#!/usr/bin/env bash

cargo install cargo-machete

echo "Running \`cargo fmt\`"
cargo fmt --check

echo "Running \`cargo clippy\`"
cargo clippy

echo "Running \`cargo machete\`"
cargo machete
