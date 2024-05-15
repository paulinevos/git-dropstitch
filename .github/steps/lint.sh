#!/usr/bin/env bash

cargo add cargo-machete

cargo fmt
cargo clippy
cargo machete
