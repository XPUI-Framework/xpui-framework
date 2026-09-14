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
| | [layout](reference/layout.md) | `VStack`, `HStack`, `vstack!`, `hstack!`, `Spacer`, `Padding`, `ScrollView`, `Alignment`, `UNBOUNDED` |
| | [modifiers](reference/modifiers.md) | `Modifiers`, `Frame`, `Flexible`, `Tappable` |
| Components | [text and images](reference/text-and-images.md) | `Text`, `Font`, `FontId`, `FontRole`, `FontStyle`, `Image`, `Icon`, `IconRef` |
| | [controls](reference/controls.md) | `Slider`, `Stepper`, and how a value control takes the keys |
| | [toggles](reference/toggles.md) | `Toggle`, `IconToggle` |
| | [lists](reference/lists.md) | `List`, `ListRow`, `list!`, `Section`, `Divider` |
| | [dialogs](reference/dialogs.md) | `Modal`, `Scrim` |
| | [indicators](reference/indicators.md) | `ProgressBar` |
| Input | [input](reference/input.md) | `Button`, `SwipeDir`, `Input`, `KeyRow`, `RowKey`: how input reaches a screen |
| | [interactions](reference/interactions.md) | `InputMask`, `Interaction`, `Interactions`, `Trigger`, `value_at`: how a view declares where it can be touched |
| Host | [theme](reference/theme.md) | `Theme`, `ThemeMetric`, `Renderer`, `ScreenChrome`, `ControlState`, `millis`, `request_update` |
| | [geometry](reference/geometry.md) | `Point`, `Size`, `Rect`, `Insets` |
| | [the backend contract](reference/backend-contract.md) | `Host`, `InputSource`, `Clock`, and installing a host and a navigator |
| | [canvas and chrome](reference/canvas-and-chrome.md) | `Canvas`, `TextMetrics`, `Chrome`, `RowField`: what a backend draws with |
| Testing | [testing](reference/testing.md) | the fake host, `Ui` and `Drive`: driving a screen in a test |
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

In [layout](reference/layout.md), [modifiers](reference/modifiers.md) and
[views](reference/views.md).

## Widgets

In [text and images](reference/text-and-images.md),
[controls](reference/controls.md), [toggles](reference/toggles.md),
[lists](reference/lists.md), [dialogs](reference/dialogs.md) and
[indicators](reference/indicators.md).

## Interaction

In [input](reference/input.md) for buttons, swipes and claiming a key, and
[interactions](reference/interactions.md) for how a control declares where it
can be touched and focused. How a value control takes the keys over is in
[controls](reference/controls.md).

## Components

In [views](reference/views.md), under `ViewExt::map`.

## Screen roots

In [navigation](reference/navigation.md).

## Navigation

In [navigation](reference/navigation.md), and `App` in
[app](reference/app.md).

## Fonts

In [text and images](reference/text-and-images.md).

## The host façades

`Renderer`, `Theme` and `ScreenChrome` are in [theme](reference/theme.md),
`Input` in [input](reference/input.md), and the traits behind them in
[the backend contract](reference/backend-contract.md) and
[canvas and chrome](reference/canvas-and-chrome.md).

## Geometry

In [geometry](reference/geometry.md).
