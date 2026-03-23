# Update Guide

## What Foro Is

`foro` is a plugin-driven code formatter daemon written in Rust. It is the end-user formatter runtime: it accepts formatting requests, loads formatter plugins, caches them, and executes them quickly across repeated runs.

In practice, `foro` itself is not "the formatter implementation" for every language. Instead, it is the host system that runs formatter plugins. Depending on the language and plugin, those plugins may be:

- native dynamic libraries
- WASM plugins
- wrappers around existing formatter engines

The `foro` source is in https://github.com/foro-fmt/foro .

## What This Repository Is

This repository, `foro-rustfmt`, is one specific `foro` plugin. Its job is to package `rustfmt` so that `foro` can use it as a formatter plugin for Rust files.

That means this repository is not only responsible for compiling against upstream `rustfmt`, but also for:

- matching `rustfmt`'s required Rust nightly/toolchain
- packaging the plugin in the `.dllpack` release format expected by `foro`
- producing platform-specific native artifacts that `foro` can download and run

Because `rustfmt` depends on `rustc` internals, this plugin is native-only here. It is not a WASM plugin.

## Why Updates In This Repo Are Often Multi-Layered

When upstream `rustfmt` or another formatter changes, breakage can happen in several layers:

- the formatter API or internal crate shape changes
- the Rust toolchain/nightly requirement changes
- native dependency packaging changes
- GitHub Actions runner or release workflow behavior changes
- helper tooling such as `dll-pack-builder` needs to learn a new platform-specific rule

So an update task here is usually a release-engineering task, not just a dependency bump.

## CI / Release Model In This Repo

This repo should be operated in two stages:

1. `Release Verify` workflow on a PR or manual dispatch
2. `Release` workflow only on a pushed version tag

`Release Verify` exists to test build and packaging behavior without creating a GitHub Release. `Release` is the final publish workflow and should run only after the PR is merged and a real version tag is pushed.

This document describes how an AI agent should update this repository when upstream formatter projects change, for example:

- `rustfmt`
- `ruff`
- `biome`
- any other formatter whose core binary/library/API changed

The goal is not just "make it build locally". The goal is:

1. update to the intended upstream version
2. keep packaging/release behavior correct on every supported platform
3. finish with a real successful GitHub Actions release run

## Scope

This repository currently packages a native `rustfmt` plugin and releases these targets:

- `x86_64-unknown-linux-gnu`
- `x86_64-apple-darwin`
- `aarch64-apple-darwin`
- `x86_64-pc-windows-msvc`

WASM is not supported here.

## Non-Negotiable Rules

- Do not stop at a local build if the release workflow is still broken.
- Do not "fix" upstream or packaging issues by vendoring random copies into this repo unless the user explicitly asked for that.
- If the failure is in a dependency tool such as `dll-pack-builder`, fix that tool in its own repo and pin a reviewed commit here.
- Prefer the smallest correct fix that preserves release behavior.
- When the user asks for a release/update, the task is complete only after GitHub Actions finishes successfully on the actual tagged release.

## Repositories Involved

- main repo: `foro-fmt/foro-rustfmt`
- packaging helper: `foro-fmt/dll-pack-builder`
- upstream formatter source: `rust-lang/rustfmt`

For other formatters, identify the equivalent upstream and packaging/helper repos first.

## Required Tooling

Assume these CLIs are available and use them:

- `git`
- `cargo`
- `rustup`
- `gh`
- `uv`
- `jq`
- `rg`

## High-Level Workflow

1. inspect current pinned upstream version and toolchain
2. inspect the latest upstream version/commit/toolchain
3. update local dependencies and version numbers
4. run local builds
5. push a branch and run `Release Verify` on a PR
6. if CI fails, identify whether the breakage is:
   - plugin code
   - toolchain/version pin
   - packaging helper
   - runner/workflow config
7. fix the correct layer and rerun PR verification
8. merge the PR after `Release Verify` is green
9. tag the merge commit and run the real `Release` workflow
10. repeat tag-level fixes only if publish-stage behavior still fails

## Step 1: Inspect Current State

Read at least:

- `Cargo.toml`
- `Cargo.lock`
- `rust-toolchain.toml`
- `.github/workflows/release-verify.yml`
- `.github/workflows/release.yml`
- `dll-pack-build-local.sh`
- `dll-pack-build-global.sh`
- `README.md`

Confirm:

- current crate version
- current upstream formatter dependency version and git source
- current Rust nightly/toolchain pin
- release target matrix
- any helper tools pinned in the workflow

## Step 2: Inspect Upstream

For `rustfmt`, check:

- upstream HEAD commit
- upstream `rust-toolchain`
- upstream crate/library version

Typical commands:

```bash
git ls-remote https://github.com/rust-lang/rustfmt HEAD
git clone --depth=1 https://github.com/rust-lang/rustfmt /tmp/rustfmt-upstream
sed -n '1,120p' /tmp/rustfmt-upstream/rust-toolchain
rg -n 'version = ' /tmp/rustfmt-upstream/Cargo.toml /tmp/rustfmt-upstream -g 'Cargo.toml'
```

Do not guess the latest version from memory.

## Step 3: Update This Repo

Update only what is required:

- formatter dependency version/commit in `Cargo.toml`
- Rust toolchain in `rust-toolchain.toml`
- crate version in `Cargo.toml`
- `Cargo.lock`

Versioning rule for this repo:

- **Upstream tracking release**: use the upstream `rustfmt` crate version as the plugin version tag verbatim (e.g., rustfmt `1.9.0` → tag `1.9.0`).
- **Plugin-only fix** (bug fix, foro ABI change, packaging fix — no upstream version change): append `-<n>` to the last upstream version, where n starts at 1 and increments (e.g., `1.9.0-1`, `1.9.0-2`).
- When a new upstream tracking release happens, n resets — the bare upstream version is used again (e.g., `1.10.0`, not `1.10.0-0`).
- Semver ordering is intentionally not preserved for the `-<n>` suffix. These tags are GitHub release identifiers, not semver coordinates.

## Step 4: Local Validation

Run the minimum local checks before tagging:

```bash
bash -n dll-pack-build-local.sh
cargo +<toolchain> build
cargo +<toolchain> build --profile super-release
```

If default rustup state is broken on the machine, use a temporary `RUSTUP_HOME`.

Example:

```bash
RUSTUP_HOME=/tmp/foro-rustfmt-rustup cargo +nightly-2026-02-19 build --profile super-release
```

Local success is necessary but not sufficient.

## Step 5: Open A PR And Run Release Verify

Do not start with a tag push. First validate the change through the PR workflow.

Typical flow:

```bash
git switch -c update/<short-topic>
git add <files>
git commit -m "<message>"
git push origin HEAD
gh pr create --fill
```

Then inspect the verification workflow:

```bash
gh run list --workflow "Release Verify" --limit 5
gh run view <run_id> --json status,conclusion,jobs,url
gh run view <run_id> --job <job_id> --log
```

The PR is not ready to merge until:

- `build-local-artifacts` succeeds on every platform
- `build-global-artifacts` succeeds

If `Release Verify` fails, fix the right layer and push more commits to the same branch until it is green.

## Step 6: Merge The PR

Assume the AI agent is allowed to merge.

After `Release Verify` succeeds, merge the PR and sync local `main`.

Typical flow:

```bash
gh pr merge --merge --delete-branch
git switch main
git pull --ff-only origin main
```

Do not tag a pre-merge branch tip. Tag the merged commit on `main`.

## Step 7: Write Release Notes, Tag, And Run The Real Release

Before pushing the tag, write end-user release notes into `RELEASE_NOTES.md`. CI will fail if this file is missing or still contains placeholder text.

### Write Release Notes

See the guidance below under "Release Notes" for what to write, format, and tone.

Once written, commit the file:

```bash
git add RELEASE_NOTES.md
git commit -m "docs: write release notes for <version>"
```

### Push The Tag

Once the verified change (including release notes) is on `main`, create and push the real version tag.

Typical flow:

```bash
git tag <version>
git push origin <version>
```

Then inspect the publish workflow:

```bash
gh run list --workflow "Release" --limit 5
gh run view <run_id> --json status,conclusion,jobs,url
gh run view <run_id> --job <job_id> --log
gh release view <tag>
```

The task is not complete until:

- `build-local-artifacts` succeeds on every platform
- `build-global-artifacts` succeeds
- `host` succeeds
- the GitHub release exists for the new tag

## Failure Triage

When CI fails, classify the failure before editing anything.

### A. Upstream formatter/library changed

Symptoms:

- compile errors in this crate
- changed API shape
- changed toolchain requirements

Action:

- update plugin code or toolchain pin
- keep changes minimal and specific to the upstream change

### B. Release workflow or runner changed

Symptoms:

- unsupported GitHub runner labels
- missing system tools in containers
- workflow syntax issues

Action:

- fix `.github/workflows/release-verify.yml` if the problem exists only in PR verification
- fix `.github/workflows/release.yml`
- avoid changing product code for CI-only failures

### C. Packaging helper broke

Symptoms:

- `cargo build` succeeds
- failure happens inside `dll-pack-builder`
- missing DLL discovery, dependency graph, or artifact merge issues

Action:

- inspect `dll-pack-builder` logs and source
- fix `foro-fmt/dll-pack-builder` in its own repo
- add a focused regression test there if feasible
- push the helper fix
- pin the helper commit in both release workflows

Do not hide this by vendoring a patched copy into `foro-rustfmt` unless explicitly requested.

### D. Platform-specific system library handling changed

Symptoms:

- only one OS fails
- typical on Windows with API set DLLs, or on macOS with shared cache/system dylibs

Action:

- determine whether the missing dependency is real or virtual/OS-provided
- fix dependency resolution in the packaging layer, not in the plugin crate, if the plugin binary itself builds successfully

## Packaging-Specific Notes For This Repo

### Windows

- The release helper uses `dll-pack-builder`.
- Windows imports like `api-ms-win-*` and `ext-ms-win-*` can be virtual OS-provided DLL names.
- If these fail only during packaging, treat it as a packaging/dependency-resolution problem first.

### macOS

- Intel and Apple Silicon are separate release targets.
- Keep both in the matrix.
- If one fails and the other passes, check runner labels and native dependency resolution separately.

### Linux

- The current workflow uses `buildpack-deps:focal`.
- If tools are missing in the container, fix the workflow/container setup rather than the plugin code.

### WASM

- Do not reintroduce WASM into the release matrix for this repo.
- `rustfmt` cannot be packaged as a WASM plugin here.

## How To Fix A Dependency Helper Correctly

If the problem is in `dll-pack-builder`:

1. clone `https://github.com/foro-fmt/dll-pack-builder`
2. make the smallest correct change there
3. add a regression test if possible
4. run the helper's own tests locally
5. commit and push to that repo
6. pin the exact helper commit in:
   - `foro-rustfmt/.github/workflows/release-verify.yml`
   - `foro-rustfmt/.github/workflows/release.yml`
7. rerun `Release Verify`
8. after merge, rerun the real tagged `Release`

Use an exact commit pin, not a floating branch reference.

Example install lines:

```yaml
run: uv tool install git+https://github.com/foro-fmt/dll-pack-builder@<commit>
run: python3 -m pip install git+https://github.com/foro-fmt/dll-pack-builder@<commit>
```

## Commit / Tag Strategy

Use small, descriptive commits.

Typical sequence:

1. formatter version bump
2. PR verification fix commit(s) if CI reveals packaging/workflow issues
3. merge after `Release Verify` is green
4. final tag for the actual release

Each failed tagged release may require a new patch version. Do not retag an existing released version.

## Release Notes

Write release notes in `RELEASE_NOTES.md` before pushing the release tag (see Step 7). CI will reject the release if this file is missing or still contains placeholder text.

### Audience

End users of `foro` — developers who run foro to format their code. They are not plugin maintainers. They do not care about CI, dll-pack, or packaging internals.

### What to write

- State clearly which upstream `rustfmt` version is bundled.
- Summarize 2–5 notable changes from upstream that affect formatting output or behavior.
- If this is a plugin-only fix (no upstream change), describe what was fixed in plain terms.
- Include a link to the upstream changelog or release for full details.
- Skip anything about CI, dll-pack, GitHub Actions, or build system changes.

### Format

```markdown
Bundles rustfmt **X.Y.Z** (nightly-YYYY-MM-DD).

**What's new:**
- ...
- ...

Full release notes: https://github.com/rust-lang/rustfmt/releases/tag/vX.Y.Z

---
*This release and summary were automatically generated by an AI agent.*
```

Or for an update with no user-facing changes:

```markdown
Bundles rustfmt **X.Y.Z** (nightly-YYYY-MM-DD). No formatting behavior changes.

---
*This release and summary were automatically generated by an AI agent.*
```

To find upstream changes, inspect:

```bash
# Check rustfmt's changelog or rust releases around the nightly date
gh release list --repo rust-lang/rust --limit 20
# Or check rustfmt repo directly
git log --oneline v<prev>..v<new> -- https://github.com/rust-lang/rustfmt
```

### Tone and length

- Friendly, direct, present tense.
- Aim for 5–15 lines total.
- No jargon about packaging, ABI, or dll-pack.

## What To Report Back

When done, report:

- final version/tag
- successful GitHub Actions run ID and URL
- release URL
- what changed in this repo
- whether any dependency helper repo was changed and to which commit it was pinned
- the generated release notes text
- any remaining warnings that did not block release

## Current Known Good Pattern

At the time this guide was written, updating `rustfmt` to `1.9.0` required:

- bumping this crate to `0.5.4`
- using `nightly-2026-02-19`
- validating first with `Release Verify`
- keeping release targets native-only
- fixing `dll-pack-builder` to ignore Windows API set virtual DLL imports
- pinning `foro-fmt/dll-pack-builder@1e90b63` in both release workflows

Do not assume these exact versions remain current. Re-verify upstream every time.
