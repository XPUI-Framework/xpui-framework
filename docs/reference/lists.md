# Lists

Rows of settings and read-outs drawn by the host's theme, and the two pieces
that group a screen around them. The theme owns a row's height, its highlight
and its type, so a list built here looks exactly like the device's own.

![A settings list of three rows: Frontlight reading On with the focus marker, Sleep after reading 5 min, and Free heap reading 182 KB](https://raw.githubusercontent.com/XPUI-Framework/xpui-gallery/main/gallery/tests/screenshots/reference/lists_list.png)

[A second screen](../a-second-screen.md) builds a list, navigation and scrolling
from nothing. This page is what each piece does.

## Topics

| | |
|---|---|
| [`List`](#list) | A themed, selectable list filling the space it is given. |
| [`ListRow`](#listrow) | One row: a title, and optionally a subtitle beneath it and a value on the right. |
| [`list!`](#list-1) | A themed list. |
| [`Section`](#section) | A heading with content beneath it, for splitting a screen into groups. |
| [`Divider`](#divider) | A one-pixel line spanning the width it is given, for separating sections. |

## `List`

A themed, selectable list filling the space it is given.

```text
pub struct List<M>
```

`M` is the screen's message type: what a row sends when it is tapped.

| Builder | Sets | When not called |
|---|---|---|
| [`push`](#listpush) · [`extend`](#listextend) | the rows, in order | no rows |
| [`selected`](#listselected) | the row highlighted while none holds focus | nothing highlighted |

**Focus is the framework's.** A row that carries a message
([`ListRow::on_tap`](#listrowon_tap)) is a focus stop, in the order the rows
read. Up and Down walk them; Confirm sends the focused row's message, which is
the same message a tap on that row sends. The list highlights whichever row
holds focus, and the screen writes no code for any of it. A row without a
message is a read-out: drawn, and never focused.

**A list is as tall as its rows**, capped by the height it is offered, so
several lists can share one screen, one per [`Section`](#section). Rows that do
not fit are neither drawn nor focusable. Put a list longer than the panel in a
`ScrollView`, which keeps the focused row in sight.

> [!NOTE]
> When any row has a subtitle, every row in the list uses the theme's taller
> two-line height, so the rows stay in line.

**Example — a settings screen**

```rust
use xpui::{List, ListRow, NavigationScreen, Screen, View};

#[derive(Clone, Copy)]
enum Msg {
    Frontlight(bool), // the state the row moves to
    Sleep,
}

struct Settings {
    frontlight: bool,
}

impl Screen for Settings {
    type Message = Msg;

    fn body(&self) -> impl View<Msg> {
        NavigationScreen::new(
            List::new()
                .push(
                    ListRow::toggle("Frontlight", self.frontlight, "On", "Off")
                        .on_tap(Msg::Frontlight(!self.frontlight)),
                )
                .push(ListRow::new("Sleep after").value("5 min").on_tap(Msg::Sleep))
                .push(ListRow::new("Free heap").value("182 KB")), // a read-out
        )
    }

    fn update(&mut self, message: Msg) {
        match message {
            Msg::Frontlight(on) => self.frontlight = on,
            Msg::Sleep => {}
        }
    }

    fn title(&self) -> Option<&'static str> {
        Some("Settings")
    }
}
```

**Example — rows built from data**

```rust
use xpui::{List, ListRow};

let networks = ["Home", "Office", "Library"];
let list: List<usize> = List::new().extend(
    networks
        .iter()
        .enumerate()
        .map(|(index, name)| ListRow::new(*name).on_tap(index)),
);
assert_eq!(list.len(), 3);
```

### Creating a list

#### `List::new`

An empty list with nothing selected.

```text
pub fn new() -> Self
```

`List` is also `Default`.

### Adding rows

#### `List::push`

Adds a row at the end.

```text
pub fn push(self, row: ListRow<M>) -> Self
```

#### `List::extend`

Adds every row of `rows`, in order.

```text
pub fn extend(self, rows: impl IntoIterator<Item = ListRow<M>>) -> Self
```

### Highlighting

#### `List::selected`

Highlights a row whenever none of the list's rows holds focus.

```text
pub fn selected(self, index: usize) -> Self
```

| Parameter | Meaning |
|---|---|
| `index` | Zero-based. An index past the last row highlights nothing. |

A list whose rows carry messages is highlighted by whichever row holds focus,
and focus wins. This is for rows nothing focuses: a list of read-outs, or a
list while focus is on a control beside it.

### Reading a list

#### `List::len`

How many rows it holds.

```text
pub fn len(&self) -> usize
```

#### `List::is_empty`

Whether it holds no rows.

```text
pub fn is_empty(&self) -> bool
```

**See also:** [`ListRow`](#listrow), [`list!`](#list-1), [`Section`](#section)

## `ListRow`

One row: a title, and optionally a subtitle beneath it and a value on the right.

```text
pub struct ListRow<M>
```

![Four rows: Wi-Fi with the subtitle Home network, Sleep after reading 5 min, Hyphenation reading On, and Firmware reading 1.4.2](https://raw.githubusercontent.com/XPUI-Framework/xpui-gallery/main/gallery/tests/screenshots/reference/lists_rows.png)

A row is not a view by itself: it exists to go in a [`List`](#list), which
measures it and has the theme draw it. Its strings are converted once, when the
row is built, not on every frame. What happens to text too long for the row is
the theme's decision.

| Builder | Sets | When not called |
|---|---|---|
| [`ListRow::new`](#listrownew) | the title | — |
| [`subtitle`](#listrowsubtitle) | a second line under the title | one line |
| [`value`](#listrowvalue) | right-aligned text | no value |
| [`on_tap`](#listrowon_tap) | the message, which makes the row a focus stop | a read-out, never focused |

**Example — every shape of row**

```rust
use xpui::{List, ListRow};

#[derive(Clone, Copy)]
enum Msg {
    Network,
    Sleep,
    Hyphenation(bool),
}

let hyphenation = true;
let list: List<Msg> = List::new()
    .push(ListRow::new("Wi-Fi").subtitle("Home network").on_tap(Msg::Network))
    .push(ListRow::new("Sleep after").value("5 min").on_tap(Msg::Sleep))
    .push(ListRow::toggle("Hyphenation", hyphenation, "On", "Off").on_tap(Msg::Hyphenation(!hyphenation)))
    .push(ListRow::new("Firmware").value("1.4.2"));
assert_eq!(list.len(), 4);
```

### Creating a row

#### `ListRow::new`

A one-line row reading `title`, with no message.

```text
pub fn new(title: impl Into<String>) -> Self
```

#### `ListRow::toggle`

A boolean setting: a row whose value reads as one of two words.

```text
pub fn toggle(title: impl Into<String>, on: bool, on_label: impl Into<String>, off_label: impl Into<String>) -> Self
```

| Parameter | Meaning |
|---|---|
| `title` | What the setting is called. |
| `on` | Its current state. |
| `on_label` | The value shown while `on` is `true`: "On", "Show". |
| `off_label` | The value shown while it is `false`: "Off", "Hide". |

The theme draws no switch graphic: the row reads one word or the other, and the
focus marker shows which row the keys are on. The caller supplies both words
because they differ from setting to setting and only the caller can translate
them.

**The row does not flip itself.** Give it a message carrying the state it moves
to, `.on_tap(Msg::Hyphenation(!on))`, and assign that in `update`. A message
that only says "toggle" undoes itself if it is applied twice.
[`Toggle`](toggles.md#toggle) works the next state out for you.

### Content

#### `ListRow::subtitle`

Secondary text below the title.

```text
pub fn subtitle(self, subtitle: impl Into<String>) -> Self
```

Every row in the list gets taller when any row has one.

#### `ListRow::value`

Right-aligned text, for the current setting of an option row.

```text
pub fn value(self, value: impl Into<String>) -> Self
```

### Interaction

#### `ListRow::on_tap`

Makes the row interactive: tapping it, or focusing it and pressing Confirm, sends `message`.

```text
pub fn on_tap(self, message: M) -> Self
```

A row without a message is a read-out, and never takes focus. The message is
cloned each time it is sent, so it is a value, never a closure.

### Reading a row

#### `ListRow::value_text`

The right-hand value as text, if the row has one.

```text
pub fn value_text(&self) -> Option<&str>
```

Mainly for tests: the theme reads a row's fields for itself.

**See also:** [`List`](#list), [`list!`](#list-1)

## `list!`

A themed list.

```text
macro_rules! list
```

```text
list![selected; row, row, …]
list![selected]
```

Shorthand for a list whose rows are fixed: it expands to
`List::new().selected(selected)` followed by one `.push(row)` per row, and
hides nothing. `selected` is always given, and applies only while no row holds
focus; see [`List::selected`](#listselected).

```rust
use xpui::{List, ListRow, list};

let selected = 0;
let list: List<()> = list![selected;
    ListRow::new("Wi-Fi").value("On"),
    ListRow::new("Bluetooth").value("Off"),
];
assert_eq!(list.len(), 2);
```

## `Section`

A heading with content beneath it, for splitting a screen into groups.

```text
pub struct Section<M>
```

![Two sections, Display and Power, each a heading with a rule and a one-row list beneath it](https://raw.githubusercontent.com/XPUI-Framework/xpui-gallery/main/gallery/tests/screenshots/reference/lists_section.png)

The heading is drawn by the theme's own sub-header, so it matches the host's
headings rather than being a bold [`Text`](text-and-images.md#text) that looks
similar. Between the heading and its content is the theme's *small* spacing
step. The space between one section and the next is the stack's own, so a
heading sits closer to what it heads than to the group above.

The content may be any view. It is usually a [`List`](#list).

**Example — a screen in two groups**

```rust
use xpui::{List, ListRow, Section, VStack, vstack};

#[derive(Clone, Copy)]
enum Msg {
    Frontlight,
    Sleep,
}

# xpui::testing::install();
let screen: VStack<Msg> = vstack![16;
    Section::new("Display", List::new().push(ListRow::new("Frontlight").value("On").on_tap(Msg::Frontlight))),
    Section::new("Power", List::new().push(ListRow::new("Sleep after").value("5 min").on_tap(Msg::Sleep))),
];
```

### Creating a section

#### `Section::new`

`content` under a heading reading `title`.

```text
pub fn new(title: impl Into<String>, content: impl View<M> + 'static) -> Self
```

## `Divider`

A one-pixel line spanning the width it is given, for separating sections.

```text
pub struct Divider
```

![Wi-Fi and Bluetooth separated by a one-pixel rule](https://raw.githubusercontent.com/XPUI-Framework/xpui-gallery/main/gallery/tests/screenshots/reference/lists_divider.png)

A divider is one pixel tall and as wide as its parent offers. It carries no
message type of its own, so it sits in any stack, and it is `Default`.

```rust
use xpui::{Divider, Text, VStack, vstack};

# xpui::testing::install();
let group: VStack<()> = vstack![8; Text::new("Wi-Fi"), Divider::new(), Text::new("Bluetooth")];
```

### Creating a divider

#### `Divider::new`

A rule across the width it is given.

```text
pub fn new() -> Self
```

**See also:** [`Section`](#section)
