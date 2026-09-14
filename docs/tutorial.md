# Your first screen

From an empty file to a screen running in a window. No hardware, no C++, and
no framework knowledge assumed — only ordinary [Rust](https://rust-lang.org/).

Every Rust block below is compiled and run by
`cargo test -p xpui --doc --features testing` — the feature, because most of
them install the fake host this crate ships for exactly that. **One block is
not**, and it says so where it appears. If any of the rest stops being true,
the build fails rather than the page quietly lying to you.

The finished screen lives in [`tutorial/`](https://github.com/XPUI-Framework/xpui-gallery/tree/main/tutorial)
and is screenshot-tested; this walks to it one piece at a time.

## What you'll build

A sleep timer: how many minutes before the device sleeps, a preset picker, and
a switch for what to do when the cover closes.

```text
┌──────────────────────────────────┐
│  Sleep timer                     │   header — the backend draws it
├──────────────────────────────────┤
│  Sleep after             15 min  │
│   −  ██▏────────────────────  +  │   one focus stop, three touch targets
│  Presets            15 minutes   │   opens a dialog
│  Sleep when closed         Yes   │
│                                  │
├──────────────────────────────────┤
│  Back        Edit                │   button hints — also the backend's
└──────────────────────────────────┘
```

It covers everything you need: state, controls, a list row, a dialog that
captures input, and the two mistakes everybody makes once.

---

## 1. A screen is a struct

Two things: what it looks like, and how it changes. Nothing else.

```rust
use xpui::screen::Screen;
use xpui::{NavigationScreen, Text, View, vstack};

struct SleepTimer;

impl Screen for SleepTimer {
    /// Everything this screen can be told. Nothing yet.
    type Message = ();

    fn body(&self) -> impl View<Self::Message> {
        NavigationScreen::new(vstack![14; Text::new("Sleep after")]).title("Sleep timer")
    }

    fn update(&mut self, _message: Self::Message) {}
}
```

`body()` is a *description*, not a sequence of draw calls. It is a pure
function of `self`, rebuilt every frame, and thrown away afterwards. That is
what removes the class of bug where a screen paints something its state no
longer agrees with — there is no stored tree that can disagree.

`NavigationScreen` is the root for a page: the backend paints the header band
and the button hints, and your content is laid out between them. Use it and
your screen looks like every other screen on the device, including ones you did
not write.

> `vstack![14; …]` stacks its children with 14 pixels between them. The
> spacing comes first and is not optional — `vstack![Text::new("x")]` would try
> to read the text as a spacing and fail to compile.

## 2. Run it

Six lines put it in a window:

```text
use xpui_boards_xteink as xteink;
use xpui_simulator::{Panel, Simulator};

Simulator::new(Panel::of(xteink::X4))
    .title("sleep timer")
    .run(SleepTimer);
```

`xteink::X4` is a 480×800 reader, and `Panel::of` takes the whole board —
size, orientation and the body around it — so the window is the device rather
than a rectangle. `Simulator` opens it and drives the frame loop.

There is no default board. The simulator knows no devices, so you name the one
you are building for out of that vendor's crate: `xpui-boards-xteink`,
`xpui-boards-pimoroni`, `xpui-boards-seeed`. For a panel none of them
describes, `Board::custom(name, width, height, touch)` needs no vendor crate at
all — and the `touch` flag is not decoration: it decides whether taps are
reported and whether the chrome reserves a band to name keys.

> **Why this block is not a doctest.** `xpui` depends on nothing, and the
> dependency only ever points inward, so the crate that owns this tutorial
> cannot see a simulator to compile the lines above. Every other snippet here
> is compiled; this one is checked by eye against
> `tutorial/src/main.rs`, which the gate does compile. That file makes
> the same three calls and differs in three ways, none of them about the
> framework: it keeps the builder in a `let` so a `--frames` flag can add to
> it, titles the window `"xpui — tutorial"`, and passes `SleepTimer::new()`,
> because by step 8 the screen has state to initialise.

**The window lives in another repository.** This one is the framework, and the
framework has nothing to draw with — that is the dependency rule rather than an
omission. The finished screen, runnable, is
[`xpui-gallery`](https://github.com/XPUI-Framework/xpui-gallery)'s `tutorial` crate:

```bash
cd .. && git clone https://github.com/XPUI-Framework/xpui-gallery
cd xpui-gallery && cargo run -p xpui-tutorial
```

Beside this checkout, not inside it — every repository in the organisation
expects its siblings next to it, and the rest of this page assumes the same.
The window is drawn through [SDL2](https://www.libsdl.org/), which has to be installed first; it is one
step of [a clean machine, in order](orientation.md#a-clean-machine-in-order).

Arrows move focus, Enter confirms, Backspace goes back, Q or Escape quits.
Clicking is a tap and the scroll wheel is a swipe, so touch behaviour works too.

**You will not need the window for most of what follows.** `App` drives the
same frame loop, and the framework ships a host that draws into memory and
records every call — so a screen can be run and checked with no backend at all,
which is how the rest of this tutorial is proven and how you will test your own
screens. That is step 4.

## 3. State, and messages that change it

A screen never reads a touch, never asks where anything is, and never asks for
a repaint. It tags controls with **its own messages**, and the runtime delivers
them to `update` — the only place state changes.

```rust
use xpui::screen::Screen;
use xpui::{NavigationScreen, Stepper, Text, View, vstack};

#[derive(Clone, Copy)]
enum Message {
    /// An absolute value, from dragging the track.
    SetMinutes(i32),
    /// A nudge of -1 or +1, from the end glyphs or the Left/Right keys.
    StepMinutes(i32),
}

struct SleepTimer {
    minutes: i32,
}

impl Screen for SleepTimer {
    type Message = Message;

    fn body(&self) -> impl View<Self::Message> {
        NavigationScreen::new(vstack![14;
            Text::new("Sleep after"),
            Stepper::ranged(self.minutes, 120)
                .on_change(Message::SetMinutes)
                .on_step(Message::StepMinutes),
        ])
        .title("Sleep timer")
    }

    fn update(&mut self, message: Self::Message) {
        match message {
            Message::SetMinutes(value) => self.minutes = value.clamp(1, 120),
            Message::StepMinutes(delta) => self.minutes = (self.minutes + delta).clamp(1, 120),
        }
    }
}
```

`Message::SetMinutes` in `.on_change(Message::SetMinutes)` is not a call — it
is the enum variant's *constructor*, used as a function `fn(i32) -> Message`.
The runtime converts a touch position into a value and calls it, so slider
geometry never reaches your screen.

**Messages, not closures.** A closure mutating screen state from inside a tree
the screen also owns would need interior mutability, and a `RefCell` borrow
failure panics — which aborts on a device. A message is a plain value; nothing
borrows.

### The framework owns focus

Up and Down walk the interactive controls in tree order and never reach
`update`. Confirm fires the focused control's message — *the identical message
a tap produces*, so touch and buttons cannot drift apart. Left and Right nudge
whatever holds focus, which is how one pair of keys drives every adjustable
control on a screen.

A `Stepper` is **one focus stop but three touch targets**: the `-`, the track,
and the `+`. Up and Down move between settings rather than through glyphs.

## 4. Testing it, before it has ever been drawn

`update` is an ordinary method on an ordinary struct. Most of a screen's
behaviour needs no UI at all to test:

```rust
# use xpui::screen::Screen;
# use xpui::{NavigationScreen, Text, View, vstack};
# #[derive(Clone, Copy)]
# enum Message { SetMinutes(i32), StepMinutes(i32) }
# struct SleepTimer { minutes: i32 }
# impl Screen for SleepTimer {
#     type Message = Message;
#     fn body(&self) -> impl View<Self::Message> {
#         NavigationScreen::new(vstack![14; Text::new("Sleep after")]).title("Sleep timer")
#     }
#     fn update(&mut self, message: Self::Message) {
#         match message {
#             Message::SetMinutes(v) => self.minutes = v.clamp(1, 120),
#             Message::StepMinutes(d) => self.minutes = (self.minutes + d).clamp(1, 120),
#         }
#     }
# }
let mut screen = SleepTimer { minutes: 15 };

screen.update(Message::StepMinutes(1));
assert_eq!(screen.minutes, 16);

screen.update(Message::SetMinutes(9999));
assert_eq!(screen.minutes, 120, "clamped to the range");
```

For what the screen *draws*, install the fake host. It records every draw call,
so layout is checked on a laptop with no window and no hardware:

```rust
use xpui::screen::{Driver, Runtime, Screen};
use xpui::{NavigationScreen, Text, View, testing, vstack};

struct Hello;

impl Screen for Hello {
    type Message = ();
    fn body(&self) -> impl View<Self::Message> {
        NavigationScreen::new(vstack![14; Text::new("Sleep after")]).title("Sleep timer")
    }
    fn update(&mut self, _message: Self::Message) {}
}

testing::install();
testing::reset();

let mut runtime = Runtime::new(Hello);
runtime.render();

let drawn: Vec<String> = testing::drawn_text()
    .into_iter()
    .map(|(_, _, text, _, _)| text)
    .collect();
assert!(drawn.contains(&"Sleep after".to_string()));
assert_eq!(testing::drawn_headers().len(), 1, "the header was painted");
```

## 5. A row, and a dialog

A `List` is drawn by the backend's own theme, so a row here and a row in a
screen somebody else wrote are the same row. A `Modal` is a dialog that
**captures input**: while it is in the tree nothing behind it can be reached,
the side buttons walk its options, and focus opens on the value already chosen.

Your screen decides only whether the dialog is in `body()`.

```rust
use xpui::screen::Screen;
use xpui::{List, ListRow, Modal, NavigationScreen, Scrim, View};

const PRESETS: [&str; 4] = ["5 minutes", "15 minutes", "30 minutes", "1 hour"];

#[derive(Clone, Copy)]
enum Message {
    OpenPresets,
    ChoosePreset(usize),
    Dismiss,
}

struct SleepTimer {
    minutes: i32,
    picking: bool,
}

impl Screen for SleepTimer {
    type Message = Message;

    fn body(&self) -> impl View<Self::Message> {
        NavigationScreen::new(
            List::new().push(
                ListRow::new("Presets")
                    .value(PRESETS[1])
                    .on_tap(Message::OpenPresets),
            ),
        )
        .title("Sleep timer")
        .overlay_if(
            self.picking,
            Modal::picker("Sleep after", PRESETS)
                .selected(1)
                .on_select(Message::ChoosePreset)
                .on_dismiss(Message::Dismiss)
                .scrim(Scrim::Dim),
        )
    }

    fn update(&mut self, message: Self::Message) {
        match message {
            Message::OpenPresets => self.picking = true,
            Message::ChoosePreset(index) => {
                self.minutes = [5, 15, 30, 60][index.min(3)];
                self.picking = false;
            }
            Message::Dismiss => self.picking = false,
        }
    }
}
```

`.on_dismiss(Message::Dismiss)` is what Back and a tap outside the options send
while the dialog is open, so the screen never compares a touch against the
dialog's geometry. Without it, Back does nothing until an option is chosen: an
open dialog is never finished along with its screen.

`.scrim(Scrim::Dim)` darkens what is behind without erasing it — ink on one
checkerboard parity, so roughly half of what was there survives and the page
stays legible underneath. That is not the same as filling with grey, and on a
1-bit panel it is the only way to say "this is behind something".

## 6. Two mistakes everybody makes once

### `format!` inside `body()`

`body()` runs on **every paint and every frame that carries input** — several
times a second while a finger is down. A `format!` there allocates on each one
and pulls `core::fmt` into the binary, which on a device with a few hundred
kilobytes of RAM is a real cost, not a hypothetical.

Format when the value changes instead:

```rust
use xpui::screen::Screen;
use xpui::{NavigationScreen, Text, View, vstack};

#[derive(Clone, Copy)]
enum Message {
    SetMinutes(i32),
}

struct SleepTimer {
    minutes: i32,
    /// The minutes, already formatted. One allocation per change, rather
    /// than one per frame.
    label: String,
}

impl Screen for SleepTimer {
    type Message = Message;

    fn body(&self) -> impl View<Self::Message> {
        // `&self.label`, not `format!(..)`.
        NavigationScreen::new(vstack![14; Text::new(&self.label)]).title("Sleep timer")
    }

    fn update(&mut self, message: Self::Message) {
        let Message::SetMinutes(value) = message;
        self.minutes = value.clamp(1, 120);
        self.label = format!("{} min", self.minutes);
    }
}

let mut screen = SleepTimer {
    minutes: 15,
    label: String::from("15 min"),
};
screen.update(Message::SetMinutes(30));
assert_eq!(screen.label, "30 min");
```

### Flipping a toggle yourself

`Toggle` hands `update` the state it is **moving to**, not the state it is in.
So a screen never writes `!self.something` — do that and it flips twice per
press, which looks like the button not working.

```rust
use xpui::screen::Screen;
use xpui::{NavigationScreen, Toggle, View, vstack};

#[derive(Clone, Copy)]
enum Message {
    SetSleepOnClose(bool),
}

struct SleepTimer {
    sleep_on_close: bool,
}

impl Screen for SleepTimer {
    type Message = Message;

    fn body(&self) -> impl View<Self::Message> {
        NavigationScreen::new(vstack![14;
            Toggle::new("Sleep when closed", self.sleep_on_close, "Yes", "No")
                .on_change(Message::SetSleepOnClose),
        ])
        .title("Sleep timer")
    }

    fn update(&mut self, message: Self::Message) {
        // Right: take the state you were handed.
        let Message::SetSleepOnClose(next) = message;
        self.sleep_on_close = next;
    }
}

// Applying the same message twice is therefore idempotent. A screen that
// flipped instead would be back where it started.
let mut screen = SleepTimer {
    sleep_on_close: true,
};
screen.update(Message::SetSleepOnClose(false));
screen.update(Message::SetSleepOnClose(false));
assert!(!screen.sleep_on_close);
```

> A toggle is a *row*, not a switch graphic — it reads `Yes` or `No` on the
> right, so it looks the same standing alone or inside a list.

## 7. Giving it a title, and letting Back work

A host that keeps a stack of screens needs two things from yours: what it is
called, and permission to leave.

```rust
use xpui::screen::Screen;
use xpui::{NavigationScreen, Text, View, vstack};

struct SleepTimer;

impl Screen for SleepTimer {
    type Message = ();

    fn body(&self) -> impl View<Self::Message> {
        NavigationScreen::new(vstack![14; Text::new("Sleep after")]).title("Sleep timer")
    }

    fn update(&mut self, _message: Self::Message) {}

    /// `'static` because a host may hold it past this frame. A title computed
    /// at run time — a file name — goes to `NavigationScreen::title` instead,
    /// where no such constraint applies.
    fn title(&self) -> Option<&'static str> {
        Some("Sleep timer")
    }
}
```

Back is already wired: the runtime finishes the screen when nothing claims the
key, and `App` pops it. You only write `finish_screen()` yourself when
something *other* than Back should close the page — a Save button, say.

## 8. The finished screen

[`tutorial/src/lib.rs`](https://github.com/XPUI-Framework/xpui-gallery/blob/main/tutorial/src/lib.rs) is all
of the above assembled, and
[`tutorial/tests/screen.rs`](https://github.com/XPUI-Framework/xpui-gallery/blob/main/tutorial/tests/screen.rs)
is the test suite for it — behaviour, the runtime driving it, and screenshots.

```bash
cd ../xpui-gallery
cargo run -p xpui-tutorial                    # in a window
cargo test -p xpui-tutorial                   # eleven tests, no window
open tutorial/tests/screenshots/tutorial.png  # the frame it must paint
```

## Where next

- **[a-second-screen.md](a-second-screen.md)** — the four things after this
  one: a list, opening a screen and coming back, a page taller than the panel,
  and a widget of your own
- [reference.md](reference.md) — every widget, layout and modifier
- [architecture.md](architecture.md) — how a frame actually runs
- [host.md](host.md) — the contract a backend implements
- [orientation.md](orientation.md) — the ten repositories side by side, a
  clean machine, and the gate
- [`xpui-gallery`'s `gallery/`](https://github.com/XPUI-Framework/xpui-gallery/tree/main/gallery) — seven more
  screens, each demonstrating one part of the framework
