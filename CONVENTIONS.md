# Conventions

Rules that apply to every project in this superproject. They exist because this
codebase is read more often than it is written, and a 700-line file is a file
nobody reviews properly.

## Source files stay small

| size | verdict |
| --- | --- |
| ≤ 250 lines | normal |
| 251–400 lines | tolerated only if the file is one coherent thing (a single transport impl) |
| > 400 lines | split it before the next commit that touches it |

The number is about *review*, not aesthetics: a diff that touches one 200-line
file can be read in a sitting, and a reviewer can hold one responsibility in
their head.

**How to split**

- By responsibility, not by line count. `sync_user.rs` is 723 lines because it
  contains token selection, pagination, HTTP calls and id mapping — four things
  that fail for four different reasons.
- Into a directory with `mod.rs` as the seam:

  ```text
  handlers/sync_user/
  ├── mod.rs        the handler: orchestration only
  ├── config.rs     SyncConfig and its validation
  ├── accounts.rs   own-account resolution and upsert
  ├── follows.rs    page walking and edge collection
  └── tests.rs      #[cfg(test)] mod tests;
  ```

- **Tests are the usual offender.** Moving `#[cfg(test)] mod tests` into a
  sibling `tests.rs` (declared as `#[cfg(test)] mod tests;` from the file it
  tests) costs nothing and keeps the production code visible in a diff.
  Integration tests stay in `crates/*/tests/`.
- One public type per file when the type has behaviour; several related plain
  structs may share a file when they are only ever used together.

**What is not a reason to split**: a function that is long but linear (a
`match` over many kinds) is read top-to-bottom and splitting it into eight
one-line functions makes it worse. Split when the *reasons to change* differ.

## Edit with the tools, not with the shell

`read` a file before changing it, and change it with `edit` — a whole-file write
is for new files and rewrites you have actually read. Scripts that mutate source
(`sed -i`, `python3 -c`, heredocs) have already produced two bad files here: a
mistyped variable overwrote `routes/mod.rs` with the contents of `main.rs`, and a
sed rename rewrote a type alias into `pub type UnitFuture<'a> = UnitFuture<'a>;`.
The editor refuses an ambiguous anchor; a script does not.

Shell stays for reading state (`git status`, `grep`, `wc -l`, `find`) and for
commands no tool covers. A repo-wide mechanical rename is the exception, and only
with `git diff` reviewed line by line afterwards.

## Other rules, stated once

- Documentation says **why**; the code says what. A comment restating the line
  below it is noise, a comment explaining why the obvious approach is wrong is
  the most valuable line in the file.
- Every non-obvious constant gets a name and a doc comment. `30.0` in a formula
  is a bug waiting for someone to guess its unit.
- Errors carry the reason a caller can act on (`feed_expired`, not `NotFound`).
- A crate that reads state never talks to the network.
- Commit messages explain the failure the change prevents.
