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

## The review

Five steps, in order, none skipped:

1. The gate passes, with the real exit code read.
2. The [code-reviewer](../.claude/agents/code-reviewer.md) agent reviews the
   change — every finding resolved, not noted.
3. The [docs-reviewer](../.claude/agents/docs-reviewer.md) agent reviews the
   prose, last: it runs every command a document gives and resolves every
   snippet against the API.
4. The author reviews the code and tests it in the simulator or on a board.
5. They say commit.

A test that cannot fail is worse than no test. Before adding one, break the
code on purpose and confirm the test notices. Prefer an assertion that pins a
relationship over one that pins a number.

## Commits

The subject says what was done — imperative, under fifty characters, one
concern. The body says what changed and why, in under about ten lines,
carrying the fact that is not in the diff. Nothing about how the bug was
found. No self-attribution.

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

## Where to read first

[`orientation.md`](orientation.md) is where the ten repositories, the gate
and the traps are explained. [`writing-a-widget.md`](writing-a-widget.md) is
the guide for adding to the framework.
