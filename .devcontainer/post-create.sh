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

echo "== tailwindcss (prebuilt, no node) =="
# The web client's stylesheet is compiled by the Tailwind standalone CLI. Pinned:
# a Tailwind minor can change which utilities exist, and the CSS is not reviewed on
# every bump. `web/Trunk.toml` runs it as a pre-build hook.
TAILWIND_VERSION="4.3.5"
if ! command -v tailwindcss >/dev/null 2>&1; then
    tmp="$(mktemp -d)"
    curl -fsSL \
        "https://github.com/tailwindlabs/tailwindcss/releases/download/v${TAILWIND_VERSION}/tailwindcss-linux-x64" \
        -o "$tmp/tailwindcss"
    chmod 0755 "$tmp/tailwindcss"
    sudo install -m 0755 "$tmp/tailwindcss" /usr/local/bin/tailwindcss
    rm -rf "$tmp"
fi
tailwindcss --help >/dev/null && echo "tailwindcss ${TAILWIND_VERSION}"

echo "== docker (shared host daemon) =="
docker --version
docker compose version || true

echo "== postgres =="
docker compose -f recommender/compose.yml up -d || {
    echo "postgres did not start; run 'docker compose -f recommender/compose.yml up -d' later" >&2
}

echo "== done =="
