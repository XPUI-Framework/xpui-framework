[![CI](https://github.com/XPUI-Framework/xpui-framework/actions/workflows/ci.yml/badge.svg)](https://github.com/XPUI-Framework/xpui-framework/actions/workflows/ci.yml) [![MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="assets/logo-black.png">
  <img src="assets/logo-white.png" alt="XPUI" width="64" height="64">
</picture>

# XPUI Framework

> [!WARNING]
> Under heavy development. Not production-ready. The API can break without
> notice. Use at your own risk.

Write an e-Paper screen once, and run it wherever an e-Paper panel is. A screen is
a plain Rust struct that says what it looks like and how it changes, and the
same code runs on a bare-metal microcontroller board, hosted inside a C++ e-Paper
firmware, and in a desktop simulator on your laptop. You find out a screen is
right before any device is on the desk, and the framework asks nothing of your
firmware: no dependencies, no build script, `no_std`.

- **One screen, every panel.** Describe a screen with stacks, lists, sliders,
  toggles and dialogs; a [backend](https://github.com/XPUI-Framework/xpui-backends)
  paints it through whatever the device has. The
  [gallery](https://github.com/XPUI-Framework/xpui-gallery) runs the same
  screens on seven boards, and the
  [simulator](https://github.com/XPUI-Framework/xpui-simulator) puts them in a
  window, inside the device's own body.
- **Tested on a laptop, not on a device.** The `testing` feature is a fake host
  that records every draw call, so `update` and `body` are ordinary code in an
  ordinary `cargo test`. Every gallery screen is also compared, pixel for pixel,
  against golden images of each board's panel.
- **Buttons and touch, handled for you.** A screen tags its controls with its own
  messages; focus, key repeat, value editing and dialogs that capture input are
  the framework's job, so a button-only reader and a touch panel behave alike.
- **Built for e-Paper's limits.** 1-bit panels, a few hundred KB of RAM, no GPU,
  and a refresh that takes a second: nothing allocates per frame, and a screen
  repaints only when something changed.

Under the hood, you describe what a screen looks like and `xpui` measures it,
routes input to it and paints it through five small traits a backend
implements. The framework names no product and no drawing library, which is
what lets the same screen move between devices, and every test run on a laptop.

Every document in this repository is listed in [docs/README.md](docs/README.md).

## Using it

```toml
[dependencies]
xpui = { git = "https://github.com/XPUI-Framework/xpui-framework", branch = "main" }

[dev-dependencies]
# The fake host: a backend that records every draw call instead of painting,
# so a screen can be tested with no window and no device.
xpui = { git = "https://github.com/XPUI-Framework/xpui-framework", branch = "main", features = ["testing"] }
```

Nothing is on [crates.io](https://crates.io/) yet, which is why the dependency above is a `git` URL. A
screen is a struct that says what it looks like and how it changes:

```rust
use xpui::{vstack, NavigationScreen, Screen, Stepper, Text, View};

struct Brightness {
    level: i32,
    /// Built when the value changes rather than when the screen is described.
    label: String,
}

/// Everything this screen can be told.
#[derive(Clone, Copy)]
enum Msg {
    Set(i32),   // an absolute value, from dragging the track
    Step(i32),  // a nudge of -1 or +1, from the end glyphs
}

impl Screen for Brightness {
    type Message = Msg;

    fn title(&self) -> Option<&'static str> {
        Some("Brightness")
    }

    fn body(&self) -> impl View<Msg> {
        NavigationScreen::new(vstack![12;
            Text::new(&self.label),
            Stepper::new(self.level)
                .on_change(Msg::Set)
                .on_step(Msg::Step),
        ])
    }

    fn update(&mut self, message: Msg) {
        self.level = match message {
            Msg::Set(level) => level.clamp(0, 100),
            Msg::Step(delta) => (self.level + delta).clamp(0, 100),
        };
        // Formatted here, not in `body`. `body` runs on every paint and every
        // frame carrying input; `update` runs when the value actually changes.
        self.label = format!("Brightness  {}%", self.level);
    }
}
```

That is a working screen: a header, a label and a stepper that answers a
finger on the track, the `−` and `+` glyphs, and the hardware buttons, with no
coordinate, hit-test or redraw call written. It cannot reach a panel yet —
`xpui` has no idea one exists — so a
[backend](https://github.com/XPUI-Framework/xpui-backends) connects it to
something that can paint. `alloc` is required; `std` is used only by the
`testing` module.

## Checking it

```bash
./build-and-test.sh
```

The checks themselves are in [`xtask/`](xtask/) — this repository's own list,
in [Rust](https://rust-lang.org/), holding nothing it does not run. `./build-and-test.sh fix` formats
in place first. Format, [clippy](https://github.com/rust-lang/rust-clippy) on the host and two bare-metal architectures,
the tests, every documented snippet compiled, every public item documented,
and every link and command in the prose resolved. How a change is reviewed
is in [docs/contributing.md](docs/contributing.md).

## Where it sits

Every arrow is a dependency in a `Cargo.toml`, and they all point inward
toward `xpui`, which depends on nothing at all. That is the rule the
organisation is arranged around: a backend can be written without the framework
knowing it exists, and a firmware reaches whatever it needs directly rather
than through whoever happens to sit above it.

```mermaid
flowchart TD
  xpui["xpui<br/>the framework"]
  chrome["xpui-chrome<br/>components"]
  boards["xpui-boards<br/>seven devices"]
  backends["xpui-backends<br/>two backends"]
  simulator["xpui-simulator<br/>a window"]
  gallery["xpui-gallery<br/>the app"]
  rp2040["xpui-rp2040<br/>firmware"]
  esp32["xpui-esp32<br/>firmware"]
  cpp["xpui-cpp<br/>a C++ host"]
  dev["xpui-dev<br/>the umbrella"]
  chrome --> xpui
  boards --> xpui
  backends --> xpui
  backends --> chrome
  simulator --> xpui
  simulator --> chrome
  simulator --> boards
  simulator --> backends
  gallery --> xpui
  gallery --> chrome
  gallery --> boards
  gallery --> backends
  gallery --> simulator
  rp2040 --> xpui
  rp2040 --> boards
  rp2040 --> backends
  rp2040 --> gallery
  esp32 --> xpui
  esp32 --> boards
  esp32 --> backends
  esp32 --> gallery
  cpp --> xpui
  cpp --> backends
  dev --> xpui
  dev --> chrome
  dev --> boards
  dev --> backends
  dev --> simulator
  dev --> gallery
  style xpui stroke-width:3px
```

## License

MIT — see [LICENSE](LICENSE). Copyright (c) 2026 Thiago Holanda.
