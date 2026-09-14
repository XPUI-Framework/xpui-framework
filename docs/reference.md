# Reference

The whole of the public API, by area. [architecture.md](architecture.md)
explains how a frame runs and [tutorial.md](tutorial.md) builds one screen from
nothing; this is what you reach for once you know the shape and want to know
what exists.

## Topics

Each page lists every public name in its area with its declaration, its
parameters, examples you can copy, and a picture of whatever it draws. The gate
checks every page against the code: every public name has a section, and what a
page says an item is, rustdoc says too.

| Group | Page | Holds |
|---|---|---|
| App structure | [screens](reference/screens.md) | `Screen`: what a screen is, and every method the runtime calls |
| | [app](reference/app.md) | `App`, `AppShell`, `Driver`, `Runtime`: the loop a host runs |
| | [navigation](reference/navigation.md) | `present`, `finish_screen`, `NavigationScreen`, `OverlayPanel`, `Hint`, `Navigator`, and a two-screen walkthrough |
| Views & layout | [views](reference/views.md) | `View`, `ViewExt`, `Mapped`: boxing, measuring, components and `.map` |
| | [stacks](reference/stacks.md) | `VStack`, `HStack`, `vstack!`, `hstack!`, `Alignment`: how a stack measures |
| | [layout](reference/layout.md) | `Spacer`, `Padding`, `ScrollView`, `UNBOUNDED` |
| | [modifiers](reference/modifiers.md) | `Modifiers`, `Frame`, `Flexible`, `Tappable` |
| Components | [text and fonts](reference/text.md) | `Text`, `Font`, `FontId`, `FontRole`, `FontStyle` |
| | [images](reference/images.md) | `Image`, `Icon`, `IconRef` |
| | [controls](reference/controls.md) | `Slider`, and how a value control takes the keys |
| | [steppers](reference/steppers.md) | `Stepper` |
| | [toggles](reference/toggles.md) | `Toggle`, `IconToggle` |
| | [lists](reference/lists.md) | `List`, `ListRow`, `list!`, `Section`, `Divider` |
| | [dialogs](reference/dialogs.md) | `Modal`, `Scrim` |
| | [indicators](reference/indicators.md) | `ProgressBar` |
| Input | [input](reference/input.md) | `Button`, `SwipeDir`, `Input`: how input reaches a screen |
| | [key rows](reference/key-rows.md) | `KeyRow`, `RowKey`: what the keys along the bottom edge mean |
| | [interactions](reference/interactions.md) | `InputMask`, `Interaction`, `Interactions`: how a view declares where it can be touched |
| | [triggers](reference/triggers.md) | `Trigger`, `value_at`: what an interaction produces when it fires |
| Host | [theme](reference/theme.md) | `Theme`, `ThemeMetric`, `ControlState`: the theme's geometry and furniture |
| | [drawing and repainting](reference/renderer.md) | `Renderer`, `ScreenChrome`, `millis`, `request_update`: the framebuffer, the header and hints, the clock and a repaint |
| | [geometry](reference/geometry.md) | `Point`, `Size`, `Rect`, `Insets` |
| | [the backend contract](reference/backend-contract.md) | `Host`, `InputSource`, `Clock`, and installing a host and a navigator |
| | [canvas and chrome](reference/canvas-and-chrome.md) | `Canvas`, `TextMetrics`, `Chrome`, `RowField`: what a backend draws with |
| Testing | [testing](reference/testing.md) | `install`, `install_forced`, `TestHost`, `reset`, the counters and the metrics: the fake host |
| | [driving a screen](reference/driving.md) | `Ui`, `Drive`, `press`, `hold`, `release`, `next_frame`, `set_swipe`, `set_millis`, `set_has_left_right_keys`, `set_swipe_moves_selection`: driving a screen in a test |
| | [recording](reference/recording.md) | `Recorder`, `DrawOp` and snapshots: asserting what was drawn |

Every `rust` block on those pages is compiled and run by
`cargo test -p xpui --features testing --doc`, so a snippet that stops matching
the API fails CI rather than quietly teaching the wrong thing. Most install the
fake host first, because widgets resolve fonts and theme metrics through the
host in their constructors; those lines are hidden where they would only
clutter the prose.

The sections below are where this page used to hold that material. Each points
to its page now, so an old link still lands somewhere useful.

## A screen

In [screens](reference/screens.md), and the loop that drives one in
[app](reference/app.md).

## Layout

In [stacks](reference/stacks.md), [layout](reference/layout.md),
[modifiers](reference/modifiers.md) and [views](reference/views.md).

## Widgets

In [text and fonts](reference/text.md), [images](reference/images.md),
[controls](reference/controls.md), [steppers](reference/steppers.md),
[toggles](reference/toggles.md),
[lists](reference/lists.md), [dialogs](reference/dialogs.md) and
[indicators](reference/indicators.md).

## Interaction

In [input](reference/input.md) for buttons, swipes and claiming a key,
[key rows](reference/key-rows.md) for the keys along the bottom edge, and
[interactions](reference/interactions.md) for how a control declares where it
can be touched and focused, with [triggers](reference/triggers.md) for what it
produces when it fires. How a value control takes the keys over is in
[controls](reference/controls.md).

## Components

In [views](reference/views.md), under `ViewExt::map`.

## Screen roots

In [navigation](reference/navigation.md).

## Navigation

In [navigation](reference/navigation.md), and `App` in
[app](reference/app.md).

## Fonts

In [text and fonts](reference/text.md).

## The host façades

`Theme` is in [theme](reference/theme.md), `Renderer` and `ScreenChrome` in
[drawing and repainting](reference/renderer.md),
`Input` in [input](reference/input.md), and the traits behind them in
[the backend contract](reference/backend-contract.md) and
[canvas and chrome](reference/canvas-and-chrome.md).

## Geometry

In [geometry](reference/geometry.md).
