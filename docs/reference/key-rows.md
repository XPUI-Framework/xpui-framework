# Key rows

What the keys along the bottom edge of a device mean, left to right. A hint bar
divides its band into one slot per key and paints a word in each, and a device
states its row as data, because neither answer can be inferred from how many
keys it has.

[Input](input.md) is the buttons and the frame of input these keys produce.
[Navigation](navigation.md) is the hints a screen asks for, which a hint bar
paints into these slots. This page is what each piece does.

## Topics

| | |
|---|---|
| [`KeyRow`](#keyrow) | What the keys along the bottom edge mean, left to right. |
| [`RowKey`](#rowkey) | What one key along the bottom edge of a device does. |

## `KeyRow`

What the keys along the bottom edge mean, left to right.

```text
pub struct KeyRow(&'static [RowKey])
```

A hint bar asks two questions, how many slots to divide its band into and which
word goes in each, and a device answers both with its row. A slot with nothing
behind it is `RowKey::Unassigned` and stays blank: naming a key the device does
not have sends a person looking for it.

Neither answer can be inferred from the key count. A device with three keys
along the bottom and an up/down pair elsewhere has a key for Back and gives it
the first slot, and an inference from the count would put every label one key
to the left of what it names. See [the key row is data](../design.md#the-key-row-is-data-not-an-inference).

`KeyRow` lives here rather than beside what paints it because it is a fact
about hardware: the crate describing a device should not have to depend on the
one drawing it. A board crate states its row, and a hint bar such as
[`xpui-chrome`](https://github.com/XPUI-Framework/xpui-chrome)'s reads it, with
words the product supplies.

**Example — a reader's row, and a badge's own**

```rust
use xpui::{KeyRow, RowKey};

const BADGE: KeyRow = KeyRow::new(&[RowKey::Back, RowKey::Confirm, RowKey::Unassigned]);
const NO_ROW: KeyRow = KeyRow::new(&[]);

assert_eq!(KeyRow::READER.len(), 4);
assert!(KeyRow::READER.contains(RowKey::Next));
assert!(!BADGE.contains(RowKey::Next));
assert!(NO_ROW.is_empty(), "reserves no hint band");
```

**Example — the words for a board's hint row**

```rust
use xpui::{KeyRow, RowKey};

/// The product's own words: only it knows what language its user reads.
fn word(key: RowKey) -> &'static str {
    match key {
        RowKey::Back => "Back",
        RowKey::Confirm => "Select",
        RowKey::Previous => "Up",
        RowKey::Next => "Down",
        RowKey::Unassigned => "",
    }
}

const BADGE: KeyRow = KeyRow::new(&[RowKey::Back, RowKey::Confirm, RowKey::Unassigned]);

let slots: Vec<&str> = BADGE.iter().map(word).collect();
assert_eq!(slots, ["Back", "Select", ""]);
```

### Creating a row

#### `KeyRow::READER`

A reader's four keys, Back leftmost.

```text
pub const READER: KeyRow = KeyRow(&[ RowKey::Back, RowKey::Confirm, RowKey::Previous, RowKey::Next, ])
```

#### `KeyRow::new`

A row of a device's own.

```text
pub const fn new(keys: &'static [RowKey]) -> KeyRow
```

| Parameter | Meaning |
|---|---|
| `keys` | The slots, left to right. An empty slice is a device with no bottom row. |

`const`, so a board table can hold one.

### Reading a row

#### `KeyRow::len`

How many slots the hint band divides into.

```text
pub const fn len(&self) -> usize
```

#### `KeyRow::is_empty`

Whether the device has no bottom row at all.

```text
pub const fn is_empty(&self) -> bool
```

A device without one reserves no band, and there is nothing to label.

#### `KeyRow::contains`

Whether some slot carries `key`.

```text
pub fn contains(&self, key: RowKey) -> bool
```

#### `KeyRow::iter`

The slots, left to right.

```text
pub fn iter(&self) -> impl Iterator<Item = RowKey> + use<>
```

**See also:** [`RowKey`](#rowkey), [`Hint`](navigation.md#hint)

## `RowKey`

What one key along the bottom edge of a device does.

```text
pub enum RowKey
```

The vocabulary a [`KeyRow`](#keyrow) is written in. Which **word** each one
paints is not here: only a product knows what language its user reads, so the
words are supplied alongside the row.

| Variant | Meaning |
|---|---|
| `RowKey::Back` | Leaves the screen, or the value being edited. |
| `RowKey::Confirm` | Acts on whatever has focus. |
| `RowKey::Previous` | Walks a list backwards. |
| `RowKey::Next` | Walks a list forwards. |
| `RowKey::Unassigned` | A key with no word in the hint vocabulary, drawn blank. |

`Previous` is `Button::Up` on a device with a reader's four keys, and `Next` is
`Button::Down`. `Unassigned` covers a key nothing is mapped to and one whose
meaning has no label, as a power key's does.

How a value control shows that these keys have changed meaning while it is open
is [`ControlState`](theme.md#controlstate).

**See also:** [`KeyRow`](#keyrow), [`Button`](input.md#button),
[`ControlState`](theme.md#controlstate)
