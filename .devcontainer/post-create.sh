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

# Tailwind, daisyUI and the composer's bundles are npm packages in `web/package.json`.
# `crates/web-client/build.rs` regenerates the stylesheet and the bundles from source, so
# this container needs Node for the first build: `cd web && npm ci` once, then cargo does
# the rest (`web/.gitignore` keeps the generated files out of the tree).

echo "== node =="
node --version

echo "== npm packages =="
(cd web && npm ci --silent --no-audit --no-fund) || {
    echo "npm install did not finish; the first cargo build will retry it" >&2
}

# The docker CLI is only needed to drive the recommender's Postgres. The socket is mounted
# regardless, so the check is about the client, not about access.
if command -v docker >/dev/null 2>&1; then
    echo "== docker (shared host daemon) =="
    docker --version
    docker compose version || true

    echo "== postgres =="
    docker compose -f recommender/compose.yml up -d || {
        echo "postgres did not start; run 'docker compose -f recommender/compose.yml up -d' later" >&2
    }
else
    echo "== docker: no CLI in the image; the host daemon is still reachable at /var/run/docker.sock =="
fi

echo "== done =="
