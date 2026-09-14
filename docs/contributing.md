# Contributing to `xpui`

## Building it

`rust-toolchain.toml` pins the toolchain and the two bare-metal targets, so
`cargo build` on a fresh clone installs what it needs. There are no other
dependencies: no C++, no SDL, no device.

```bash
cargo test --features testing        # the suite, on a laptop
./build-and-test.sh                  # everything CI checks
```

## The gate

A change is not finished until `./build-and-test.sh` passes. It is the same
command CI runs, so a green run locally means what a green tick means there.
The checks are listed in [`AGENTS.md`](../AGENTS.md) and implemented in
[`xtask/`](../xtask/); `./build-and-test.sh fix` formats in place first.

Two checks bite here more than anywhere else:

- **The framework names no product.** Not a device, a backend or a drawing
  library, anywhere under `src/`. If you need something a backend has, add a
  trait method.
- **Every public item is documented**, the `testing` module included.

### The gate in every repository

One command in every repository:

```bash
./build-and-test.sh          # everything CI checks
./build-and-test.sh fix      # the same, formatting in place first
```

The three repositories that produce an image — `xpui-rp2040`, `xpui-esp32`,
`xpui-cpp` — also take `all`, which adds the link, the build or the self-test
a quick run should not pay for.

The script builds and runs `xtask/`, a Rust program with no dependencies.
Its `main.rs` is that repository's own list of checks and is meant to differ;
the ten modules under it — reading a markdown fence, a manifest, a path, a
comment — are byte-identical across the nine, and `xpui-dev` fails if any two
copies differ. A check that is written and never listed is a dead function the
build refuses, so the list in `main.rs` is the truth about what a repository
checks.

`xpui-dev` runs what no single repository can: that the shared files agree,
that every lock file resolves the crates whose types cross a boundary the same
way, that every `github.com/XPUI-Framework/…` link names a file on that
repository's pushed `main`, and — by default — every sibling's own gate. Its
[README](https://github.com/XPUI-Framework/xpui-dev/blob/main/README.md) says
what `cross` skips and why.

## The review

**Work is specified before it is written.** A spec says why, what exists
already, what to do, the acceptance criteria, and the command that proves it
done; it is finished when somebody who was not in the conversation can execute
it. The code comes last.

A change goes through five steps, in order, and none is skipped:

1. The gate passes, with the real exit code read.
2. The [code-reviewer](../.claude/agents/code-reviewer.md) agent reviews it
   — every finding resolved, not noted.
3. The [docs-reviewer](../.claude/agents/docs-reviewer.md) agent reviews the
   prose, last of the automated checks: it runs every command a document
   gives, resolves every snippet against the API that exists, and judges every
   comment.
4. The author reviews the code and tests it in the simulator or on a board.
5. They say commit. Not before.

Both agents are the same two files in every repository, compared by
`xpui-dev`. Each repository's `AGENTS.md`, loaded into every session, says
what that repository is, what only it checks, and the style that bites there;
this one is [`AGENTS.md`](../AGENTS.md).

### Who verifies what

| | |
|---|---|
| **The assistant proves** | every gate, both bare-metal targets linking, each fix failing before it passes, and both agent reviews |
| **The author proves** | the simulator window as a person sees it, flashing a board, anything on hardware |

Do not open the simulator window and report what it looks like — ask. The
headless `--frames N` path exists so the loop can be *tested*; it is not a
substitute for eyes.

## A test that cannot fail

A test that cannot fail is worse than no test. Before adding one, break the
code on purpose and confirm the test notices.

Eight that were shipped here, all written in good faith and all passing:

- **A test that never installed the backend**, so it exercised a different host
  and asserted `0 == 0`.
- **A window-geometry test that asserted its own arithmetic** — it recomputed
  the number it was checking.
- **A scaling test that still passed with half its assertion deleted.**
- **A menu test that let two rows open the wrong screen.**
- **`scrolling_sections`, whose content fitted on one screen**, so it asserted
  a scroll that never happened.
- **A dialog paint-versus-hit-test check too loose to notice a six-pixel
  drift** — a row is 40px, so "the text is somewhere inside the rect" tolerated
  a fault that makes tapping row 3 select row 2.
- **A headless `--frames N` run treated as an input test.** It proves the loop
  starts, ticks and exits; it drives no input and asserts no pixels. The arrow
  keys doing nothing survived 249 tests that way.
- **A test driving `Runtime` directly and asserting `focused_index()`**, which
  moves correctly even when nothing is ever drawn.

The last is the general lesson: **prefer an assertion that pins a relationship
over one that pins a number.** "The label sits at the same offset within every
row" catches drift that "the label is inside its row" cannot.

## Commits

The subject says what was done — imperative, under fifty characters, one
concern. The body says what changed and why, in under about ten lines,
carrying the fact that is not in the diff. Nothing about how the bug was
found. No self-attribution.

### Git

Never stage and never commit without being asked, each time. Never push and
never open a pull request without being asked, each time. No assistant
self-attribution in a commit message. Never rewrite a commit that already
exists; a correction is a new commit. The index is the reviewer's queue —
leave new work unstaged.

## Working across the repositories

`xpui` depends on nothing, but nine repositories depend on it. A change to a
public item reaches all of them through a `git` dependency on `main`, so
before pushing one, run the umbrella:

```bash
for d in ../xpui*/; do git -C "$d" fetch --quiet --all; done
cd ../xpui-dev && ./build-and-test.sh cross
```

It builds every crate from the sibling checkouts on disk and says which one
broke. `cross` is that repository's gate, not this one's — run it from there,
not here. The fetch first, because its link check resolves every
`github.com/XPUI-Framework/…` URL against each sibling's `origin/main`, and a
stale remote is a stale answer. [`orientation.md`](orientation.md) describes the layout it expects.

## Traps that have cost time

- **The framework may not name a product, a device or a backend.** `xpui`'s
  gate greps its `src/` for eleven forbidden words. Device names live in
  `xpui-boards`, one crate per vendor; if you want to reach for a backend from
  inside the framework, add a trait method — that is what the seam is for.
- **Neither bare-metal target has atomic compare-and-swap.** Load and store
  only — never `swap`, `fetch_or` or `compare_exchange`. Cortex-M0+ has none,
  and neither does `riscv32imc`, which has no A extension. The gates lint both
  so a crate cannot be checked on one and not the other.
- **`Canvas::draw_text` takes a top-left origin, not a baseline.** Most
  drawing libraries take a baseline, so it is the obligation a new backend is
  most likely to get backwards — every glyph one line too high while
  everything compiles and every test passes. The method's own documentation
  says how to convert. And **a font id is derived from the face's bytes**: a
  glyph cache is keyed on it, so a stable id over changed bytes serves
  yesterday's type with every test green; `TextMetrics::font` says so.
- **Screenshots are compared pixel for pixel.** A first run writes the golden
  *and fails*, so nobody commits a picture they never looked at. Re-bless with
  `UPDATE_SNAPSHOTS=1`, then open them.
- **The `Host` is process-wide.** Tests that install one hold the shared
  lock `xpui::testing::Ui` keeps for its whole life, or they corrupt each
  other in ways that look like flakiness. Every test file here takes the same
  `static SERIAL: Mutex<()>` first.
- **A git dependency on `main` means a sibling's push changes your build.**
  Every `xpui*` dependency is `git = …, branch = "main"`, and `Cargo.lock` is
  committed. `cargo update` re-resolves every one of them to the sibling's
  current `main` and every third-party crate to its newest compatible version:
  run it on purpose, in its own commit, and let `xpui-dev` compare the lock
  files afterwards. To test a change across repositories before anything is
  pushed, build from `xpui-dev`, whose `[patch]` table points every git
  dependency at the checkout beside it.

## Where to read first

[`orientation.md`](orientation.md) is where the ten repositories, how they
sit on disk and a clean machine are explained. [`writing-a-widget.md`](writing-a-widget.md) is
the guide for adding to the framework.
