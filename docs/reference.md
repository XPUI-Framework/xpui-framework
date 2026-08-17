# Reference

The whole of the public API, by area. [architecture.md](architecture.md)
explains how a frame runs and [tutorial.md](tutorial.md) builds one screen from
nothing; this is what you reach for once you know the shape and want to know
what exists.

- [A screen](#a-screen)
- [Layout](#layout)
- [Widgets](#widgets)
- [Interaction](#interaction)
- [Components](#components)
- [Screen roots](#screen-roots)
- [Navigation](#navigation)
- [Fonts](#fonts)
- [The host façades](#the-host-façades)
- [Geometry](#geometry)

Every ```rust block below is compiled and run by `cargo test -p xpui --features
testing --doc`. That is the point of writing them this way: a snippet that stops
matching the API fails CI rather than quietly teaching the wrong thing. It is
also why most of them install the fake host first — widgets resolve fonts and
theme metrics through the host *in their constructors*, so there has to be one.
Those lines are hidden where they would only clutter the prose.

## A screen

A screen is a struct with a message type, a `body()` describing what it looks
like, and an `update()` that is the only place its state changes.

```rust
use xpui::{NavigationScreen, Screen, Stepper, Text, View, vstack};

/// Everything this screen can be told.
#[derive(Clone, Copy)]
enum Msg {
    Set(i32),   // an absolute value, from dragging the track
    Step(i32),  // a nudge of -1 or +1, from the end glyphs
}

struct Brightness {
    level: i32,
}

impl Screen for Brightness {
    type Message = Msg;

    fn body(&self) -> impl View<Msg> {
        NavigationScreen::new(vstack![12;
            Text::new("Brightness"),
            Stepper::new(self.level)
                .on_change(Msg::Set)
                .on_step(Msg::Step),
        ])
    }

    fn update(&mut self, message: Msg) {
        match message {
            Msg::Set(level) => self.level = level.clamp(0, 100),
            Msg::Step(delta) => self.level = (self.level + delta).clamp(0, 100),
        }
    }

    fn title(&self) -> Option<&'static str> {
        Some("Brightness")
    }
}

// `update` is an ordinary method, so a screen is testable with no UI at all.
let mut screen = Brightness { level: 40 };
screen.update(Msg::Step(-1));
assert_eq!(screen.level, 39);
```

`Message` must be `Clone`: a widget carries values of it and the runtime hands
them back, sometimes more than once in a frame.

| Method | Default | What it is for |
|---|---|---|
| `body()` | — | Describes the screen. A pure function of `self`, called once per paint and once per frame carrying input. |
| `update(msg)` | — | Applies a message. The runtime repaints afterwards, so no screen calls `request_update` itself. |
| `title()` | `None` | The screen's name, for a host that keeps a stack of them. `&'static str`, because a host may hold it past this frame. |
| `on_key(button)` | `None` | A key, offered **before** the runtime applies its own meaning. Return a message to consume it. |
| `on_swipe(dir)` | `None` | A swipe, likewise offered first. |
| `on_background_tap(point)` | `None` | A touch no control claimed. |
| `tick()` | — | A frame happened. Called once per frame, **before any input is considered and on frames where none arrived**. |
| `is_overlay()` | `false` | Whether this screen paints over what is already on the panel instead of clearing. |
| `on_enter()` / `on_exit()` | — | Lifecycle, for work that should not happen in `body()`. |
| `handle_home_gesture()` | `false` | Return `true` to consume the system home gesture. |

`tick` is where anything depending on *time passing* lives — a countdown, a
timeout, an auto-refresh, a value that settles a moment after it stops
changing. Every other method fires because something arrived; this one fires
because nothing did.

It takes no argument on purpose. A screen that wants the clock asks `millis()`
and compares; passing the time in would make every screen that ignores it carry
a parameter, and would fix the units at the trait.

```rust
use xpui::{Screen, Text, View};

const IDLE_MS: u32 = 30_000;

struct Reader {
    last_touched: u32,
    dimmed: bool,
}

impl Screen for Reader {
    type Message = ();

    fn tick(&mut self) {
        // Nothing ever arrives to say a screen has been left alone.
        if !self.dimmed && xpui::host::millis().wrapping_sub(self.last_touched) > IDLE_MS {
            self.dimmed = true;
            xpui::host::request_update();
        }
    }

    fn body(&self) -> impl View<()> {
        Text::new("...")
    }

    fn update(&mut self, _message: ()) {}
}
```

A screen that changes something in `tick` asks for a repaint the same way
anything else does — the runtime does not assume a tick changed anything, or
every screen would repaint at frame rate. Only the screen on top ticks; one
further down the stack is not running.

`title()` returning `None` under a [`NavigationScreen`](#screen-roots) that has
no title of its own draws an **empty header band**, because that is literally
what was asked for. Give one in either place. An overlay panel is the case where
`None` is right.

Because `body()` is rebuilt every frame, keep expensive work in `update()`,
where it happens once per event. Formatting is the usual offender: a `format!`
in `body()` allocates several times a second and drags `core::fmt` into the
binary, whereas formatting in `update()` costs one allocation per actual change.

## Layout

Containers take views **by value**. You never write `Box::new` to build a tree.

```rust
use xpui::{Spacer, Text, VStack};

# xpui::testing::install();
let show_detail = true;
let optional: Option<Text> = None;
let rows = ["Wi-Fi", "Bluetooth"].into_iter().map(Text::new);

let tree: VStack<()> = VStack::new(20)   // vertical, 20px between children
    .push(Text::new("Title"))
    .push_if(show_detail, Text::new("Detail"))
    .push_some(optional)
    .extend(rows)
    .push(Spacer::new());
```

Or the macro form, which expands to exactly those `push` calls and suits a tree
whose shape is fixed:

```rust
use xpui::{HStack, Spacer, Text, VStack, hstack, vstack};

# xpui::testing::install();
let page: VStack<()> = vstack![20;
    Text::new("Title").bold(),
    Spacer::new(),
    Text::new("Footer"),
];

let row: HStack<()> = hstack![8; Text::new("Battery"), Spacer::new(), Text::new("72%")];
```

| Type | Purpose |
|---|---|
| `VStack` / `HStack` | Stack children along one axis. Two passes: fixed children measure first, flexible ones divide what is left. |
| `Spacer` | Absorbs leftover space, pushing what follows to the far end. |
| `Padding::all(child, 12)` | Insets a child. Also `symmetric(child, h, v)` and `new(child, Insets)`. |
| `ScrollView::new(child)` | A window onto content taller than itself. |

Stacks align children at the leading cross edge. A row mixing a 32px icon with a
line of text wants `.align(Alignment::Center)`, or the text hangs off the top.

Any view can be modified in place, chainably. `.frame(w, h)` fixes a size and
centres the view in it — either axis may be `0` to stay natural — and
`.flexible()` makes a view absorb leftover space the way a `Spacer` does:

```rust
use xpui::{Modifiers, Slider, Text, hstack};

#[derive(Clone, Copy)]
enum Msg { Set(i32), Down, Up }

# xpui::testing::install();
let row = hstack![8;
    // `Text` is a `View<M>` for every `M`, so a modifier applied to a bare one
    // has to name the message type. `Slider` below already knows its own.
    Modifiers::<Msg>::frame(Text::new("−"), 44, 44).on_tap(Msg::Down),
    Slider::new(30, 100).on_change(Msg::Set).flexible(),
    Modifiers::<Msg>::frame(Text::new("+"), 44, 44).on_tap(Msg::Up),
];
```

Order matters in that chain: framing *before* `on_tap` makes the frame itself
the touch target, which is what a glyph a few pixels wide wants.

To collect views of different types, box them — `Box<dyn View<M>>` is itself a
`View`, and `.boxed()` is the short way to make one:

```rust
use xpui::{Divider, Text, VStack, View, ViewExt};

# xpui::testing::install();
let rows: Vec<Box<dyn View<()>>> = vec![
    Text::new("Wi-Fi").boxed(),
    Divider::new().boxed(),
];
let tree = VStack::new(4).extend(rows);
```

### Measuring

`measure` records the size a view wants within what it is offered; `size`
reports what it decided. Both are generic over the message type, so a widget
that is a `View<M>` for *every* `M` — `Text`, `Divider`, `Image` — has to be
told which one you mean:

```rust
use xpui::{Size, Text, View};

xpui::testing::install();
let mut text = Text::new("Battery");
View::<()>::measure(&mut text, Size::new(480, 800));
assert!(View::<()>::size(&text).width > 0);
```

Inside a tree this never comes up: the stack knows its own message type and
passes it down.

### Scrolling

`ScrollView` shows as much of its content as fits and lets the rest be scrolled
to — by swipe on a touch panel, by Up/Down on a button one. The runtime keeps
whatever holds focus on screen, so a screen never tracks an offset itself.

```rust
use xpui::{List, ListRow, ScrollView, Size, View};

xpui::testing::install();
let rows = (0..40).map(|index| ListRow::new("Setting").on_tap(index));
let mut scroll = ScrollView::new(List::new().extend(rows));

View::<i32>::measure(&mut scroll, Size::new(480, 400));
assert_eq!(
    View::<i32>::size(&scroll).height,
    400,
    "the view occupies the band it was given, however tall its content is"
);
```

Content is measured against [`UNBOUNDED`] rather than against the band —
asking it to fit is what squeezed rows out before there was a scroll view to
hold them — and drawn under a clip, so overflow is discarded rather than
painted over the header and the button hints.

`UNBOUNDED` is a large number, not `i32::MAX`, and the difference matters: a
few views echo the height they were offered straight back — `Spacer`, `Modal`,
a nested `ScrollView` — and a stack then adds that to its siblings. With
`i32::MAX` that overflows, which is a panic in debug and a *negative* height in
release. Anything scrolled out of sight
keeps its focus stop, which is how it can be reached at all, but stops accepting
touches aimed at whatever now occupies that part of the panel.

**One per screen.** The scroll offset lives in the runtime beside focus, so a
second scroll view would share the first one's position.

## Widgets

| Widget | Notes |
|---|---|
| `Text::new(s)` | One line. `.font(f)`, `.bold()`, `.italic()`. Measured with the host's real font metrics. |
| `Divider::new()` | A one-pixel rule across the available width. |
| `Section::new(title, content)` | A titled group of anything, headed by the theme's own sub-header. |
| `List` / `ListRow` | A themed, selectable list. `ListRow::new(t).subtitle(s).value(v).on_tap(msg)`. |
| `ListRow::toggle(t, on, on_label, off_label)` | A boolean setting as a row. |
| `ProgressBar::new(current, total)` | Or `ProgressBar::percent(72)`. `.height(px)` overrides the theme. |
| `Slider::new(value, max)` | A bare track. `.on_change(Msg::V)` reports drags and taps. Also `Slider::percent(72)`. |
| `Stepper::new(value)` | `−` / track / `+` as one control, over `0..=100`; `Stepper::ranged(v, max)` for anything else. `.on_change`, `.on_step`. |
| `Toggle::new(label, on, on_label, off_label)` | A boolean row. `.on_change(Msg::V)` receives the **next** state. |
| `Modal::picker(title, options)` | A centred option dialog. `.selected(i)`, `.on_select(Msg::V)`, `.scrim(Scrim::Dim)`. Also `Modal::confirm` and `Modal::new`. |
| `Image::new(data, w, h)` | A 1-bpp bitmap you supply, borrowed rather than copied. |
| `Icon::new(glyph)` | A host asset. `.filled(bool)`, `.size(px)`. |
| `IconToggle::new(glyph, on)` | An icon that shows and flips a boolean. `.on_change(Msg::V)` receives the **next** state. |

**A toggle is a row, not a switch.** `Toggle` renders through the theme's list,
so it looks identical standing alone or sitting inside one, and it hands
`update` the state it is *moving to* — which is what stops a screen ever writing
`!self.something`, the mistake that makes a toggle flip twice per press.

```rust
use xpui::{List, ListRow, Toggle};

#[derive(Clone, Copy)]
enum Msg { Hyphenation(bool), Justify(bool) }

# xpui::testing::install();
let hyphenation = false;
let justify = true;

// Standing alone: the framework works out the state being moved to.
let single = Toggle::new("Hyphenation", hyphenation, "On", "Off")
    .on_change(Msg::Hyphenation);   // Msg::Hyphenation(true) when currently off

// Several settings in one themed list are rows, and say it themselves.
let list: List<Msg> = List::new()
    .push(ListRow::toggle("Hyphenation", hyphenation, "On", "Off").on_tap(Msg::Hyphenation(!hyphenation)))
    .push(ListRow::toggle("Justify", justify, "On", "Off").on_tap(Msg::Justify(!justify)));
```

(`Toggle::into_row()` hands back the bare row for placing by hand, but only
before `.on_change` — which consumes it into a list of its own.)

**Interactive widgets are stateless.** `Slider`, `Stepper` and `Modal` draw the
value they are given and never change it; the screen owns the state and adjusts
it in `update`. None of them hit-tests either: they declare their regions and
the runtime converts a touch into a value or an index, so no screen sees
geometry.

A dialog **captures input**. While one is in the tree nothing behind it can be
reached, focus opens on the value already chosen, and the side buttons walk its
options rather than the list underneath. Dismissing returns focus to the row
that opened it. A screen decides only whether the dialog is in `body()`:

```rust
use xpui::{List, ListRow, Modal, NavigationScreen, Screen, Scrim, View};

const FONTS: [&str; 3] = ["Serif", "Sans", "Mono"];

#[derive(Clone, Copy)]
enum Msg { Open, Chose(usize), Dismiss }

struct Typeface {
    chosen: usize,
    picking: bool,
}

impl Screen for Typeface {
    type Message = Msg;

    fn body(&self) -> impl View<Msg> {
        NavigationScreen::new(
            List::new().push(
                ListRow::new("Typeface")
                    .value(FONTS[self.chosen])
                    .on_tap(Msg::Open),
            ),
        )
        .overlay_if(
            self.picking,
            Modal::picker("Typeface", FONTS)
                .selected(self.chosen)
                .on_select(Msg::Chose)
                .scrim(Scrim::Dim),
        )
    }

    fn update(&mut self, message: Msg) {
        match message {
            Msg::Open => self.picking = true,
            Msg::Chose(index) => {
                self.chosen = index.min(FONTS.len() - 1);
                self.picking = false;
            }
            Msg::Dismiss => self.picking = false,
        }
    }

    /// A touch no control claimed — here, the dimmed area around the dialog.
    fn on_background_tap(&self, _at: xpui::Point) -> Option<Msg> {
        self.picking.then_some(Msg::Dismiss)
    }

    fn title(&self) -> Option<&'static str> {
        Some("Typeface")
    }
}
```

`Scrim::Dim` darkens what is behind by adding ink on one checkerboard parity
rather than filling, so about half the pixels behind survive and the region
reads as grey while staying legible. Plain dithering cannot do this — it clears
the interior before applying its pattern, destroying the very content an overlay
exists to preserve.

### Icons

An icon is chosen by what it *means*, not by filename: the framework passes an
opaque `IconRef` and the host decides which asset that is. A backend publishes
its own roles and converts:

```rust
use xpui::{Icon, IconRef, IconToggle};

/// What a backend would publish. The framework only ever sees the number.
#[derive(Copy, Clone)]
enum Glyph {
    Sun = 0,
    Folder = 1,
}

impl From<Glyph> for IconRef {
    fn from(glyph: Glyph) -> IconRef {
        IconRef::new(glyph as u16)
    }
}

#[derive(Clone, Copy)]
enum Msg { Light(bool) }

# xpui::testing::install();
let folder = Icon::new(Glyph::Folder).size(24);
let light = IconToggle::new(Glyph::Sun, true).on_change(Msg::Light);
```

Solid means on and outline means off, which is what `.filled(bool)` selects. An
icon the host ships nothing for measures zero and draws nothing, rather than
painting something arbitrary at a guessed size.

## Interaction

A screen never computes a rect, never hit-tests and never polls a button. It
tags controls with **its own messages** and the runtime delivers them.

```rust
use xpui::{IconRef, IconToggle, Screen, Stepper, View, vstack};
# #[derive(Copy, Clone)]
# enum Glyph { Sun = 0 }
# impl From<Glyph> for IconRef {
#     fn from(glyph: Glyph) -> IconRef { IconRef::new(glyph as u16) }
# }

#[derive(Clone, Copy)]
enum Msg {
    Brightness(i32),   // a new absolute value
    Step(i32),         // a relative nudge
    Light(bool),       // the state the icon is moving to
}

struct Panel {
    brightness: i32,
    on: bool,
}

impl Screen for Panel {
    type Message = Msg;

    fn body(&self) -> impl View<Msg> {
        vstack![12;
            IconToggle::new(Glyph::Sun, self.on).on_change(Msg::Light),
            Stepper::new(self.brightness)
                .on_change(Msg::Brightness)   // dragged or tapped on the track
                .on_step(Msg::Step),          // -1 / +1 from the end glyphs
        ]
    }

    fn update(&mut self, message: Msg) {
        match message {
            Msg::Brightness(value) => self.brightness = value.clamp(0, 100),
            Msg::Step(delta) => self.brightness = (self.brightness + delta).clamp(0, 100),
            Msg::Light(next) => self.on = next,
        }
    }
}
```

`Msg::Brightness` in `.on_change(Msg::Brightness)` is the variant *constructor*,
`fn(i32) -> Msg`. The framework converts the touch position into a value and
calls it, so slider geometry never reaches a screen.

**Messages, not closures.** A closure mutating screen state from inside a tree
the screen also owns needs interior mutability, and a failed `RefCell` borrow
panics — which aborts on a device with no unwinder. A message is a plain value;
nothing borrows. It is also why those constructors are taken as `fn` pointers
rather than `impl Fn`: a function pointer costs nothing to store and cannot
close over anything.

| Modifier | Effect |
|---|---|
| `.on_tap(msg)` | Touch, and Confirm when focused |
| `.on_touch(msg)` | Touch only — stays out of the focus order |
| `.on_long_press(msg)` | Adds the held-press threshold to an ordinary tap |
| `.flexible()` | Absorbs leftover space, like a `Spacer` |
| `.frame(w, h)` | Fixes the size and centres the view in it |
| `.map(Msg::Variant)` | Folds a component's messages into this screen's |

**Focus is the framework's.** Up/Down move it through the interactive controls
in tree order, wrapping at both ends, and never reach `update`. Confirm fires
the focused control's message — **the identical message a tap produces** — so
touch and buttons cannot drift apart. A `List` highlights whichever row holds
focus with no screen code at all.

**Left/Right nudge whatever holds focus.** A control that opts into adjustment
(`Stepper` does) receives `-1` / `+1` there, so one pair of keys drives every
adjustable setting on a screen rather than the screen wiring keys to one of
them. Confirm on such a control does nothing: there is no absolute value to
commit.

**A composite is one focus stop.** A `Stepper` offers three touch targets — `−`,
the track, `+` — but a single stop for buttons, so Up/Down move between settings
rather than through glyphs. Use `.on_touch(msg)` rather than `.on_tap(msg)` for
anything that should take a finger without joining the focus order.

**Held frames only reach drag controls.** Each interaction declares an
`InputMask`, and only `DRAG` — sliders — sees frames while the finger is down.
Everything else acts once, on release. Without that, a finger resting on a
button re-fires it every tick. The masks are listed in
[writing-a-widget.md](writing-a-widget.md), which is where they matter.

**A control smaller than a fingertip is widened automatically**, to the theme's
minimum touch target, centred on what was drawn. The message still reports the
control, not the widened area.

**A vertical swipe moves focus**, so a touch panel and a button one navigate the
same list the same way. Which way it walks is the host's preference
(`InputSource::swipe_moves_selection`): by default the swipe drags the
*content*, so swiping up moves focus down.

**Auto-repeat is free.** A key fires on press, then repeats after 500ms at 500ms
intervals, whether the runtime claimed it or a screen did.

A screen that wants a key or a gesture for itself claims it, and is asked first:

```rust
use xpui::{Button, NavigationScreen, Screen, SwipeDir, Text, View, finish_screen, vstack};

#[derive(Clone, Copy)]
enum Msg { NextPage, PreviousPage, Close }

struct Reader {
    page: usize,
}

impl Screen for Reader {
    type Message = Msg;

    fn body(&self) -> impl View<Msg> {
        NavigationScreen::new(vstack![0; Text::new("…")])
    }

    fn update(&mut self, message: Msg) {
        match message {
            Msg::NextPage => self.page += 1,
            Msg::PreviousPage => self.page = self.page.saturating_sub(1),
            Msg::Close => finish_screen(),
        }
    }

    /// Consulted before the runtime gives the key its own meaning, so a reader
    /// pages with Up/Down instead of moving focus.
    fn on_key(&self, key: Button) -> Option<Msg> {
        match key {
            Button::Down | Button::PageForward => Some(Msg::NextPage),
            Button::Up | Button::PageBack => Some(Msg::PreviousPage),
            _ => None,
        }
    }

    fn on_swipe(&self, direction: SwipeDir) -> Option<Msg> {
        match direction {
            SwipeDir::Up => Some(Msg::NextPage),
            SwipeDir::Down => Some(Msg::PreviousPage),
            _ => None,
        }
    }

    fn title(&self) -> Option<&'static str> {
        Some("Reader")
    }
}
```

Back is the one key worth knowing the runtime keeps: unclaimed, it finishes the
screen.

## Components

A component owns its state, declares its own message type, and the parent folds
it in with `.map()`:

```rust
use xpui::{Modifiers, Screen, Text, View, ViewExt, vstack};

#[derive(Clone, Copy)]
enum UnitMsg { Cycle }

struct Units {
    binary: bool,
}

impl Units {
    /// Tapping the figure cycles the units — a message this component owns and
    /// the screen around it never sees.
    fn view(&self, bytes: i32) -> impl View<UnitMsg> + use<> {
        let text = if self.binary {
            format!("{} KiB", bytes / 1024)
        } else {
            format!("{} kB", bytes / 1000)
        };
        Text::new(text).on_tap(UnitMsg::Cycle)
    }

    fn update(&mut self, message: UnitMsg) {
        match message {
            UnitMsg::Cycle => self.binary = !self.binary,
        }
    }
}

#[derive(Clone, Copy)]
enum Msg { Units(UnitMsg) }

struct Storage {
    units: Units,
    free: i32,
}

impl Screen for Storage {
    type Message = Msg;

    fn body(&self) -> impl View<Msg> {
        vstack![8;
            Text::new("Free space"),
            self.units.view(self.free).map(Msg::Units),
        ]
    }

    fn update(&mut self, message: Msg) {
        match message {
            Msg::Units(inner) => self.units.update(inner),
        }
    }
}
```

The component's controls keep their place in the parent's focus order, exactly
where they appear in the tree.

The `+ use<>` on the return type is worth understanding rather than copying. In
edition 2024 an `impl Trait` return captures every lifetime in scope, including
the `&self` the method was called on — and a stack requires `'static` children.
`use<>` says the returned view captures none of them, which is true here because
`Text` owns its string. A component that genuinely borrows cannot be pushed into
a stack, and this is where the compiler says so.

A plain `fn thing(..) -> impl View<M> + use<M>` is a first-class component too;
there is no registration and no trait to implement.

## Screen roots

`NavigationScreen::new(content)` is the root for an ordinary page. The theme
draws the title band and the button hints, and content is laid out between them.

```rust
use xpui::{Hint, NavigationScreen, Text, vstack};

# xpui::testing::install();
let screen: NavigationScreen<()> = NavigationScreen::new(vstack![20;
    Text::new("Firmware"),
    Text::new("1.4.2").bold(),
])
.title("About")   // else the screen's own title
.hints(Hint::Standard, Hint::text("Save"), Hint::None, Hint::None);
```

`Hint::Standard` uses the host's own translated label for that slot and is the
default in all four; `Hint::None` blanks one. The slots are given by meaning —
back, confirm, previous, next — and the host reorders them to match the user's
button layout. Blank three of them and a device with no touch panel shows no
sign that its other buttons do anything, which is why the default is not "Back
only".

`.overlay(view)` and `.overlay_if(cond, view)` put a view over the content — a
dialog, typically. It is measured against the whole panel rather than the
content band, drawn last, and sits outside any `ScrollView`, so it is neither
clipped nor scrolled away. It is also *declared* last, which is what lets a
capturing dialog discard everything the content declared.

`OverlayPanel::new(content)` is the root for a drop-down over whatever is
already on screen. It sizes itself to its content, paints only its own band and
rules its bottom edge; the screen underneath survives untouched. Pair it with
`is_overlay()` so the runtime does not clear first.

```rust
use xpui::{OverlayPanel, Screen, Scrim, Slider, Text, View, finish_screen, vstack};

#[derive(Clone, Copy)]
enum Msg { Brightness(i32), Dismiss }

struct Frontlight {
    brightness: i32,
}

impl Screen for Frontlight {
    type Message = Msg;

    fn body(&self) -> impl View<Msg> {
        OverlayPanel::new(vstack![12;
            Text::new("Frontlight"),
            Slider::new(self.brightness, 100).on_change(Msg::Brightness),
        ])
        .scrim(Scrim::Dim)            // push the screen below into the background
        .on_scrim_tap(Msg::Dismiss)   // a touch down there closes the panel
    }

    fn update(&mut self, message: Msg) {
        match message {
            Msg::Brightness(value) => self.brightness = value.clamp(0, 100),
            Msg::Dismiss => finish_screen(),
        }
    }

    /// The panel paints over the screen it dropped from, so the runtime must
    /// not clear first — there would be nothing left to overlay.
    fn is_overlay(&self) -> bool {
        true
    }
}
```

`.on_scrim_tap(msg)` is an ordinary interaction, so the screen never compares a
touch against the panel's own height. Both the dimming and the dismiss target
come from one rect, which is what makes the region that looks tappable the
region that is.

## Navigation

Who owns the screen stack is a different question from what paints the pixels,
so it is a separate trait — `Navigator` — installed separately from the host.

| The stack lives in | `Navigator` is |
|---|---|
| a C++ firmware with its own activity manager | a call across the FFI |
| a Rust binary, a simulator, an example | `App`, which installs itself |
| nowhere — a single screen with no Back | not installed, and Back does nothing |

Two free functions reach it, and a screen calls them from `update`:

```rust
use xpui::{NavigationScreen, Screen, Text, View, finish_screen, present, vstack};

struct Details;

impl Screen for Details {
    type Message = ();
    fn body(&self) -> impl View<()> {
        NavigationScreen::new(vstack![0; Text::new("Details")])
    }
    fn update(&mut self, _message: ()) {}
    fn title(&self) -> Option<&'static str> {
        Some("Details")
    }
}

#[derive(Clone, Copy)]
enum Msg { Open, Done }

struct Summary;

impl Screen for Summary {
    type Message = Msg;

    fn body(&self) -> impl View<Msg> {
        NavigationScreen::new(vstack![0; Text::new("Summary")])
    }

    fn update(&mut self, message: Msg) {
        match message {
            // `present` returns false when the host's navigation is not the
            // framework's to drive; the screen is dropped, having gone nowhere.
            Msg::Open => {
                present(Details);
            }
            Msg::Done => finish_screen(),
        }
    }

    fn title(&self) -> Option<&'static str> {
        Some("Summary")
    }
}
```

Both are called from inside a screen's own frame, so an implementation that owns
the stack records the request and acts on it once the frame is over. Popping
there would free the screen currently running.

`App` is that implementation, for a host with no navigation of its own:

```rust
use xpui::{App, NavigationScreen, Screen, Text, View, vstack};

struct Home;

impl Screen for Home {
    type Message = ();
    fn body(&self) -> impl View<()> {
        NavigationScreen::new(vstack![0; Text::new("Home")])
    }
    fn update(&mut self, _message: ()) {}
    fn title(&self) -> Option<&'static str> {
        Some("Home")
    }
}

xpui::testing::install();
let mut app = App::new(Home);   // installs itself as the navigator
app.tick();                     // one frame of input, then any navigation it asked for
app.render_if_dirty();          // paint only when something changed
assert!(app.is_running());
assert_eq!(app.depth(), 1);
```

A host loops on those two calls until `is_running()` returns false, which
happens when the last screen finishes. `render_if_dirty` rather than `render`
because e-ink takes a second or more to refresh, so an unconditional repaint is
not free; `invalidate()` marks the screen dirty for a change the framework
cannot see, such as a window resize. `home_gesture()` offers the system gesture
to the top screen and pops everything down to the root if nothing claims it.

## Fonts

Fonts are named by what the text is *for*, not by typeface. Which face a role
resolves to is the host's business, and a role survives the assets being
changed.

```rust
use xpui::{Font, FontRole, FontStyle};

xpui::testing::install();
let ui = Font::ui();                                    // interface text: the widget default
let small = Font::ui_small();                           // captions, secondary labels
let reader = Font::reader();                            // the face the user reads in
let bold = Font::ui().bold();                           // also .italic()
let both = Font::ui().with_style(FontStyle::BoldItalic);

assert_eq!(Font::role(FontRole::Ui), ui);
assert!(ui.text_width("Battery") > 0);
assert!(ui.line_height() > 0);
```

`.bold()` and `.italic()` set the style rather than combining it with what is
already there, so ask for `FontStyle::BoldItalic` when you want both.

A build may compile a font out. That resolves to `Font::UNAVAILABLE`, which
measures zero and draws nothing — a missing face degrades quietly instead of
painting garbage at an arbitrary size:

```rust
use xpui::Font;

let missing = Font::UNAVAILABLE;
assert!(!missing.is_available());
assert_eq!(missing.text_width("Battery"), 0);
assert_eq!(missing.line_height(), 0);
```

Never estimate a width. `Text` measures through the host's own font engine
because an estimate drifts from what is painted, and content drifts off the
panel with it.

## The host façades

Widgets and screens reach the installed host through four small façades rather
than threading a context through every call.

| Façade | For |
|---|---|
| `Renderer` | Drawing primitives and `screen_size()`. Mostly for widget authors. |
| `Theme` | Themed furniture and the metrics behind it. |
| `Input` | One frame of buttons, touch and gestures. |
| `ScreenChrome` | The header band and the button hints. Used by the screen roots. |

Plus three free functions: `millis()`, `request_update()` and the navigation
pair above.

### Theme

Ask the theme for geometry rather than writing pixel offsets, and a change to
the theme needs no screen changes:

```rust
use xpui::{Renderer, Theme, ThemeMetric};

xpui::testing::install();
let content = Theme::content_area();   // between header and hints, already inset
let row = Theme::metric(ThemeMetric::ListRowHeight);

assert!(content.height() > 0);
assert!(row > 0);
assert!(content.width() < Renderer::screen_size().width);
```

`ThemeMetric` covers the header and hint bands, the content edges, list row
height and gap, progress bar height, the minimum touch size, the slider knob and
inset dimensions, sub-header height and the theme's two spacing steps. They are
asked for one at a time by tag rather than mirrored as a struct, because a copy
of a host's metrics table here would silently read the wrong field the day one
was inserted.

### Input

Buttons are named by meaning, never by position: the host applies the user's
remapping and the screen orientation, so a screen asking for `Confirm` gets
whatever the user has decided that is.

```rust
use xpui::{Button, Input, SwipeDir};

xpui::testing::install();
xpui::testing::reset();
assert!(!Input::was_pressed(Button::Back));
assert_eq!(Input::swipe(), SwipeDir::None);
assert_eq!(Input::tap(), None);
```

| Query | Reports |
|---|---|
| `was_pressed(b)` / `was_released(b)` | An edge — true for exactly one frame |
| `is_pressed(b)` | Whether it is down now |
| `tap()` | A completed tap, at the position the finger went down |
| `touch_held()` | Where the finger is while it is down — what a drag needs |
| `has_touch()` / `touch_released()` | Whether the panel is being touched at all |
| `swipe()` | The `SwipeDir` for this frame |
| `was_back_gesture()` / `was_home_gesture()` | System gestures |
| `swipe_moves_selection()` | Which way a vertical swipe walks focus |

The fifteen buttons are `Back`, `Confirm`, `Left`, `Right`, `Up`, `Down`,
`Power`, `PageBack`, `PageForward`, `NavNext`, `NavPrevious` and the four
`Screen*` directions, which are the directions as seen on the rendered panel
whatever the orientation.

A screen rarely calls any of this: the runtime reads input and delivers
messages. Reach for `Input` only when a screen genuinely needs the raw frame.

## Geometry

`Point`, `Size`, `Rect` and `Insets`, all in logical screen pixels in the
current orientation. Never assume a panel size — ask `Renderer::screen_size()`.

```rust
use xpui::{Insets, Point, Rect, Size};

let card = Rect::new(0, 0, 200, 80);
assert!(card.contains(Point::new(10, 10)));
assert!(!card.contains(Point::new(200, 10)));   // the right edge is outside
assert_eq!(card.inset(Insets::all(8)).size, Size::new(184, 64));

// Sizes are never negative, so a layout that over-subtracts cannot produce an
// inverted rectangle.
assert_eq!(Size::new(10, 10).shrink(20, 0), Size::new(0, 10));
```

`Rect` also offers `x()`, `y()`, `width()`, `height()`, `right()`, `bottom()`
and `intersects()`. Edges are treated consistently: both `contains` and
`intersects` count the right and bottom edges as outside, so two rectangles that
merely touch do not overlap.
