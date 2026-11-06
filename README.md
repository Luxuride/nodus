# MastodonReccom

A recommendation engine for Mastodon — people to follow, derived from what
similar people already follow, like and reply to — plus a web client that shows
them in a **Feed** tab next to the timelines.

This repository is the **superproject**: it holds the dev environment and pins the
three projects that make up the system.

| Submodule | Repository | What it is |
| --- | --- | --- |
| `sdk/` | `mastodon-reccom-sdk` | The client SDK: wire contract, Mastodon domain model, and the client state machine (`AppState`, `Command`, four ports). Source of truth for client state. |
| `backend/` | `mastodon-reccom-backend` | Recommendation engine, background sync worker, API, database migrations. |
| `web/` | `mastodon-reccom-web` | Leptos/CSR web client. Renders SDK state; owns no Mastodon logic. |

The boundary is deliberate: the backend computes rankings and never renders; the
web client renders and never scores; the SDK sits between them and is the only
place that knows what the client's state looks like. See
[`backend/docs/08-sdk.md`](backend/docs/08-sdk.md) and
[`web/docs/02-architecture.md`](web/docs/02-architecture.md).

## Layout

```text
MastodonReccom/
├── .devcontainer/        Rust 1.99, wasm32 target, trunk, docker CLI, zsh
├── .gitmodules           sdk → backend → web
├── sdk/                  crates/{shared,mastodon,sdk}
│   └── README.md         start here for the SDK
├── backend/
│   ├── compose.yml       Postgres 17 + pgvector
│   ├── crates/           {core,store,ingest,api}
│   └── docs/             README, 01-vision … 11-roadmap, decisions
└── web/
    ├── Trunk.toml
    ├── crates/web/       Leptos app
    └── docs/             README, 01-scope … 05-quality
```

## Getting started

```bash
git clone --recurse-submodules git@github.com:luxuride/MastodonReccom.git
cd MastodonReccom
code .          # reopen in the devcontainer; Postgres and the Rust toolchain come up with it
```

If you cloned without submodules:

```bash
git submodule update --init --recursive
```

Then, per project:

```bash
# API + worker (backend/)
docker compose up -d
cargo run --bin mastodon-reccom-api

# client (web/)
cd web && trunk serve --port 8080     # proxies /api to the API on :3000
```

## Documentation

Documentation comes before code in this project, and it is split the same way the
repositories are:

- **Backend / system design** — [`backend/docs/`](backend/docs/README.md): vision,
  architecture, data model, ingestion, similarity, ranking, API, SDK contract,
  privacy, operations, roadmap, decisions.
- **Client** — [`web/docs/`](web/docs/README.md): scope and parity targets,
  architecture, data layer, UI, quality budgets.
- **SDK** — [`sdk/README.md`](sdk/README.md).

The contract between the two sides is versioned; see
[`backend/docs/08-sdk.md`](backend/docs/08-sdk.md) § Versioning.

## Status

Pre-implementation. The workspace builds green (`cargo clippy -- -D warnings`,
`trunk build --release`) with documentation-only crates and an SDK state machine
that has no transports yet. Nothing is deployed and nothing talks to a real
instance.

## License

AGPL-3.0-or-later. A network service whose value is improved by user-contributed
code should stay open to those users; AGPL also matches Mastodon itself, which
keeps licensing of derived clients unambiguous.
