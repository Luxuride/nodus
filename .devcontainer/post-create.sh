#!/usr/bin/env bash
# Runs once when the container is created.
#
# Installs what the three sibling projects need beyond the base image:
#   * wasm32 target + trunk  -> web/  (Leptos client)
#   * docker + compose       -> recommender/ (Postgres for local development)
#
# Everything here is idempotent so it can be re-run by hand.
set -euo pipefail

echo "== toolchain =="
rustc --version
cargo --version

echo "== wasm32 target =="
rustup target add wasm32-unknown-unknown

echo "== trunk (prebuilt, no cargo compile) =="
if ! command -v trunk >/dev/null 2>&1; then
    tmp="$(mktemp -d)"
    curl -fsSL \
        "https://github.com/trunk-rs/trunk/releases/latest/download/trunk-x86_64-unknown-linux-gnu.tar.gz" \
        -o "$tmp/trunk.tar.gz"
    tar -xzf "$tmp/trunk.tar.gz" -C "$tmp"
    sudo install -m 0755 "$tmp/trunk" /usr/local/bin/trunk
    rm -rf "$tmp"
fi
trunk --version

echo "== docker (shared host daemon) =="
docker --version
docker compose version || true

echo "== postgres =="
docker compose -f recommender/compose.yml up -d || {
    echo "postgres did not start; run 'docker compose -f recommender/compose.yml up -d' later" >&2
}

echo "== done =="
