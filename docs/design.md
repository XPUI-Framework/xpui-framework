# Design decisions

The arguments behind choices the code states in one sentence. Each section
names the item that carries the sentence.

## `App::keep_root` lives in the framework, not in a host

`Button::Back` means three things, tried in order: a screen may claim it
through `on_key`, an open edit cancels with it, and only then does it finish
the screen. A host that withholds the key to prevent the third suppresses all
three — a screen cannot dismiss its own picker, and a value opened on a root
screen can be committed but never cancelled. Declining the *pop* leaves the
first two meanings alone. Every host that owns no stack underneath its root
wants this, and one solving it for itself reaches for the key rather than the
pop; so the decision is `App`'s.

## The navigator is not the chrome

`Navigator` and `Chrome` have different owners: a backend crate supplies the
drawing, the application supplies the stack. `request_update` stays on
`Chrome` because a repaint is a display concern — every backend that can paint
can ask for one, and the framework calls it on every dispatch, so a host that
installed no navigator must still refresh. The name is `Navigator` rather
than `Shell` because in UI vocabulary a shell *is* the chrome, and a reader
would have no way to guess which trait a method lived on.

## Whether an edit is open is a widget's question

`Interactions::is_editing` sits on the interactions table rather than on
`Screen` or in `update` because a value widget genuinely needs it — a
third-party slider cannot paint the working copy without `editing_value` —
and because a screen that branched on the mode would make it the screen's
rather than the framework's. Nothing prevents a screen from writing a `View`
to read it; the design makes that the awkward way round rather than the
obvious one.

## A value control's number sits above the track

A number at the trailing end takes width from the track: around 40 columns
against the 288 a control gets on a 296-wide panel, and the 232 left between a
stepper's two glyphs — a seventh to a sixth of the room a value has. It also
separates the number from the name, leaving a list of settings reading as a
column of anonymous tracks. A line above the track costs height, which a
settings screen has, rather than width, which it does not. The line is the
control's rather than the screen's because only the control knows the working
value while an edit is open; a screen is not told it, and must not be.

## A held key re-arms after a blind gap

`Runtime::active_key` treats a gap longer than a repeat period as a new hold
rather than a repeat due. A panel that blocks for most of a second while it
refreshes leaves the loop blind, and a button released during the refresh
still reads as down on the frame after, because input is sampled at the top
of a frame. Crediting that gap to the hold turns one tap into a run: on a
slow panel a single tap of Down walks the selection several rows. Re-arming
loses a repeat a person genuinely earned by holding through a refresh and
keeps a tap moving by one; below the threshold nothing changes, and a display
that draws straight through never reaches it.

## The key row is data, not an inference

`KeyRow` exists because a hint bar's two questions — how many slots, and which
word in each — cannot be inferred from the key count. Three keys along the
bottom does not mean no key to spare for Back: a device with three there and
an up/down pair elsewhere has a key for Back and gives it the first slot, and
an inference from the count puts every label on it one key to the left of what
it names. So a device states its row, and a slot with nothing behind it is
`RowKey::Unassigned`.

## `Text` is one line

There is no wrapping widget yet, and list rows that carry a subtitle are
single-line by the theme's own rule. `Text` measures one line, the font's line
height, and a label wider than its space is cut by the panel's edge. A list
row is different: `Chrome::draw_list` hands the host each title, value and
subtitle whole, so whether a long one ends in an ellipsis is the backend's
decision rather than the framework's. `xpui-chrome` shortens them; a C++ host
drawing its own widgets does whatever that toolkit does.
