# The host contract

`xpui` draws nothing by itself. It describes what it needs through five traits in
[`src/host/`](../src/host/); you implement them and install the result once.

The crates under [`crates/backend/`](../../backend/) implement these against
real drawing substrates. This page describes what each trait owes `xpui` —
worth reading if you are changing one of those, or writing a new one.

## The five traits a backend implements

| Trait | You provide | File |
|---|---|---|
| `Canvas` | Fill and stroke rectangles, draw text, lines, bitmaps, icons | [canvas.rs](../src/host/canvas.rs) |
| `TextMetrics` | Width and line height for a string in a font | [metrics.rs](../src/host/metrics.rs) |
| `Chrome` | Header, button hints, list rows, dialogs, and "repaint please" — your theme | [chrome.rs](../src/host/chrome.rs) |
| `InputSource` | Buttons, taps, drags, gestures for one frame | [input.rs](../src/host/input.rs) |
| `Clock` | Milliseconds since boot | [clock.rs](../src/host/clock.rs) |

Implement all five on one type and it satisfies `Host` automatically:

```rust
pub struct MyBackend;

impl Canvas for MyBackend { /* ... */ }
impl TextMetrics for MyBackend { /* ... */ }
impl Chrome for MyBackend { /* ... */ }
impl InputSource for MyBackend { /* ... */ }
impl Clock for MyBackend { /* ... */ }

static BACKEND: MyBackend = MyBackend;

// `install` is unsafe: it must run before the first measure, render or
// interactions pass, and never alongside one.
unsafe { xpui::host::install(&BACKEND) };
```

A compile-time assertion is worth adding so a missing trait is caught at the
definition rather than at the install site:

```rust
const _: fn() = || {
    fn assert_host<T: xpui::host::Host>() {}
    assert_host::<MyBackend>();
};
```

## The sixth trait, which is not a backend's job

`Navigator` — [navigator.rs](../src/host/navigator.rs) — answers "what is this
screen called" and "go back". Those depend on who owns the screen stack, which
is a different question from what paints the pixels, so it is installed
separately:

```rust
unsafe { xpui::host::install_navigator(&SHELL) };
```

A backend crate does **not** implement it. Either the application does, or
[`App`](../src/app.rs) does it for you — `App::new` installs itself. A C++
firmware whose own activity manager owns the stack implements it over the FFI.

Note what deliberately stayed on `Chrome`: `request_update`. A repaint is a
display concern, anything that can paint can ask to paint again, and the
framework calls it on every dispatch — so a host that forgot to install a
navigator gets a dead Back button, not a screen that never refreshes.

## Things that are easy to get wrong

**Install before anything runs.** That is the safety contract on `install`, not
a style note. Rendering and input run on different tasks on the device, so the
lifecycle installs from both entry points rather than assuming which wakes first.

**`Chrome` is your theme, and `xpui` has no opinion about it.** The framework
never decides what a list row looks like; it asks, and you answer. A backend
sitting on a component library — FreeInkUI, say — answers by calling that
library, so a screen written here and a native one are the same pixels.

That leaves a backend sitting on a *drawing* library with eight components to
paint and no toolkit to paint them with. It does not have to write them: the
`chrome` backend paints all eight from `Canvas` and `TextMetrics` alone, so
such a backend implements `Canvas`, `TextMetrics`, `InputSource` and `Clock`,
and takes `Chrome` from there. Shared implementation, not a default — you still
choose it explicitly.

**Ask for metrics honestly.** `TextMetrics` must reflect the font you will
actually paint with. If it does not, everything measures correctly and draws
wrongly.

**Icons are roles, not files.** `Canvas::draw_icon` receives an opaque number
meaning "the thing you use for *sun*", and you choose the asset. That keeps
asset names out of the framework.

**Report zero rather than guessing.** If a font or icon is missing from a build,
return `0` for its size. `xpui` then draws nothing, rather than painting garbage
at an arbitrary size.

## Testing without hardware

`xpui` ships a fake host behind the `testing` feature:

```toml
[dev-dependencies]
xpui = { workspace = true, features = ["testing"] }
```

```rust
xpui::testing::install();
```

It reports fixed screen and theme dimensions and records everything drawn, so
you can assert on layout and touch behaviour in an ordinary `cargo test`. The
framework's own 56 tests use nothing else.

## Worked examples

The crates under [`crates/backend/`](../../backend/) are the real ones, and
they are deliberately different shapes:

| Backend | Satisfies | How |
|---|---|---|
| `embedded_graphics` | `Canvas`, `TextMetrics` | directly, over a `DrawTarget`; takes `Chrome` from `chrome` |
| `fui` | all five | over an FFI boundary, into C++ FreeInkUI |
| `chrome` | `Chrome` | from `Canvas` primitives, for backends that have no toolkit |

`xpui`'s own `unsafe` is confined to the four lines that read the installed
host. A backend crossing an FFI boundary holds the rest.
