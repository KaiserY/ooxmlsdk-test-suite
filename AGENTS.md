# Repository Guidelines

This repository is a standalone test workspace for `ooxmlsdk` and related
projects. It stores test code plus imported corpora whose licenses may differ
from the test code license.

Read first:

1. `README.md` for repository purpose.
2. `corpus/README.md` for imported corpus inventory and license pointers.
3. `corpus-manifest.toml` for workspace-level corpus index.
4. `docs/round-trip/` for round-trip coverage status and run history.

## Project Shape

- `crates/ooxmlsdk-corpus-test-support/`: shared helpers for corpus tests.
- `crates/ooxmlsdk-roundtrip-tests/`: generated per-file round-trip tests.
- `corpus/`: imported Office documents and corpus-local manifests.
- `licenses/`: third-party corpus license and notice files.
- `docs/round-trip/`: round-trip status summaries.

This repository depends on `../ooxmlsdk/crates/ooxmlsdk` by path during local
development. Keep that path dependency unless the user explicitly asks to switch
to a git or registry dependency.

## Corpus Rules

- Do not treat corpus files as project-owned source code.
- Do not mix files from different upstream projects into the same corpus
  directory.
- Each corpus must have a corpus-local manifest, such as
  `corpus/Open-XML-SDK/manifest.toml`.
- `corpus-manifest.toml` is only the workspace-level corpus index.
- Corpus manifests should describe metadata and exceptions, not list every
  generated test.
- Default behavior should come from file scanning; add manifest entries only for
  `invalid`, `open_only`, `known_failure`, or other non-default expectations.
- Do not add local checkout paths, machine-specific paths, or temporary import
  notes to corpus metadata.
- Preserve upstream license and notice files under `licenses/<corpus>/`.

## Cargo And Locks

- Keep `Cargo.lock` checked in. This is a test workspace, and reproducible test
  dependency resolution matters.
- Do not delete or regenerate `Cargo.lock` casually. If dependency changes alter
  it, mention that in the final summary.
- Run Cargo commands from the repository root.
- Use the default `target/` directory. Do not set `CARGO_TARGET_DIR`.
- Cargo commands must run sequentially. Never start a second Cargo command while
  another Cargo command is still running.
- If Cargo waits on a target lock, do not probe processes or start competing
  commands. Wait for Cargo to finish.
- After starting a Cargo command, let it run to completion and report the final
  result.

## Commands

- `cargo fmt --all`: format all Rust code.
- `cargo check --workspace --tests`: compile workspace test targets without
  running tests.
- `cargo clippy --workspace --tests -- -D warnings`: run Clippy for workspace
  test targets.
- `cargo test --workspace`: run default tests. This does not run ignored corpus
  tests.
- `cargo test -p ooxmlsdk-corpus-test-support --test float_rules_sync -- --ignored`:
  compare checked-in schema float rules with `ooxmlsdk/data`.
- `cargo test -p ooxmlsdk-roundtrip-tests --test apache_poi_roundtrip -- --ignored`:
  run the Apache POI round-trip corpus lane.
- `cargo test -p ooxmlsdk-roundtrip-tests --test open_xml_sdk_roundtrip -- --ignored`:
  run the Open-XML-SDK round-trip corpus lane.

Round-trip corpus tests are generated per supported Office package file and are
ignored by default. Run ignored corpus tests explicitly, and expect long runtime
and large output when failures are present.

## Round-Trip Standard

The round-trip helper should follow the high-standard `doc_samples` model from
`ooxmlsdk`: open, save, reopen, package part graph comparison, zip entry
comparison, and canonical XML equivalence.

Schema float lexical normalization uses checked-in static rules from
`crates/ooxmlsdk-corpus-test-support/data/schema-float-rules.json`. Do not make
normal build or test paths read `../ooxmlsdk/data`; use the ignored
`float_rules_sync` test when checking rule drift against the SDK schema data.

If this workspace intentionally differs from the upstream `doc_samples`
comparison, document the difference in `docs/round-trip/`.

## Clippy

- Run Clippy before finalizing Rust changes:
  `cargo clippy --workspace --tests -- -D warnings`.
- Fix Clippy findings directly. Do not silence findings with `#[allow(...)]`,
  `#![allow(...)]`, or similar hacks unless the user explicitly asks for a
  specific exception and the exception is documented with the reason.

## Documentation

- Update `corpus/README.md` when adding or removing a corpus.
- Update the round-trip index in `README.md` only.
- Update the corpus-specific round-trip page, such as
  `docs/round-trip/Open-XML-SDK.md`, after a full corpus run.
- Do not record local checkout paths or machine-specific paths in docs.

## Licensing

- The root test code license uses `LICENSE-APACHE` and `LICENSE-MIT`.
- Corpus files keep their upstream licenses.
- Do not collapse corpus licenses into the root license.
- Add license/notice files under `licenses/<corpus>/` when importing a corpus.

## Git Guidance

- Do not run `git add`, `git commit`, `git commit --amend`, or other index or
  history update commands. The user creates commits.
- Do not revert unrelated user changes.
- Before finalizing substantial work, inspect `git status --short` and report
  verification status.
