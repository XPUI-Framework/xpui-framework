# App

What drives a screen. `screen::Runtime` drives one `Screen`, and
`screen::Driver` is that screen with its type erased, which is what a firmware
whose own activity manager owns the stack keeps. `App` keeps a stack of them and
runs the frame loop, for a host with no navigation of its own: a simulator, a
bare-metal Rust binary, a test.

[How a frame runs](../architecture.md) follows one frame from input to paint.
[Screens](screens.md) is the trait these drive. This page is what each piece
does.

## Topics

| | |
|---|---|
| [`App`](#app) | A stack of screens, and the frame loop that drives the top one. |
| [`app::AppShell`](#appappshell) | What the navigator writes and `App` reads, in its own allocation. |
| [`screen::Driver`](#screendriver) | A screen with its type erased, so a host can drive one without knowing which `Screen` it is. |
| [`screen::Runtime`](#screenruntime) | Drives a `Screen`: owns focus, input routing, repeat timing and the first-paint guard. |

## `App`

A stack of screens, and the frame loop that drives the top one.

```text
pub struct App
```

`App` is the [`Navigator`](navigation.md#navigator) for a host with no
navigation of its own: a simulator, a bare-metal Rust binary, a test. A
firmware whose activity manager owns the stack drives one
[`screen::Driver`](#screendriver) at a time instead; see
[who owns the stack](navigation.md#who-owns-the-stack).

A host calls two methods in a loop until the last screen finishes:

```rust,no_run
# use xpui::{App, Screen, Text, View};
# struct MainMenu;
# impl Screen for MainMenu {
#     type Message = ();
#     fn body(&self) -> impl View<()> { Text::new("Main menu") }
#     fn update(&mut self, _message: ()) {}
# }
# xpui::testing::install();
let mut app = App::new(MainMenu);
while app.is_running() {
    app.tick(); // input, then any navigation it asked for
    app.render_if_dirty(); // paint only when something changed
}
```

**Navigation happens between frames.** A screen's `present` and `finish_screen`
are recorded while its frame runs, and `tick` acts on them once the frame is
over: the pop first, then the push. A screen that finishes itself and opens a
replacement in the same frame is replaced. One push is taken per frame, and a
second `present` in that frame returns `false`.

> [!NOTE]
> `App` is single-threaded. A host that renders on a second task implements
> `Navigator` over its own synchronisation instead.

**Example — a loop that ends**

```rust
use xpui::{App, Screen, Text, View, finish_screen, testing};

struct Splash {
    frames: u32,
}

impl Screen for Splash {
    type Message = ();

    fn body(&self) -> impl View<()> {
        Text::new("xpui")
    }

    fn update(&mut self, _message: ()) {}

    fn tick(&mut self) {
        self.frames += 1;
        if self.frames == 3 {
            finish_screen();
        }
    }
}

testing::install();
let mut app = App::new(Splash { frames: 0 });
let mut paints = 0;
while app.is_running() {
    app.tick();
    if app.render_if_dirty() {
        paints += 1;
    }
}
assert_eq!(paints, 1, "painted once, not once a frame");
```

**Example — a screen the host opens**

```rust
use xpui::{App, Screen, Text, View, testing};

# struct Page(&'static str);
# impl Screen for Page {
#     type Message = ();
#     fn body(&self) -> impl View<()> { Text::new(self.0) }
#     fn update(&mut self, _message: ()) {}
# }
testing::install();
let mut app = App::new(Page("Library"));
app.render();

// The battery driver reports 5%, which no screen could know.
app.push(Page("Battery low"));
assert_eq!(app.depth(), 2);
assert!(app.render_if_dirty(), "a push is a repaint");
```

### Starting an app

#### `App::new`

Starts an app showing `root`, and installs the navigator behind it.

```text
pub fn new<S: Screen + 'static>(root: S) -> Self
```

The root's `on_enter` runs here, before anything is painted. The navigator is an
[`app::AppShell`](#appappshell), leaked so it outlives every screen that could
call it. A second `App` installs its own navigator over the first one's.

#### `App::keep_root`

Refuses to finish the root screen, for a host with nothing underneath it.

```text
pub fn keep_root(mut self) -> Self
```

A window and a C++ host have somewhere to return to, so by default the last
screen finishing ends the app. A device does not: the loop stops, and a board
that stops answering looks exactly like one that crashed. Back still arrives,
so a screen can claim it and an open value still cancels with it; only the pop
is declined. A root that finishes and presents a replacement in the same frame
is replaced as usual.

**Example — a device's root screen**

```rust
use xpui::{App, Button, Screen, Text, View, testing};

# struct Menu;
# impl Screen for Menu {
#     type Message = ();
#     fn body(&self) -> impl View<()> { Text::new("Menu") }
#     fn update(&mut self, _message: ()) {}
# }
testing::install();
let mut app = App::new(Menu).keep_root();
testing::press(Button::Back);
app.tick();
assert!(app.is_running(), "the root declined to finish");

let mut plain = App::new(Menu);
testing::press(Button::Back);
plain.tick();
assert!(!plain.is_running(), "without it, Back ends the app");
```

### Running the loop

#### `App::tick`

One frame of input, then whatever navigation it asked for.

```text
pub fn tick(&mut self)
```

Runs the top screen's frame: its `tick`, then touch, swipe and keys, as
[`screen::Runtime`](#screenruntime) describes. Then it pops the screen if it
asked to finish, and pushes the screen it presented. Call it on every pass of
the loop, quiet ones included, or `Screen::tick` never runs. It paints nothing.

#### `App::render`

Paints the top screen, and whatever it is transparent over.

```text
pub fn render(&mut self)
```

Unconditional, and it clears the dirty flag before painting, so a repaint asked
for while it paints is kept for the next one. Below an overlay it paints each
screen from the first that is not an overlay upwards.

#### `App::render_if_dirty`

Paints only when something changed, returning whether it painted.

```text
pub fn render_if_dirty(&mut self) -> bool
```

E-ink takes a second or more to refresh, so an unconditional repaint is not
free. Something changed when a screen was pushed or popped, when any code
called `request_update`, or after [`invalidate`](#appinvalidate). Returns
`false` without painting once the stack is empty.

#### `App::is_running`

Whether any screen is left.

```text
pub fn is_running(&self) -> bool
```

The loop ends when the last one finishes. With [`keep_root`](#appkeep_root) it
never does.

#### `App::is_dirty`

Whether the screen has changed since it was last painted.

```text
pub fn is_dirty(&self) -> bool
```

True after a push or a pop, after `invalidate`, and after anything called
`request_update` since the last `render`. [`render_if_dirty`](#apprender_if_dirty)
is this and `render` in one call.

#### `App::invalidate`

Marks the screen as needing a repaint, for a host reacting to something the framework cannot see — a window resize, say.

```text
pub fn invalidate(&mut self)
```

```rust
# use xpui::{App, Screen, Text, View, testing};
# struct Page(&'static str);
# impl Screen for Page {
#     type Message = ();
#     fn body(&self) -> impl View<()> { Text::new(self.0) }
#     fn update(&mut self, _message: ()) {}
# }
testing::install();
let mut app = App::new(Page("Library"));
app.render();
assert!(!app.render_if_dirty(), "nothing changed");

app.invalidate(); // the window was resized
assert!(app.render_if_dirty());
```

### The stack

#### `App::push`

Pushes a screen and shows it.

```text
pub fn push<S: Screen + 'static>(&mut self, screen: S)
```

For the host: a screen asks with [`present`](navigation.md#present) instead,
since it has no `App` to call. The push is immediate rather than queued, runs
the screen's `on_enter`, and marks the app dirty, so the screen is painted by
the next `render_if_dirty`.

#### `App::depth`

How many screens are on the stack.

```text
pub fn depth(&self) -> usize
```

The root counts, so a running app is at least 1. In a test, prefer asserting
what is on the panel: a depth of 2 says that *something* opened.

#### `App::home_gesture`

Offers the system home gesture to the top screen, and pops everything down to the root if nothing claims it.

```text
pub fn home_gesture(&mut self)
```

The host calls this when its input reports the gesture; the runtime does not
look for one. Each popped screen gets its `on_exit`. The root is never popped
here, with or without `keep_root`.

**See also:** [`Screen`](screens.md#screen), [`Navigator`](navigation.md#navigator), [`app::AppShell`](#appappshell)

## `app::AppShell`

What the navigator writes and [`App`](#app) reads, in its own allocation.

```text
pub struct AppShell
```

A type no application names. It is public because it is the
[`Navigator`](navigation.md#navigator) that `App::new` installs, and a
navigator is installed as a `&'static`, while `App` needs `&mut self` to run
frames. So what a screen asks for lands here, in a separate leaked allocation,
and `App` reads it between frames. The same split keeps `finish_screen`, called
from inside a frame, from popping the screen that is running.

It holds three things: whether the running screen asked to finish, at most one
screen waiting to be pushed, and the top screen's title for
`Navigator::screen_title`. A second `present` in one frame finds the slot taken
and is handed back, so `present` returns `false` and the screen is dropped.

**Example — one push per frame**

```rust
use core::sync::atomic::{AtomicBool, Ordering};
use xpui::{App, Button, Screen, Text, View, present, testing};

static SECOND_TAKEN: AtomicBool = AtomicBool::new(true);

# struct Page(&'static str);
# impl Screen for Page {
#     type Message = ();
#     fn body(&self) -> impl View<()> { Text::new(self.0) }
#     fn update(&mut self, _message: ()) {}
# }
struct Launcher;

impl Screen for Launcher {
    type Message = ();

    fn body(&self) -> impl View<()> {
        Text::new("Launcher")
    }

    fn on_key(&self, key: Button) -> Option<()> {
        (key == Button::Confirm).then_some(())
    }

    fn update(&mut self, _message: ()) {
        present(Page("Library"));
        SECOND_TAKEN.store(present(Page("Settings")), Ordering::Relaxed);
    }
}

testing::install();
let mut app = App::new(Launcher);
testing::press(Button::Confirm);
app.tick();
assert_eq!(app.depth(), 2, "Library, and only Library");
assert!(!SECOND_TAKEN.load(Ordering::Relaxed));
```

**See also:** [`App`](#app), [`Navigator`](navigation.md#navigator)

## `screen::Driver`

A screen with its type erased, so a host can drive one without knowing which `Screen` it is.

```text
pub trait Driver
```

The lifecycle entry points live in the host crate. `Screen::body` returns
`impl View`, so a `Screen` is never a trait object; `Box<dyn Driver>` is what a
stack of different screens holds instead. It is what
[`Navigator::present`](navigation.md#navigatorpresent) receives, what `App`
keeps, and what a C++ firmware's activity calls through the FFI.

Implemented by [`screen::Runtime`](#screenruntime), for every `Screen`. A
screen never implements it.

**Example — a stack a host keeps itself**

```rust
use xpui::screen::{Driver, Runtime};
use xpui::{Screen, Text, View, testing};

# struct Page(&'static str);
# impl Screen for Page {
#     type Message = ();
#     fn body(&self) -> impl View<()> { Text::new(self.0) }
#     fn update(&mut self, _message: ()) {}
#     fn title(&self) -> Option<&'static str> { Some(self.0) }
# }
testing::install();
let mut stack: Vec<Box<dyn Driver>> = Vec::new();
for page in [Page("Home"), Page("Settings")] {
    let mut screen: Box<dyn Driver> = Box::new(Runtime::new(page));
    screen.on_enter();
    stack.push(screen);
}

let top = stack.last_mut().unwrap();
top.loop_();
top.render();
assert_eq!(top.title(), Some("Settings"));
```

### Required methods

#### `screen::Driver::on_enter`

The screen was pushed, before its first frame.

```text
fn on_enter(&mut self)
```

Forwards to `Screen::on_enter`.

#### `screen::Driver::loop_`

Ticks the screen, then handles one frame of input.

```text
fn loop_(&mut self)
```

Called on every frame, quiet ones included, or `tick` never runs. The trailing
underscore is because `loop` is a keyword.

#### `screen::Driver::on_exit`

The screen is being popped.

```text
fn on_exit(&mut self)
```

Forwards to `Screen::on_exit`.

#### `screen::Driver::render`

Paints the screen.

```text
fn render(&mut self)
```

Clears the panel first unless the screen is an overlay, and paints whether or
not anything changed; deciding when is the host's business.

#### `screen::Driver::handle_home_gesture`

Offers the home gesture; `true` means the screen consumed it.

```text
fn handle_home_gesture(&mut self) -> bool
```

#### `screen::Driver::title`

This screen's title, for a host that shows one.

```text
fn title(&self) -> Option<&'static str>
```

A host owning a stack has no other way to ask, since it cannot name the
`Screen`.

#### `screen::Driver::is_overlay`

Whether this screen paints over what is already on the panel.

```text
fn is_overlay(&self) -> bool
```

A host stacking screens reads this to know it must paint whatever sits beneath
before drawing this one.

**See also:** [`screen::Runtime`](#screenruntime), [`Navigator`](navigation.md#navigator)

## `screen::Runtime`

Drives a [`Screen`](screens.md#screen): owns focus, input routing, repeat timing and the first-paint guard.

```text
pub struct Runtime<S: Screen>
```

The state a screen never sees: which control holds focus, how far the scroll
view is scrolled, a value open for editing, and the button being held. The view
tree is rebuilt every frame and cannot remember any of it, so the runtime does.
It is how a `Screen` becomes a [`screen::Driver`](#screendriver).

Each frame ticks the screen, then offers input in this order. The first step
that produces a message, or moves the focus, ends the frame:

| Step | Offered to the screen as | Otherwise |
|---|---|---|
| tick | `Screen::tick` | always runs |
| a drag or a tap | the control under it, then `on_background_tap` | ignored |
| a swipe | `on_swipe` | up and down move focus |
| a key | `on_key` | the runtime's own meaning: focus, Confirm, a nudge, Back |

**No touch or swipe is routed before the first paint**, since the tree it would
be tested against has not been shown. Keys are. [How a frame
runs](../architecture.md#a-frame-with-a-touch-in-it) walks through the same
steps.

> [!WARNING]
> Driving a `Runtime` directly and asserting `focused_index` proves little:
> focus moves whether or not anything is drawn. Drive an `App` and assert what
> was painted; see [testing](../testing.md).

**Example — focus, from the outside**

```rust
use xpui::screen::{Driver, Runtime};
use xpui::{Button, List, ListRow, Screen, View, testing};

struct Rows;

impl Screen for Rows {
    type Message = usize;
    fn body(&self) -> impl View<usize> {
        List::new().extend(["Wi-Fi", "Bluetooth"].into_iter().enumerate().map(|(index, name)| ListRow::new(name).on_tap(index)))
    }
    fn update(&mut self, _message: usize) {}
}

testing::install();
testing::reset();
let mut runtime = Runtime::new(Rows);
Driver::render(&mut runtime);

testing::press(Button::Down);
Driver::loop_(&mut runtime);
assert_eq!(runtime.focused_index(), 1);
assert_eq!(testing::drawn_lists(), [(2, 0)], "and it has not repainted yet");
```

### Creating a runtime

#### `screen::Runtime::new`

A runtime driving `screen`, focus on its first control.

```text
pub fn new(screen: S) -> Self
```

Nothing is measured or painted here, so no host is needed yet.

### Inspecting it in a test

These two exist only with the `testing` feature.

#### `screen::Runtime::screen`

The screen itself.

```text
pub fn screen(&self) -> &S
```

Exposed only for tests that drive the runtime directly.

#### `screen::Runtime::focused_index`

Which interaction holds focus.

```text
pub fn focused_index(&self) -> usize
```

Exposed only for tests that drive the runtime directly; screens never see focus
at all. The index counts focus stops in the order the tree declares them.

**See also:** [`screen::Driver`](#screendriver), [`App`](#app)
