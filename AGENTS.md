# `xpui`

## What this is, and what it may not become

A declarative UI framework for e-ink firmware, and the root of every
dependency edge in the organisation: a screen is a struct with `body()` and
`update()`, the framework measures, routes input and paints, and a backend
supplies the painting through five small traits. It has no dependencies and
builds `no_std` for bare metal.

**It may not name a product, a device, a backend or a drawing library.** Not
in code and not in a comment under `src/`; the gate greps for eleven such
words there and fails on any of them. A test fixture may name a device. To
reach something a backend has, add a trait method; that is what the seam is
for. Screens belong in an application, devices in `xpui-boards`, painting in
`xpui-backends`.

## The gate

```bash
./build-and-test.sh          # everything below
./build-and-test.sh fix      # the same, formatting in place first
```

```text
format · the framework names no product · file sizes · crates are tested · READMEs warn · prose is compiled · documented paths resolve · rustdoc links resolve · documented commands resolve · lint · tests · doctests · README sections · AGENTS.md · published crates deny missing_docs · comment blocks · comment narration
```

There is no `all` mode; this list is the whole of it, and a last stage,
`the gate is documented`, compares it to what ran. Run it before saying a
change is done, and read the real exit code.

## What only this repository checks

- **`the framework names no product`** — the genericity grep, carried by
  `xpui` alone. Every other repository is allowed to say a device's name.
- **Two bare-metal clippy runs** under `lint`, on `riscv32imc` and
  `thumbv6m`. The host build never parses code behind
  `cfg(target_os = "none")`, so these are the only checks that reach the
  `no_std` paths before a firmware build does.
- **The genericity of the README** is not checked; the grep reads `src/`.

## Style that bites here

- **`no_std`.** `alloc::` types explicitly; `std` is used only under
  `src/testing/`. The two bare-metal clippy runs are what catch a `std` use
  in `src/`; a doc example builds for the host and is not checked there.
- **No per-frame allocation.** `body()` runs on every paint and every frame
  carrying input. Build strings in `update`, not while describing the screen;
  keep `format!` off those paths.
- **No atomic compare-and-swap.** Neither target has it: load and store only,
  never `swap`, `fetch_or` or `compare_exchange`.
- **Never estimate text metrics.** Measure through the host's font. An
  estimate drifts from what is painted.
- **No pixel offsets.** Ask the theme through `ThemeMetric` and
  `Theme::content_area()`.
- **`unsafe` lives in three places** — the installed-host globals, the
  single-threaded cells in `AppShell`, and the `testing` module — and stays
  there. Every block carries a `// Safety:` line naming the invariant.
- **Saturating arithmetic at layout boundaries.** A view may report an
  unbounded height.
- **Every `pub` item is documented.** `#![deny(missing_docs)]` is on, and the
  `testing` module is public.
- **A file under `src/` is at most 400 lines.** Split by job, never by type.

## Where the documentation lives, and what proves each piece

| Document | Proven by |
|---|---|
| [`README.md`](README.md) | its `rust` fences are doctests, mounted by `src/lib.rs` |
| [`docs/reference.md`](docs/reference.md) | doctests, mounted by `src/lib.rs` |
| [`docs/tutorial.md`](docs/tutorial.md), [`docs/a-second-screen.md`](docs/a-second-screen.md) | doctests, mounted by `src/lib.rs` |
| [`docs/architecture.md`](docs/architecture.md), [`docs/host.md`](docs/host.md) | doctests, mounted by `src/lib.rs` |
| [`docs/design.md`](docs/design.md) | mounted by `src/lib.rs`, but it carries no `rust` fence; its paths are checked like any page's |
| [`docs/writing-a-backend.md`](docs/writing-a-backend.md), [`docs/writing-a-widget.md`](docs/writing-a-widget.md), [`docs/testing.md`](docs/testing.md) | doctests, mounted by `src/lib.rs` |
| [`docs/orientation.md`](docs/orientation.md) | every relative path resolves (`documented paths resolve`); its organisation links are `xpui-dev`'s to check |
| [`docs/contributing.md`](docs/contributing.md) | every path and command it gives resolves; the umbrella command is `xpui-dev`'s |
| `AGENTS.md` | the stage list above is compared to what the gate runs |
| every `///` and `//!` | `rustdoc links resolve`, and the two comment checks |

A `rust` fence in a page nothing mounts fails `prose is compiled`. A path or
command in any page that does not resolve fails its check.

## Git

Never stage, never commit, never push without being asked, each time. The
index is the reviewer's queue; leave new work unstaged. No self-attribution
in a commit message. Never rewrite a commit that exists; a correction is a new
commit. The rules that apply to all ten repositories, and the five review
steps, are in [`docs/orientation.md`](docs/orientation.md); how a change is
built and reviewed here is in [`docs/contributing.md`](docs/contributing.md).
