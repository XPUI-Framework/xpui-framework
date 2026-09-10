# Orientation

Read this before exploring any of the ten repositories. It is what the code
cannot tell you: what the ten are, how they sit on disk, what a clean machine
needs, how a change is proved, and the traps that have already cost a day.

## What XPUI is

A declarative UI framework for e-ink screens, in Rust. A screen is written once
against `xpui`'s traits and runs on any backend — a C++ firmware drawing
through FreeInkUI, a bare-metal Rust firmware drawing through
`embedded-graphics`, or a window on a laptop. The framework depends on nothing
and names no product, device or backend; everything else depends inward on it.

| Repository | What it is | Depends on |
|---|---|---|
| [`xpui-framework`](https://github.com/XPUI-Framework/xpui-framework) | the crate `xpui`: the framework | nothing |
| [`xpui-chrome`](https://github.com/XPUI-Framework/xpui-chrome) | themed components painted from drawing primitives alone | `xpui` |
| [`xpui-boards`](https://github.com/XPUI-Framework/xpui-boards) | seven devices as data, one crate per vendor over a `core` vocabulary | `xpui` |
| [`xpui-backends`](https://github.com/XPUI-Framework/xpui-backends) | the `embedded-graphics` and FreeInkUI backends, the host framebuffer, the ABI checker | `xpui`, `xpui-chrome` |
| [`xpui-simulator`](https://github.com/XPUI-Framework/xpui-simulator) | an `xpui` app in a desktop window | the four above |
| [`xpui-gallery`](https://github.com/XPUI-Framework/xpui-gallery) | the reference application, and the seven-board conformance suite it doubles as | the five above |
| [`xpui-rp2040`](https://github.com/XPUI-Framework/xpui-rp2040) | the gallery as firmware for the Badger 2040 and the Tufty 2040 | `xpui`, `xpui-boards`, `xpui-backends`, `xpui-gallery` |
| [`xpui-esp32`](https://github.com/XPUI-Framework/xpui-esp32) | the gallery as firmware for the Xteink X3 and the Seeed Sticky | the same four |
| [`xpui-cpp`](https://github.com/XPUI-Framework/xpui-cpp) | a C++ host for Rust screens over the C ABI, on a desktop and as an ESP32 image | `xpui`, `xpui-backends` |
| [`xpui-dev`](https://github.com/XPUI-Framework/xpui-dev) | the umbrella: the nine built as one from local paths, and the checks no single repository can make | the six library repositories |

Every arrow is a `Cargo.toml` dependency, and every one points inward. The
diagram is in each repository's README under **Where it sits**.

## Ten checkouts, side by side

Every cross-repository path in the organisation is relative and assumes the
ten are cloned beside each other under one directory. `xpui-dev`'s `[patch]`
table points at `../xpui`, `../xpui-chrome` and so on; `xpui-cpp` finds the
backends' C++ at `../xpui-backends`; the C++ stages look for the FreeInk SDK
beside the checkouts.

```bash
git clone https://github.com/XPUI-Framework/xpui-framework.git xpui
git clone https://github.com/XPUI-Framework/xpui-chrome.git
git clone https://github.com/XPUI-Framework/xpui-boards.git
git clone https://github.com/XPUI-Framework/xpui-backends.git
git clone https://github.com/XPUI-Framework/xpui-simulator.git
git clone https://github.com/XPUI-Framework/xpui-gallery.git
git clone https://github.com/XPUI-Framework/xpui-rp2040.git
git clone https://github.com/XPUI-Framework/xpui-esp32.git
git clone https://github.com/XPUI-Framework/xpui-cpp.git
git clone https://github.com/XPUI-Framework/xpui-dev.git
```

The first line names its directory on purpose: the repository is
`xpui-framework`, the crate it holds is `xpui`, and the checkout is named for
the crate. See [the repository name](#the-repository-name).

## A clean machine, in order

1. **Rust, through `rustup`.** Every repository carries a `rust-toolchain.toml`
   pinning the same channel, so the first `cargo` call installs the right
   compiler, `rustfmt` and `clippy`.
2. **The two bare-metal targets.** `riscv32imc-unknown-none-elf` and
   `thumbv6m-none-eabi`. A repository that lints for one names it in its
   toolchain file, so rustup installs it on the first call; where a gate finds
   one missing it either fails and says which command installs it, or prints
   `SKIPPED` and the target — never silence.
3. **SDL2** — `brew install sdl2` or `apt install libsdl2-dev`. The simulator
   links it.
4. **clang-format 21 or newer** — the two repositories with C++ format it,
   and an older clang-format ignores options it does not know rather than
   rejecting them.
5. **The FreeInk SDK's headers**, found beside the checkouts or named by
   `FREEINK_SDK_INCLUDE`. Without them the C++ stages skip and say so.
6. **Optional, for `all` runs and for flashing:** CMake for the C++ host's
   build and self-test; the Xtensa `esp` fork (`espup install`) for the
   ESP32-S3 image; `probe-rs` or `elf2uf2-rs` for an RP2040 board.

| Repository | Needs |
|---|---|
| `xpui`, `xpui-chrome`, `xpui-boards` | Rust, both targets |
| `xpui-gallery` | Rust, both targets, SDL2 |
| `xpui-backends` | Rust, both targets, clang-format; the SDK for the C++ stages |
| `xpui-simulator` | Rust, SDL2 |
| `xpui-rp2040` | Rust, `thumbv6m-none-eabi`; `probe-rs` or `elf2uf2-rs` to flash |
| `xpui-esp32` | Rust, `riscv32imc-unknown-none-elf`; the `esp` fork for the S3 image |
| `xpui-cpp` | Rust, clang-format; the SDK and SDL2 for the host's compile, CMake for `all` |
| `xpui-dev` | Rust, and the other nine checked out beside it |

## The gate

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

## Who verifies what

| | |
|---|---|
| **The assistant proves** | every gate, both bare-metal targets linking, each fix failing before it passes, and both agent reviews |
| **The author proves** | the simulator window as a person sees it, flashing a board, anything on hardware |

Do not open the simulator window and report what it looks like — ask. The
headless `--frames N` path exists so the loop can be *tested*; it is not a
substitute for eyes.

## How work is organised

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

### Git

Never stage and never commit without being asked, each time. Never push and
never open a pull request without being asked, each time. No assistant
self-attribution in a commit message. Never rewrite a commit that already
exists; a correction is a new commit. The index is the reviewer's queue —
leave new work unstaged.

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
  other in ways that look like flakiness.
- **A git dependency on `main` means a sibling's push changes your build.**
  Every `xpui*` dependency is `git = …, branch = "main"`, and `Cargo.lock` is
  committed. `cargo update` re-resolves every one of them to the sibling's
  current `main` and every third-party crate to its newest compatible version:
  run it on purpose, in its own commit, and let `xpui-dev` compare the lock
  files afterwards. To test a change across repositories before anything is
  pushed, build from `xpui-dev`, whose `[patch]` table points every git
  dependency at the checkout beside it.

## The repository name

The organisation's `xpui-framework` holds the crate `xpui`, and every guide in
the organisation checks it out as `xpui` to match. That is why the directory
beside your other checkouts is `xpui` while its remote says `xpui-framework`.
Said once, here; no other document has to.
