# Input

How a key, a touch or a swipe reaches a screen. A screen never polls a button,
never computes a rect and never hit-tests. Each widget declares the regions it
owns and the message each one produces, and every frame the runtime resolves
input against that list and hands the screen one of its own messages. So a
touch panel and a device with four buttons run the same screen.

[Writing a widget](../writing-a-widget.md) builds a control that declares its
own regions. [Controls](controls.md) is what the keys do to a slider or a
stepper. This page is the vocabulary underneath both: the buttons and the frame
of input. The keys along the bottom edge are [Key rows](key-rows.md), and what a
widget declares, and how the runtime resolves it, is
[Interactions](interactions.md).

## Topics

| | |
|---|---|
| [`Button`](#button) | A button by meaning, never by physical position. |
| [`SwipeDir`](#swipedir) | The direction a completed swipe travelled. |
| [`Input`](#input-1) | One frame of input, as screens reach for it. |

## How input reaches a screen

### Messages, not closures

**A screen tags its controls with its own messages**, and the runtime delivers
them to `update`. A control's message is built by a function pointer,
`fn(i32) -> M` or `fn(bool) -> M`, which is what an enum variant's constructor
already is: in `.on_change(Msg::Brightness)`, `Msg::Brightness` is
`fn(i32) -> Msg`. The framework converts the touch position into a value and
calls it, so slider geometry never reaches a screen.

A closure that mutated screen state from inside a tree the screen also owns
would need interior mutability, and a failed `RefCell` borrow panics, which
aborts on a device with no unwinder. A message is a plain value, and nothing
borrows. A function pointer costs nothing to store and cannot close over
anything.

**Example — a screen that only names messages**

```rust
use xpui::{IconRef, IconToggle, Screen, Stepper, View, vstack};
# #[derive(Copy, Clone)]
# enum Glyph { Sun = 0 }
# impl From<Glyph> for IconRef {
#     fn from(glyph: Glyph) -> IconRef { IconRef::new(glyph as u16) }
# }

#[derive(Clone, Copy)]
enum Msg {
    Brightness(i32), // a new absolute value
    Step(i32),       // a relative nudge
    Light(bool),     // the state the icon is moving to
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
                .on_change(Msg::Brightness) // dragged or tapped on the track
                .on_step(Msg::Step),        // -1 / +1 from the end glyphs
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

### Focus is the framework's

Up and Down move focus through the focusable interactions **in tree order**,
wrapping at both ends, and never reach `update`. Confirm fires the focused
control's message, **the identical message a tap produces**, so touch and
buttons cannot drift apart. A `List` highlights whichever row holds focus with
no screen code at all. How a value control takes the keys over and gives them
back is in [controls](controls.md#how-a-control-is-driven).

The runtime reads one frame in this order, and at each step the screen is asked
first:

| Step | Offered to the screen through | Then the runtime |
|---|---|---|
| a touch held on a `DRAG` region | nothing | sends that region's message every frame |
| a touch held 500 ms on a `LONG_PRESS` region | nothing | sends that region's message once; the release sends nothing |
| a tap on a `TAP` region | `Screen::on_background_tap`, when no region and no dialog's `on_dismiss` takes it | sends that region's message |
| a swipe | `Screen::on_swipe` | moves focus on a vertical swipe |
| a key press, a held key's repeat, or the back gesture | `Screen::on_key` | walks focus, nudges, confirms, dismisses a dialog, or finishes the screen |

Touches and swipes are declined while a value is open for editing. A held key
repeats every 500 ms after a 500 ms delay, and a finger becomes a long press
after the same 500 ms.

### A vertical swipe moves focus

A touch panel and a button device navigate the same list the same way. Which
way a swipe walks is the host's preference,
[`Input::swipe_moves_selection`](#inputswipe_moves_selection). A horizontal swipe
moves nothing: that axis belongs to the back and home gestures.

### Ask what the device has

**Never assume it.** [`Input::has_left_right_keys`](#inputhas_left_right_keys)
answers whether there is a pair to nudge a value with. No rule of thumb about
the shape of a device gets it right, since two devices of the same family
differ, and the host trait gives no default to fall back on, so a backend cannot
inherit a guess. Which device answers what is for the vendor crates under
[`xpui-boards`](https://github.com/XPUI-Framework/xpui-boards) to say.

### Claiming a key

A screen that wants a key or a gesture for itself claims it in
[`Screen::on_key`](screens.md#screenon_key) or
[`Screen::on_swipe`](screens.md#screenon_swipe), and is asked before the runtime.
Back is the key worth knowing the runtime keeps: unclaimed, it finishes the
screen, or, while a dialog is open, goes to the dialog instead; see
[`Modal::on_dismiss`](dialogs.md#modalon_dismiss).

## `Button`

A button by meaning, never by physical position.

```text
pub enum Button
```

The host applies the user's remapping and the screen orientation, so a screen
asking for `Confirm` gets whatever the user has decided that is. Which physical
key sends which meaning is the host's to say, and a device may send only some
of the fifteen.

| Variant | Meaning |
|---|---|
| `Button::Back` | Leaves the screen, cancels an open edit, or dismisses a dialog. |
| `Button::Confirm` | Acts on whatever has focus. |
| `Button::Left` | Nudges the focused value down; where nothing under the focus adjusts, moves focus back. |
| `Button::Right` | Nudges the focused value up; where nothing under the focus adjusts, moves focus forward. |
| `Button::Up` | Moves focus to the previous control, or raises an open value. |
| `Button::Down` | Moves focus to the next control, or lowers an open value. |
| `Button::Power` | The power key. |
| `Button::PageBack` | Page navigation backwards, honouring the user's side-button swap. |
| `Button::PageForward` | Page navigation forwards, honouring the user's side-button swap. |
| `Button::NavNext` | The next item, as a reader's side key means it. |
| `Button::NavPrevious` | The previous item, as a reader's side key means it. |
| `Button::ScreenLeft` | Left as seen on the rendered screen, whatever the orientation. |
| `Button::ScreenRight` | Right as seen on the rendered screen, whatever the orientation. |
| `Button::ScreenUp` | Up as seen on the rendered screen, whatever the orientation. |
| `Button::ScreenDown` | Down as seen on the rendered screen, whatever the orientation. |

**All fifteen reach `Screen::on_key`**, before the runtime gives any of them a
meaning, and a held key repeats whether or not the screen claims it.

**Eight of them mean something to the runtime.** When the screen declines
`Left`, `Right`, `Up`, `Down`, `Confirm`, `Back`, `PageBack` or `PageForward`,
the runtime gives it the meaning above. `PageBack` walks focus like `Up` and
`PageForward` like `Down`, which is what a device with only two side keys
relies on. `Power`, `NavNext`, `NavPrevious` and the four `Screen*` directions
mean nothing to the runtime: a screen that wants one claims it, and one that
declines it loses nothing.

**The back gesture is `Back`.** When the host reports
[`Input::was_back_gesture`](#inputwas_back_gesture), the runtime takes it as a
press of `Back`: offered to `on_key`, then cancelling an open edit, dismissing a
dialog or finishing the screen. It does not repeat.

**Example — a reader's side keys skip chapters**

```rust
use xpui::screen::{Driver, Runtime};
use xpui::{Button, NavigationScreen, Screen, Text, View, testing};

#[derive(Clone, Copy)]
enum Msg {
    Chapter(i32),
}

struct Book {
    chapter: i32,
}

impl Screen for Book {
    type Message = Msg;

    fn body(&self) -> impl View<Msg> {
        NavigationScreen::new(Text::new("Chapter"))
    }

    fn update(&mut self, message: Msg) {
        let Msg::Chapter(by) = message;
        self.chapter = (self.chapter + by).max(1);
    }

    fn on_key(&self, key: Button) -> Option<Msg> {
        match key {
            Button::NavNext => Some(Msg::Chapter(1)),
            Button::NavPrevious => Some(Msg::Chapter(-1)),
            _ => None,
        }
    }

    fn title(&self) -> Option<&'static str> {
        Some("Walden")
    }
}

testing::install();
testing::reset();
let mut runtime = Runtime::new(Book { chapter: 3 });
runtime.render();
testing::press(Button::NavNext);
runtime.loop_();
assert_eq!(runtime.screen().chapter, 4);
```

`Button` is `#[repr(u8)]`, numbered in the order of the table from `Back = 0`
to `ScreenDown = 14`, so a backend can pass it across a C boundary.

**Example — a reader that claims the page keys and a horizontal swipe**

```rust
use xpui::screen::{Driver, Runtime};
use xpui::{Button, NavigationScreen, Screen, SwipeDir, Text, View, testing};

#[derive(Clone, Copy)]
enum Msg {
    Turn(i32),
}

struct Reader {
    page: i32,
}

impl Screen for Reader {
    type Message = Msg;

    fn body(&self) -> impl View<Msg> {
        NavigationScreen::new(Text::new("Chapter one"))
    }

    fn update(&mut self, message: Msg) {
        let Msg::Turn(by) = message;
        self.page = (self.page + by).max(0);
    }

    fn on_key(&self, key: Button) -> Option<Msg> {
        match key {
            Button::PageForward => Some(Msg::Turn(1)),
            Button::PageBack => Some(Msg::Turn(-1)),
            _ => None, // Back stays the runtime's
        }
    }

    fn on_swipe(&self, direction: SwipeDir) -> Option<Msg> {
        match direction {
            SwipeDir::Left => Some(Msg::Turn(1)),
            SwipeDir::Right => Some(Msg::Turn(-1)),
            _ => None,
        }
    }

    fn title(&self) -> Option<&'static str> {
        Some("Reader")
    }
}

testing::install();
testing::reset();
let mut runtime = Runtime::new(Reader { page: 0 });
runtime.render();

testing::press(Button::PageForward);
runtime.loop_();
testing::set_swipe(SwipeDir::Left);
runtime.loop_();
assert_eq!(runtime.screen().page, 2);

// Unclaimed, Back finishes the screen.
testing::press(Button::Back);
runtime.loop_();
assert_eq!(testing::finishes(), 1);
```

**See also:** [`Input`](#input-1), [`RowKey`](key-rows.md#rowkey),
[`Screen::on_key`](screens.md#screenon_key)

## `SwipeDir`

The direction a completed swipe travelled.

```text
pub enum SwipeDir
```

A swipe is an edge event: the host reports it on the one frame it completes,
and [`Input::swipe`](#inputswipe) answers `SwipeDir::None` on every other.
`SwipeDir::None` is also the `Default`.

| Variant | Meaning |
|---|---|
| `SwipeDir::None` | No swipe this frame. |
| `SwipeDir::Left` | Towards the left edge. |
| `SwipeDir::Right` | Towards the right edge. |
| `SwipeDir::Up` | Towards the top edge. |
| `SwipeDir::Down` | Towards the bottom edge. |

A swipe the screen does not claim in `Screen::on_swipe` moves focus when it is
vertical, and does nothing when it is horizontal:

| `swipe_moves_selection()` | A swipe up | A swipe down |
|---|---|---|
| `false`, the default | moves focus to the next control, as though dragging the page | moves focus to the previous control |
| `true` | moves focus to the previous control | moves focus to the next control |

**Example — both readings of a vertical swipe**

```rust
use xpui::screen::{Driver, Runtime};
use xpui::{Button, List, ListRow, NavigationScreen, Screen, SwipeDir, View, testing};

#[derive(Clone, Copy)]
enum Msg {
    Open(usize),
}

struct Library {
    opened: Option<usize>,
}

impl Screen for Library {
    type Message = Msg;

    fn body(&self) -> impl View<Msg> {
        NavigationScreen::new(
            List::new()
                .push(ListRow::new("Books").on_tap(Msg::Open(0)))
                .push(ListRow::new("Articles").on_tap(Msg::Open(1)))
                .push(ListRow::new("Notes").on_tap(Msg::Open(2))),
        )
    }

    fn update(&mut self, message: Msg) {
        let Msg::Open(index) = message;
        self.opened = Some(index);
    }

    fn title(&self) -> Option<&'static str> {
        Some("Library")
    }
}

testing::install();
testing::reset();
let mut runtime = Runtime::new(Library { opened: None });
runtime.render();

// By default a swipe drags the content, so swiping up walks focus down.
testing::set_swipe(SwipeDir::Up);
runtime.loop_();
assert_eq!(runtime.focused_index(), 1);

// With the host's preference flipped, the focus follows the finger.
testing::set_swipe_moves_selection(true);
testing::set_swipe(SwipeDir::Up);
runtime.loop_();
assert_eq!(runtime.focused_index(), 0);

// Confirm sends the focused row's message: the one a tap would send.
testing::press(Button::Confirm);
runtime.loop_();
assert_eq!(runtime.screen().opened, Some(0));
```

**See also:** [`Input::swipe`](#inputswipe), [`Button`](#button),
[`Screen::on_swipe`](screens.md#screenon_swipe)

## `Input`

One frame of input, as screens reach for it.

```text
pub struct Input
```

`Input` has no value to hold: each query is an associated function that asks
the installed host's
[`InputSource`](backend-contract.md#hostinputsource) the same question.
**A screen rarely calls any of this**, because the runtime reads input and
delivers messages. Reach for `Input` when a screen needs the raw frame, or a
fact about the device.

| Query | Reports |
|---|---|
| [`was_pressed(b)`](#inputwas_pressed) / [`was_released(b)`](#inputwas_released) | an edge: true for exactly one frame |
| [`is_pressed(b)`](#inputis_pressed) | whether the button is down now |
| [`tap()`](#inputtap) | a completed tap, at the position the finger went down |
| [`touch_held()`](#inputtouch_held) | where the finger is while it is down, which is what a drag needs |
| [`has_touch()`](#inputhas_touch) / [`touch_released()`](#inputtouch_released) | whether *this frame* carries a touch, never whether the panel has a digitiser |
| [`swipe()`](#inputswipe) | the `SwipeDir` for this frame |
| [`was_back_gesture()`](#inputwas_back_gesture) / [`was_home_gesture()`](#inputwas_home_gesture) | the system gestures |
| [`swipe_moves_selection()`](#inputswipe_moves_selection) | which way a vertical swipe walks focus |
| [`has_left_right_keys()`](#inputhas_left_right_keys) | whether the device has a pair to nudge a value with |

Every query but the last describes this frame. `has_left_right_keys` describes
the device, and answers the same thing every time it is asked.

> [!WARNING]
> With no host installed, every query panics. A build with the `testing`
> feature installs the fake host instead, whose frame is empty until a test
> puts something in it: see [testing](testing.md).

**Example — reading a frame under the fake host**

```rust
use xpui::{Button, Input, SwipeDir, testing};

testing::install();
testing::reset();
assert!(!Input::was_pressed(Button::Back));
assert_eq!(Input::swipe(), SwipeDir::None);
assert_eq!(Input::tap(), None); // the fake host has no touch panel

// A held key is an edge on its first frame and a level until it lifts.
testing::hold(Button::Down);
assert!(Input::was_pressed(Button::Down));
assert!(Input::is_pressed(Button::Down));
testing::release();
assert!(!Input::is_pressed(Button::Down));
```

**Example — a screen that asks what the device has**

```rust
use xpui::{Input, NavigationScreen, Screen, Text, View, testing};

struct Help;

impl Screen for Help {
    type Message = ();

    fn body(&self) -> impl View<()> {
        let how = if Input::has_left_right_keys() {
            "Left and Right change a value"
        } else {
            "Confirm opens a value, Up and Down move it"
        };
        NavigationScreen::new(Text::new(how))
    }

    fn update(&mut self, _message: ()) {}

    fn title(&self) -> Option<&'static str> {
        Some("Help")
    }
}

testing::install();
testing::reset();
testing::set_has_left_right_keys(true);
assert!(Input::has_left_right_keys());
```

### Reading buttons

#### `Input::was_pressed`

See `InputSource::was_pressed`.

```text
pub fn was_pressed(button: Button) -> bool
```

Whether `button` went down this frame. The runtime reads this for its eight
keys; a screen reads it for the other seven.

#### `Input::is_pressed`

See `InputSource::is_pressed`.

```text
pub fn is_pressed(button: Button) -> bool
```

Whether `button` is down, this frame included. A level, not an edge: it is
what auto-repeat reads.

#### `Input::was_released`

See `InputSource::was_released`.

```text
pub fn was_released(button: Button) -> bool
```

Whether `button` came up this frame.

### Reading touch

#### `Input::has_touch`

See `InputSource::has_touch`.

```text
pub fn has_touch() -> bool
```

Whether this frame carries any touch at all, down or just lifted. The runtime
looks at taps and drags only when it is true.

#### `Input::tap`

See `InputSource::tap`.

```text
pub fn tap() -> Option<Point>
```

A completed tap, at the position the finger went down.

#### `Input::touch_held`

See `InputSource::touch_held`.

```text
pub fn touch_held() -> Option<Point>
```

Where the finger is now, while it is down. The runtime offers it to `DRAG`
regions every frame.

#### `Input::touch_released`

See `InputSource::touch_released`.

```text
pub fn touch_released() -> bool
```

Whether a finger lifted this frame. The release that ends a drag is swallowed
rather than read as a tap.

### Reading gestures

#### `Input::swipe`

See `InputSource::swipe`.

```text
pub fn swipe() -> SwipeDir
```

A completed swipe, or `SwipeDir::None`.

#### `Input::was_back_gesture`

See `InputSource::was_back_gesture`.

```text
pub fn was_back_gesture() -> bool
```

The system back gesture, an edge swipe on a touch device. The runtime takes it
as a press of `Button::Back`, so a screen sees it in `Screen::on_key`.

#### `Input::was_home_gesture`

See `InputSource::was_home_gesture`.

```text
pub fn was_home_gesture() -> bool
```

The system home gesture. `App::tick` reads it and offers it to the top screen
through `Screen::handle_home_gesture`, so neither a screen nor a host needs to.

#### `Input::swipe_moves_selection`

See `InputSource::swipe_moves_selection`.

```text
pub fn swipe_moves_selection() -> bool
```

Which way a vertical swipe moves focus. `false`, the default a host inherits,
moves the content, so swiping up walks down the list. `true` moves the focus
with the finger. See [`SwipeDir`](#swipedir).

### Asking what the device has

#### `Input::has_left_right_keys`

See `InputSource::has_left_right_keys`.

```text
pub fn has_left_right_keys() -> bool
```

Whether the device has a Left/Right pair to nudge a value with. With the pair,
Left and Right move a value where it stands. Without one, Confirm opens a
value and Up and Down move it: see [the value mode](controls.md#the-value-mode).

**See also:** [`Button`](#button), [`SwipeDir`](#swipedir),
[`InputSource`](backend-contract.md#hostinputsource)
