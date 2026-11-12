# Nodus full review — 2026-11-12

Scope: `web`, `sdk`, `recommender`, `hydrator`, plus deployment and the public-browsing path.
Code size: web 31.8k Rust lines, recommender 21.6k, sdk 13.3k, hydrator 1.1k.

## What is solid

- **Renderer split is coherent.** `nodus-ui` (pieces), `nodus-pages` (screen bodies),
  `nodus-route` (`RouteView`/`Runtime`), and the two hosts form a clean dependency DAG.
  SSR and CSR paint the same pages for data-backed routes.
- **Mastodon SDK is transport-neutral.** One endpoint layer over `ReqwestTransport` and
  `GlooTransport`; Misskey sits behind the same `InstanceApi` instead of a parallel client.
- **The recommendation feed is a real snapshot pipeline.** Candidates are merged from
  affinity, fresh, recent, exploration and trending pools; 200-row snapshots are persisted
  and cursor-paged, so later pages do not re-rank under the reader.
- **Failure states are explicit.** Feed/timeline/profile pages keep their own loading,
  unavailable, empty and degraded branches instead of falling back to a different screen.
- **Mobile layout has been exercised at 320/390 px**, including composer, threads, media,
  More and public pages. Horizontal overflow regressions were fixed at the component level.

## Critical gaps

1. **Hydrator writes still live in the calculator.** The service is read-through with a
   5-minute process cache; it does not write Postgres itself. The public profile browse
   path now enqueues `refresh_account`, which does persist (verified: 60 Misskey notes
   written for `Colon_BR@misskey.io`).
2. **Misskey storage is fixed by a compatibility key.** `status.remote_id` keeps the
   opaque id; `status.mastodon_id` holds a stable FNV-1a hash for legacy joins. A future
   schema can move to text ids without changing feed cursors.
3. **The web server now calls the hydrator.** `web/crates/web-server/src/public_api.rs`
   resolves local shadow profiles to their origin and falls back to `NODUS_HYDRATE_URL`
   (`http://10.0.1.17:8091` on LXC 100) when the origin's Mastodon API fails or is empty.
   Verified `Colon_BR@misskey.io` renders 7,250 origin posts anonymously.
4. **Hydrator search needs a token.** Mastodon `/api/v2/search` returns 401 anonymously;
   `NODUS_HYDRATE_SEARCH_TOKEN` is now forwarded as a bearer token, otherwise search
   reports the upstream 401.
5. **ActivityPub federation is still partial.** `nodus-federation` now has real
   RSA-SHA256 signing, verification and body digests behind `crypto`, but still no key
   fetch/cache, inbox replay window or delivery loop.
6. **Feed personalization is data-starved.** Production had 0 `interaction_event` rows,
   4 follow edges and few recent statuses. The vector profile (`interest.rs`) therefore
   clusters mostly followed-author text; `trending`/`recent` still dominate. Seeding
   interests from favourites/bookmarks is not done.
7. **CSR compose/admin bodies remain client-only.** `Runtime::body` mounts them; SSR paints
   shared placeholders. This is deliberate (ProseMirror/transport), but it means those
   routes are still not fully unified.
8. **Native quotes are blocked.** Instance is Mastodon 4.3.8; quote posts are 4.4+.

## Project notes

### sdk
- Good: feature composition; `nodus-sdk` is the composed entry point; Misskey adapter
  covers timelines, status, context, graph, notifications, post/delete, media mapping.
- Missing: Misskey login (MiAuth/OAuth), media upload (`drive/files`), streaming,
  capability detection, and an app-level software adapter that picks Mastodon vs Misskey.
- `nodus-federation` is a protocol skeleton, not yet a working signer/verifier.
- AP reader is happy-path JSON parsing; no collection paging beyond first page, no
  authorized-fetch support, no actor key validation.

### web
- Server-side public browsing now works anonymously and host-qualified; public pages hide
  reply/verbs when there is no session.
- `public_api.rs` is a BFF but still origin-first; wiring the hydrator would remove the
  browser's last dependency on remote origins and centralize rate/caching policy.
- SSR HTML cache is `no-store` in Caddy; assets are immutable. That is correct.
- Remaining security review needed for the public proxy: host allow/deny, response size,
  redirect handling, and SSRF (it currently validates the host but performs server-side
  fetches).
- Deferred UI requests still open: stable status UIDs, path-based nav everywhere,
  Saved under Library, streaming-box removal, full quote support.

### recommender
- Strong: snapshot pagination, per-author cap, cold-start fallback, discoverable gating,
  origin-first refresh for Mastodon, profile-edge expansion.
- Weak: schema is Mastodon-id-centric; no text-id path for Misskey.
- Job system lacks dead-letter/backoff visibility; `refresh_public` is a fixed federated
  page sweep, not a prioritised queue.
- Deploy script now stops services before replacing binaries, but remains manual and
  environment-specific.
- No retention policy for old snapshots/jobs/statuses; `expires_at` exists but cleanup
  policy is not documented here.

### hydrator
- Now has account, posts, followers/following pagination, thread, status, instance,
  search and AP endpoints, plus optional bearer auth and a TTL cache.
- The cache is process-local; no shared Redis. Restart loses it; multiple instances
  would not share it.
- No persistence and no write path. It cannot fill the DB.
- Misskey search/thread depth and AP reply paging are limited by each origin.

### infra
- No CI pipeline visible for these workspaces; checks/tests/clippy are manual.
- Deployments are hand-built in `rust:1-bookworm` containers and pushed with `pct`.
- Secrets/tokens live in systemd env files and DB; no rotation story documented.
- Backups exist for `/srv/nodus` during deploy; DB backup/restore not exercised here.

## Recommended order

1. **Decide the id model** for federated statuses (text `external_id` + internal
   `bigserial`, or a separate `remote_status` table). This unblocks Misskey persistence.
2. **Add a write path** to the hydrator or a hydrator→recommender ingest endpoint;
   call it from `/public-api` on post/profile views. Otherwise "hydrate" stays a cache.
3. **Wire `public-api` to the hydrator**, with `NODUS_HYDRATE_URL`, timeout, circuit
   breaker and fallback to origin.
4. **Make search authenticated or drop it** from the public hydrator surface.
5. **Implement federation signing/verification** (key fetch/cache, HTTP Signatures,
   digest, replay window) before claiming ActivityPub federation.
6. **Seed interest vectors from favourites/bookmarks** and add an interaction-event
   producer; then re-evaluate the feed mix with real data.
7. Add CI: `cargo check/test/clippy` per workspace, `web` wasm check, trunk build.
8. Finish the deferred UI/quote work only after the data path is trustworthy.

## Verification run for this review

- `cargo check --workspace` (hydrator): green.
- `cargo test --workspace` (hydrator): green (1 integration test).
- `cargo clippy --workspace --all-targets` (hydrator): green.
- Hydrator deployed to LXC 115; `/healthz` 200; account/posts/status/instance endpoints
  verified from LXC 100.
- Earlier verified: view/pages/route tests 18/6/4, server 7; web client wasm + trunk build;
  recommender store/feeds/calculator tests and production feed API.
