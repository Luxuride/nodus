# Nodus

A recommendation engine for Mastodon — people to follow, derived from what
similar people already follow, like and reply to — plus a web client that shows
them in a **Feed** tab next to the timelines.

This repository is the **superproject**: it holds the dev environment and pins every
project that makes up the system.

| Submodule | Repository | What it is |
| --- | --- | --- |
| `sdk/` | `nodus-sdk` | The client SDK: shared model, Mastodon/Misskey adapters, client state machine (`AppState`, `Command`, four ports). Source of truth for client state. |
| `server-sdk/` | `nodus-server-sdk` | Server-side protocol SDK: ActivityPub core, opt-in reader, federation, implementation extensions and service auth. |
| `hydrator/` | `nodus-hydrator` | Public federation reads: resolves profiles, posts, graphs and threads from origin instances. |
| `recommender/` | `nodus-recommender` | Recommendation engine, background sync worker, API, database migrations. |
| `web/` | `nodus-web` | Leptos SSR/CSR web client. Renders SDK state; owns no Mastodon logic. |
| `nodus-backend/` | `nodus-backend` | The Nodus server API and storage layer. |

The boundary is deliberate: the recommender computes rankings and never renders; the
web client renders and never scores; the client SDK sits between them and is the only
place that knows what client state looks like; the server SDK owns protocol work that
never runs in a client. See
[`sdk/docs/`](sdk/docs/README.md) (start with
[02-architecture.md](sdk/docs/02-architecture.md)) and
[`web/docs/02-architecture.md`](web/docs/02-architecture.md).

## Layout

```text
Nodus/
├── .devcontainer/        Rust 1.99, wasm32 target, trunk, docker CLI, zsh
├── .gitmodules           sdk, server-sdk, hydrator, recommender, web, nodus-backend
├── sdk/                  crates/{shared,client-core,mastodon,misskey,client-sdk,runtime,state,discovery,persist,protocol}
│   └── docs/             README, 01-scope … 06-crates
├── server-sdk/           crates/{activitypub-core,activitypub-client,activitypub,federation,extensions/{mastodon,nodus},service,server}
├── hydrator/             crates/{hydrator,service}
├── recommender/
│   ├── compose.yml       Postgres 17 + pgvector
│   ├── crates/           {core,store,ingest,api}
│   └── docs/             README, 01-vision … 10-roadmap, decisions
└── web/
    ├── Trunk.toml
    ├── crates/web/       Leptos app
    └── docs/             README, 01-scope … 05-quality
```

## Getting started

```bash
git clone --recurse-submodules git@github.com:luxuride/Nodus.git
cd Nodus
code .          # reopen in the devcontainer; Postgres and the Rust toolchain come up with it
```

If you cloned without submodules:

```bash
git submodule update --init --recursive
```

Then, per project:

```bash
# API + worker (recommender/)
docker compose up -d
cargo run --bin nodus-api

# client (web/)
cd web && trunk serve --port 8080     # proxies /api to the API on :3000
```

## Documentation

Documentation comes before code in this project, and it is split the same way the
repositories are:

- **Backend / system design** — [`recommender/docs/`](recommender/docs/README.md): vision,
  architecture, data model, ingestion, similarity, ranking, API, privacy,
  operations, roadmap, decisions.
- **Client** — [`web/docs/`](web/docs/README.md): scope and parity targets,
  architecture, data layer, UI, quality budgets.
- **SDK** — [`sdk/docs/`](sdk/docs/README.md): scope, architecture and the client
  state model, contract versioning, roadmap; [`sdk/README.md`](sdk/README.md) is the
  crate overview.

The contract between the sides is versioned; see
[`sdk/docs/03-contract.md`](sdk/docs/03-contract.md) § Version semantics.

## Status

Pre-implementation. The workspace builds green (`cargo clippy -- -D warnings`,
`trunk build --release`) with documentation-only crates and an SDK state machine
that has no transports yet. Nothing is deployed and nothing talks to a real
instance.

## License

AGPL-3.0-or-later. See [`LICENSE`](LICENSE). A network service whose value is
improved by user-contributed code should stay open to those users; AGPL also
matches Mastodon itself, which keeps licensing of derived clients unambiguous.

## Naming and trademarks

Nodus is not affiliated with, endorsed by, or sponsored by Mastodon GmbH.
"Mastodon" is a trademark of Mastodon GmbH and is used here only to describe
compatibility — the mark never appears in the project, repository, crate or
binary names, and no Mastodon artwork is used. The rules we follow, and the
checklist for anything new, are in
[`recommender/docs/11-trademark.md`](recommender/docs/11-trademark.md).
