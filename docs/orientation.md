# Orientation

Read this before exploring any of the ten repositories. It is what the code
cannot tell you: what the ten are, how they sit on disk, and what a clean
machine needs. How a change is proved, and the traps that have already cost a
day, are in [contributing.md](contributing.md).

## What XPUI is

A declarative UI framework for e-ink screens, in [Rust](https://rust-lang.org/). A screen is written once
against `xpui`'s traits and runs on any backend — a C++ firmware drawing
through [FreeInkUI](https://github.com/Free-Ink/freeink-sdk/tree/main/libs/ui/FreeInkUI), a bare-metal Rust firmware drawing through
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
| [`xpui-rp2040`](https://github.com/XPUI-Framework/xpui-rp2040) | the gallery as firmware for the [Badger 2040](https://shop.pimoroni.com/products/badger-2040) and the [Tufty 2040](https://shop.pimoroni.com/products/tufty-2040) | `xpui`, `xpui-boards`, `xpui-backends`, `xpui-gallery` |
| [`xpui-esp32`](https://github.com/XPUI-Framework/xpui-esp32) | the gallery as firmware for the [Xteink X3](https://www.xteink.com/products/xteink-x3) and the [Seeed Sticky](https://www.seeedstudio.com/reTerminal-Sticky-p-6861.html) | the same four |
| [`xpui-cpp`](https://github.com/XPUI-Framework/xpui-cpp) | a C++ host for Rust screens over the C ABI, on a desktop and as an ESP32 image | `xpui`, `xpui-backends` |
| [`xpui-dev`](https://github.com/XPUI-Framework/xpui-dev) | the umbrella: the nine built as one from local paths, and the checks no single repository can make | the six library repositories |

Every arrow is a `Cargo.toml` dependency, and every one points inward. The
diagram is in each repository's README under **Where it sits**.

## Ten checkouts, side by side

Every cross-repository path in the organisation is relative and assumes the
ten are cloned beside each other under one directory. `xpui-dev`'s `[patch]`
table points at `../xpui`, `../xpui-chrome` and so on; `xpui-cpp` finds the
backends' C++ at `../xpui-backends`; the C++ stages look for the [FreeInk SDK](https://github.com/Free-Ink/freeink-sdk)
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

## A clean machine, in order

1. **Rust, through `rustup`.** Every repository carries a `rust-toolchain.toml`
   pinning the same channel, so the first `cargo` call installs the right
   compiler, `rustfmt` and `clippy`.
2. **The two bare-metal targets.** `riscv32imc-unknown-none-elf` and
   `thumbv6m-none-eabi`. A repository that lints for one names it in its
   toolchain file, so [rustup](https://rustup.rs/) installs it on the first call; where a gate finds
   one missing it either fails and says which command installs it, or prints
   `SKIPPED` and the target — never silence.
3. **[SDL2](https://www.libsdl.org/)** — `brew install sdl2` or `sudo apt install libsdl2-dev`. The
   simulator links it.
4. **[clang-format](https://clang.llvm.org/docs/ClangFormat.html) 21 or newer** — the two repositories with C++ format it,
   and an older clang-format ignores options it does not know rather than
   rejecting them.
5. **The FreeInk SDK's headers**, found beside the checkouts or named by
   `FREEINK_SDK_INCLUDE`. Without them the C++ stages skip and say so.
6. **Optional, for `all` runs and for flashing:** [CMake](https://cmake.org/) for the C++ host's
   build and self-test; the Xtensa `esp` fork (`espup install`) for the
   [ESP32-S3](https://www.espressif.com/en/products/socs/esp32-s3) image; `probe-rs` or `elf2uf2-rs` for an [RP2040](https://www.raspberrypi.com/products/rp2040/) board.

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
