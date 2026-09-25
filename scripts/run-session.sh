#!/bin/sh
set -eu

deadline_unix="$(($(date +%s) + 3600))"
cargo run --bin session_archive -- rust-101 learner-42 "Ada Learner" "$deadline_unix"

