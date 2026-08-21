//! Interaction regression tests.
//!
//! Widgets declare their interactive regions in a walk that mirrors `render`.
//! Nothing in the type system keeps those two in step, so these tests are what
//! catch a container whose implementations have drifted apart — the failure
//! mode being touches that land on the control next door.

use xpui::screen::{Driver, Runtime};
use xpui::testing::{self, MIN_TOUCH_SIZE, SCREEN_HEIGHT, SCREEN_WIDTH};
use xpui::{
    Alignment, Button, HStack, Hint, Input, InputMask, Interactions, List, ListRow, Modal,
    Modifiers, NavigationScreen, Point, Rect, ScrollView, Section, Size, Slider, Spacer, Stepper,
    SwipeDir, Text, Toggle, Trigger, VStack, View, ViewExt, hstack, value_at, vstack,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Msg {
    First,
    Second,
    Third,
    Value(i32),
    Step(i32),
    Toggled(bool),
}

fn screen() -> Size {
    Size::new(SCREEN_WIDTH, SCREEN_HEIGHT)
}

/// Lays a tree out and collects its interactions, as the runtime does.
fn collect(view: &mut dyn View<Msg>, focus: usize) -> Interactions<Msg> {
    xpui::testing::install();
    view.measure(screen());
    let mut out = Interactions::new(focus);
    view.interactions(Point::ORIGIN, &mut out);
    out
}

/// The message a touch at `point` produces, scanning back to front so the
/// innermost control wins — the same order the runtime uses.
fn touch(interactions: &Interactions<Msg>, point: Point, kind: InputMask) -> Option<Msg> {
    interactions
        .items()
        .iter()
        .rev()
        .find(|item| item.mask.contains(kind) && item.rect.contains(point))
        .map(|item| item.trigger.resolve(item.rect, point.x))
}

/// A touch must resolve through nested containers to the control actually under
/// it, not to whichever sibling was declared first.
#[test]
fn a_nested_tree_resolves_to_the_control_under_the_touch() {
    let mut tree: VStack<Msg> = vstack![10;
        Text::new("Heading"),
        hstack![8;
            Text::new("left").on_tap(Msg::First),
            Text::new("middle").on_tap(Msg::Second),
            Text::new("right").on_tap(Msg::Third),
        ],
    ];
    let interactions = collect(&mut tree, 0);

    let heading_height = {
        let mut heading = Text::new("Heading");
        View::<Msg>::measure(&mut heading, screen());
        View::<Msg>::size(&heading).height
    };
    let row_y = heading_height + 10;

    let mut cursor = 0;
    for (label, expected) in [
        ("left", Msg::First),
        ("middle", Msg::Second),
        ("right", Msg::Third),
    ] {
        let mut probe = Text::new(label);
        View::<Msg>::measure(&mut probe, screen());
        let width = View::<Msg>::size(&probe).width;
        let point = Point::new(cursor + width / 2, row_y + 2);

        assert_eq!(
            touch(&interactions, point, InputMask::TAP),
            Some(expected),
            "touch at {point:?} hit the wrong control"
        );
        cursor += width + 8;
    }
}

/// A touch on nothing interactive produces nothing, rather than the nearest
/// control — otherwise tapping a screen's background would fire something.
#[test]
fn a_miss_produces_nothing() {
    let mut tree: VStack<Msg> =
        vstack![10; Text::new("label"), Text::new("tap").on_tap(Msg::First)];
    let interactions = collect(&mut tree, 0);

    assert_eq!(
        touch(
            &interactions,
            Point::new(SCREEN_WIDTH - 1, SCREEN_HEIGHT - 1),
            InputMask::TAP
        ),
        None
    );
}

/// A view with no `on_tap` declares nothing at all.
#[test]
fn an_untagged_view_is_not_interactive() {
    let mut text = Text::new("just a label");
    let interactions = collect(&mut text, 0);
    assert!(interactions.is_empty());
}

/// Held frames must reach a slider and nothing else. This is the property that
/// stops a finger resting on a button re-firing it every tick.
#[test]
fn only_a_drag_control_accepts_held_frames() {
    let mut row: HStack<Msg> = hstack![8;
        Text::new("x").on_tap(Msg::First),
        Slider::new(50, 100).on_change(Msg::Value).flexible(),
    ];
    let interactions = collect(&mut row, 0);

    let button = interactions
        .items()
        .iter()
        .find(|item| matches!(item.trigger.resolve(item.rect, 0), Msg::First))
        .expect("the button should declare an interaction");
    assert!(
        !button.mask.contains(InputMask::DRAG),
        "a plain button must not receive held frames"
    );

    let slider = interactions
        .items()
        .iter()
        .find(|item| item.mask.contains(InputMask::DRAG))
        .expect("a slider must accept drags");
    assert!(slider.mask.contains(InputMask::TAP), "and taps too");
}

/// The rect a slider declares must be the one `value_at` was written against,
/// or a drag maps to the wrong percentage.
#[test]
fn a_slider_touch_converts_to_the_expected_value() {
    let mut slider = Slider::new(0, 100).on_change(Msg::Value);
    let interactions = collect(&mut slider, 0);

    let item = &interactions.items()[0];
    let rect = item.rect;

    assert_eq!(item.trigger.resolve(rect, rect.x()), Msg::Value(0));
    assert_eq!(
        item.trigger.resolve(rect, rect.x() + rect.width()),
        Msg::Value(100)
    );

    let middle = value_at(rect, rect.x() + rect.width() / 2, 100);
    assert!(
        (48..=52).contains(&middle),
        "the midpoint should read as about half, got {middle}"
    );
}

/// A list declares one interaction per interactive row, in reading order, so a
/// touch produces that row's own message.
#[test]
fn a_list_row_produces_its_own_message() {
    let mut list: List<Msg> = List::new()
        .push(ListRow::new("First").on_tap(Msg::First))
        .push(ListRow::new("Second").on_tap(Msg::Second))
        .push(ListRow::new("Third").on_tap(Msg::Third));
    let interactions = collect(&mut list, 0);

    assert_eq!(interactions.focusable_count(), 3);

    let height = View::<Msg>::size(&list).height / 3;
    for (index, expected) in [Msg::First, Msg::Second, Msg::Third]
        .into_iter()
        .enumerate()
    {
        let point = Point::new(20, index as i32 * height + height / 2);
        assert_eq!(touch(&interactions, point, InputMask::TAP), Some(expected));
    }
}

/// A read-out row carries no message, so it neither responds nor takes focus.
#[test]
fn a_row_without_a_message_is_inert() {
    let mut list: List<Msg> = List::new()
        .push(ListRow::new("read only"))
        .push(ListRow::new("tap me").on_tap(Msg::First));
    let interactions = collect(&mut list, 0);

    assert_eq!(interactions.focusable_count(), 1);
}

/// A glyph narrower than a fingertip must still be reachable: the hit area
/// grows without moving what is drawn.
#[test]
fn a_small_control_still_gets_a_full_size_touch_target() {
    let mut glyph = Text::new("-").on_tap(Msg::First);
    let interactions = collect(&mut glyph, 0);

    let declared = interactions.items()[0].rect;
    assert!(
        declared.width() >= MIN_TOUCH_SIZE,
        "a tiny control should be widened to {MIN_TOUCH_SIZE}, got {}",
        declared.width()
    );
    assert!(
        View::<Msg>::size(&glyph).width < MIN_TOUCH_SIZE,
        "and the drawn size must stay small"
    );
}

/// Focus follows tree order, and a widget learns its own state as it declares
/// itself — which is what lets a list highlight the focused row with no screen
/// code at all.
#[test]
fn focus_follows_tree_order() {
    let build = || -> List<Msg> {
        List::new()
            .push(ListRow::new("a").on_tap(Msg::First))
            .push(ListRow::new("b").on_tap(Msg::Second))
            .push(ListRow::new("c").on_tap(Msg::Third))
    };

    for focus in 0..3 {
        let mut list = build();
        let interactions = collect(&mut list, focus);
        assert_eq!(interactions.focusable_count(), 3);

        // The focused entry is the one Confirm would fire.
        let expected = [Msg::First, Msg::Second, Msg::Third][focus];
        let item = &interactions.items()[focus];
        assert_eq!(item.trigger.resolve(item.rect, 0), expected);
    }
}

/// A tap and Confirm on the same control must produce the identical message,
/// so touch and buttons are never subtly different.
#[test]
fn a_tap_and_confirm_agree() {
    let mut list: List<Msg> = List::new().push(ListRow::new("only").on_tap(Msg::Second));
    let interactions = collect(&mut list, 0);

    let item = &interactions.items()[0];
    let by_confirm = item.trigger.resolve(item.rect, 0);
    let centre = Point::new(
        item.rect.x() + item.rect.width() / 2,
        item.rect.y() + item.rect.height() / 2,
    );
    let by_touch = touch(&interactions, centre, InputMask::TAP).unwrap();

    assert_eq!(by_confirm, by_touch);
}

/// A toggle reports the state it is moving to, so a screen never writes `!x`.
#[test]
fn a_toggle_reports_the_next_state() {
    for current in [false, true] {
        let mut toggle = Toggle::new("Hyphenation", current, "On", "Off").on_change(Msg::Toggled);
        let interactions = collect(&mut toggle, 0);

        let item = &interactions.items()[0];
        assert_eq!(item.trigger.resolve(item.rect, 0), Msg::Toggled(!current));
    }
}

/// A stepper's end glyphs carry the sign, and its track carries the value.
#[test]
fn a_stepper_declares_steps_and_a_track() {
    let mut stepper = Stepper::new(50).on_change(Msg::Value).on_step(Msg::Step);
    let interactions = collect(&mut stepper, 0);

    let messages: Vec<Msg> = interactions
        .items()
        .iter()
        .map(|item| item.trigger.resolve(item.rect, item.rect.x()))
        .collect();

    assert!(messages.contains(&Msg::Step(-1)), "expected a minus step");
    assert!(messages.contains(&Msg::Step(1)), "expected a plus step");
    assert!(
        messages.iter().any(|m| matches!(m, Msg::Value(_))),
        "expected a draggable track"
    );
}

/// Centre alignment moves children across the axis; the interaction walk has to
/// apply the same offset `render` does, or touches miss by half the row.
#[test]
fn centre_alignment_moves_the_touch_area_with_the_drawing() {
    let mut row: HStack<Msg> = hstack![8;
        Slider::new(50, 100),
        Text::new("x").on_tap(Msg::First),
    ]
    .align(Alignment::Center);
    let interactions = collect(&mut row, 0);

    let declared = interactions.items()[0].rect;
    let row_height = View::<Msg>::size(&row).height;

    // Assert on the centre, not the edges: the glyph is shorter than the
    // minimum touch target, so its declared area is grown around it and may
    // legitimately start above the row. What centring guarantees is that the
    // two midpoints line up.
    let declared_centre = declared.y() + declared.height() / 2;
    let row_centre = row_height / 2;
    assert!(
        (declared_centre - row_centre).abs() <= 1,
        "a centred child's touch area should sit on the row's midline: \
         child centre {declared_centre}, row centre {row_centre}"
    );
}

/// A spacer declares nothing, so it can never swallow a touch meant for a
/// sibling.
#[test]
fn a_spacer_is_never_interactive() {
    let mut row: HStack<Msg> = hstack![8; Text::new("a"), Spacer::new(), Text::new("b")];
    let interactions = collect(&mut row, 0);
    assert!(interactions.is_empty());
}

/// A stepper is **one** focus stop, not three. Before this, Up/Down walked
/// through the `-` glyph, the track and the `+` glyph instead of moving between
/// settings — which made button navigation useless on the frontlight panel.
#[test]
fn a_stepper_is_a_single_focus_stop() {
    let mut stepper = Stepper::new(50).on_change(Msg::Value).on_step(Msg::Step);
    let interactions = collect(&mut stepper, 0);

    assert_eq!(
        interactions.focusable_count(),
        1,
        "a stepper should be one stop for buttons, however many touch targets it has"
    );
    assert!(
        interactions.items().len() > 1,
        "and it should still offer separate touch targets"
    );
}

/// Two steppers are two stops, so Up/Down move between settings.
#[test]
fn stacked_steppers_are_one_stop_each() {
    let mut panel: VStack<Msg> = vstack![10;
        Stepper::new(60).on_change(Msg::Value).on_step(Msg::Step),
        Stepper::new(40).on_change(Msg::Value).on_step(Msg::Step),
    ];
    let interactions = collect(&mut panel, 0);
    assert_eq!(interactions.focusable_count(), 2);
}

/// Left/Right nudge whatever holds focus, so one pair of keys drives every
/// adjustable control.
#[test]
fn the_focused_stepper_takes_the_nudge() {
    let mut stepper = Stepper::new(50).on_change(Msg::Value).on_step(Msg::Step);
    let interactions = collect(&mut stepper, 0);

    let adjustable = interactions
        .items()
        .iter()
        .find(|item| item.mask.contains(InputMask::ADJUST))
        .expect("a stepper must accept adjustment");

    assert_eq!(adjustable.trigger.resolve_step(-1), Some(Msg::Step(-1)));
    assert_eq!(adjustable.trigger.resolve_step(1), Some(Msg::Step(1)));
}

/// A touch-only control stays out of the focus order, so buttons never stop on
/// something they cannot activate.
#[test]
fn a_touch_only_control_is_not_focusable() {
    let mut row: HStack<Msg> = hstack![8;
        Text::new("icon").on_touch(Msg::First),
        Text::new("row").on_tap(Msg::Second),
    ];
    let interactions = collect(&mut row, 0);

    assert_eq!(interactions.focusable_count(), 1, "only the tappable row");
    assert_eq!(interactions.items().len(), 2, "but both still take a touch");
}

// -- swipe navigation ---------------------------------------------------------
//
// These drive the real `Runtime` rather than the routing functions, because the
// behaviour under test is the runtime's: a swipe with no control under it still
// has to move focus, exactly as the C++ home screen does.

/// A screen with three focusable rows, optionally claiming swipes itself.
struct Nav {
    claims_swipe: bool,
    claimed: Option<SwipeDir>,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum NavMsg {
    Row(u8),
    Swiped(SwipeDir),
}

impl xpui::Screen for Nav {
    type Message = NavMsg;

    fn body(&self) -> impl View<NavMsg> {
        vstack![4;
            Text::new("one").on_tap(NavMsg::Row(0)),
            Text::new("two").on_tap(NavMsg::Row(1)),
            Text::new("three").on_tap(NavMsg::Row(2)),
        ]
    }

    fn on_swipe(&self, direction: SwipeDir) -> Option<NavMsg> {
        self.claims_swipe.then_some(NavMsg::Swiped(direction))
    }

    fn update(&mut self, message: NavMsg) {
        if let NavMsg::Swiped(direction) = message {
            self.claimed = Some(direction);
        }
    }
}

fn runtime(claims_swipe: bool) -> Runtime<Nav> {
    testing::install();
    testing::reset();
    let mut runtime = Runtime::new(Nav {
        claims_swipe,
        claimed: None,
    });
    // Nothing routes before the first paint, so paint once.
    runtime.render();
    runtime
}

// -- editing a value in place -------------------------------------------------
//
// A board with four directions and no pair to spare cannot produce Left or
// Right, so Confirm opens an adjustable control and the keys that walk the list
// move the value instead. These drive the real `Runtime`, because the mode is
// entirely about which key means what when.

/// One adjustable control and one ordinary row beneath it.
struct Dial {
    value: i32,
    tapped: usize,
    /// How many messages the screen was handed — the count an edit that holds
    /// its value has to keep at one.
    dispatches: usize,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum DialMsg {
    Set(i32),
    Step(i32),
    Tapped,
}

impl xpui::Screen for Dial {
    type Message = DialMsg;

    fn body(&self) -> impl View<DialMsg> {
        vstack![10;
            Stepper::new(self.value).on_change(DialMsg::Set).on_step(DialMsg::Step),
            Text::new("a plain row").on_tap(DialMsg::Tapped),
        ]
    }

    fn update(&mut self, message: DialMsg) {
        self.dispatches += 1;
        match message {
            DialMsg::Set(value) => self.value = value,
            DialMsg::Step(delta) => self.value += delta,
            DialMsg::Tapped => self.tapped += 1,
        }
    }
}

/// What the panel says the value is, from the last frame it painted.
///
/// **Not the screen's field.** While an edit is open the framework holds the
/// value and the screen's has not moved, so this is the only place the working
/// copy is observable — which is the point: it is what a person sees.
fn painted_value(ops: &[xpui::testing::DrawOp]) -> i32 {
    // The last one in the log, so a caller that has driven several frames reads
    // the newest rather than the first ever painted.
    ops.iter()
        .rev()
        .find_map(|op| match op {
            xpui::testing::DrawOp::Slider { value, .. } => Some(*value),
            _ => None,
        })
        .expect("the screen draws a slider")
}

/// What state the panel painted the control in, from the last frame.
fn slider_state_of(ops: &[xpui::testing::DrawOp]) -> xpui::ControlState {
    ops.iter()
        .rev()
        .find_map(|op| match op {
            xpui::testing::DrawOp::Slider { state, .. } => Some(*state),
            _ => None,
        })
        .expect("the screen draws a slider")
}

/// The value on the panel after one more settled frame.
fn shown<S: xpui::Screen>(runtime: &mut Runtime<S>) -> i32 {
    runtime.render();
    painted_value(&testing::ops_log())
}

fn dial() -> Runtime<Dial> {
    testing::install();
    testing::reset();
    let mut runtime = Runtime::new(Dial {
        value: 50,
        tapped: 0,
        dispatches: 0,
    });
    runtime.render();
    runtime
}

/// A control reached through `map` keeps everything an editor needs.
///
/// `ViewExt::map` re-boxes a trigger so a sub-component can speak its own
/// message type, and every field has to survive the crossing: the reading an
/// edit opens on, the bounds a nudge clamps against, and the setter that puts
/// the value back. Drop the setter and the control silently stops being
/// editable — on a board with no Left/Right pair that is a row no key can
/// change, which is the same fault the unmapped path had.
///
/// Nothing in this repository maps a value control yet, so without this the
/// mapped half of four `Trigger` methods is unreached.
#[test]
fn a_mapped_control_can_still_be_nudged_and_committed() {
    testing::install();
    testing::reset();

    #[derive(Clone, Copy, PartialEq, Eq, Debug)]
    enum Outer {
        Inner(DialMsg),
    }

    let mut view = Stepper::new(40)
        .on_change(DialMsg::Set)
        .on_step(DialMsg::Step)
        .map(Outer::Inner);

    let mut out = Interactions::new(0);
    view.measure(screen());
    view.interactions(Point::ORIGIN, &mut out);

    let item = out
        .items()
        .iter()
        .find(|item| item.mask.contains(InputMask::ADJUST))
        .expect("a mapped stepper is still adjustable");

    assert_eq!(
        item.trigger.reading(),
        Some(40),
        "the reading has to cross, or an edit opens on nothing"
    );
    assert!(
        item.trigger.is_editable(),
        "and the setter has to cross, or an edit could never be committed"
    );
    assert_eq!(
        item.trigger.resolve_step(1),
        Some(Outer::Inner(DialMsg::Step(1))),
        "a nudge arrives as the outer message"
    );
    assert_eq!(
        item.trigger.set_to(25),
        Some(Outer::Inner(DialMsg::Set(25))),
        "and committing sets the value outright, through the conversion"
    );
    assert_eq!(
        item.trigger.stepped(100, 1),
        Some(100),
        "the bound has to cross too, or an open edit runs off the end of the track"
    );
}

/// A slider nudged by a key stops at its own ends.
///
/// An absolute control is nudged by resolving `value + delta`, so without a
/// clamp Left at zero sends `-1` and Right at the maximum sends `max + 1`. A
/// screen that clamps hides it; one that does not stores a value outside the
/// range it declared, and the track then draws past its own end.
///
/// Asserted at both ends, because a clamp with one bound is the easier mistake.
#[test]
fn nudging_a_slider_stops_at_its_ends() {
    testing::install();
    testing::reset();

    let at = |value: i32, delta: i32| {
        let mut slider = Slider::new(value, 100).on_change(DialMsg::Set);
        let mut out = Interactions::new(0);
        slider.measure(screen());
        slider.interactions(Point::ORIGIN, &mut out);
        out.items()
            .iter()
            .find(|item| item.mask.contains(InputMask::ADJUST))
            .expect("a slider is adjustable")
            .trigger
            .resolve_step(delta)
    };

    assert_eq!(
        at(0, -1),
        Some(DialMsg::Set(0)),
        "Left at zero stays at zero"
    );
    assert_eq!(at(0, 1), Some(DialMsg::Set(1)), "and Right still moves");
    assert_eq!(
        at(100, 1),
        Some(DialMsg::Set(100)),
        "Right at the maximum stays at the maximum"
    );
    assert_eq!(at(100, -1), Some(DialMsg::Set(99)), "and Left still moves");
}

/// The same control on a screen that refuses what it cannot hold.
///
/// Clamping is the ordinary case — every value row in the gallery does it — and
/// it is what broke cancel: a screen at its maximum takes `+1` and stays put,
/// so a framework counting the steps it *sent* believes in a move the value
/// never made.
struct ClampedDial {
    value: i32,
    dispatches: usize,
}

impl xpui::Screen for ClampedDial {
    type Message = DialMsg;

    fn body(&self) -> impl View<DialMsg> {
        vstack![10;
            Stepper::new(self.value).on_change(DialMsg::Set).on_step(DialMsg::Step),
        ]
    }

    fn update(&mut self, message: DialMsg) {
        self.dispatches += 1;
        match message {
            DialMsg::Set(value) => self.value = value.clamp(0, 100),
            DialMsg::Step(delta) => self.value = (self.value + delta).clamp(0, 100),
            DialMsg::Tapped => {}
        }
    }
}

/// Opening a value changes the frame, and changes it differently from focus.
///
/// The whole complaint against spec 26's mode was that entering it repainted an
/// identical frame — on a Badger, a second of the panel's life spent saying
/// nothing. Three states, three pictures: idle, focused, open. If any two match,
/// the mode is invisible and the refresh that entered it bought nothing.
#[test]
fn the_three_states_of_a_value_row_look_different() {
    use xpui::testing::DrawOp;

    let slider_state = |ops: &[DrawOp]| {
        ops.iter()
            .find_map(|op| match op {
                DrawOp::Slider { state, .. } => Some(*state),
                _ => None,
            })
            .expect("the screen draws a slider")
    };

    testing::install();
    testing::reset();
    testing::set_has_left_right_keys(false);

    // One value control and one plain row, so the keys have somewhere else to
    // be and the control has a state to change out of.
    let mut runtime = Runtime::new(Dial {
        value: 50,
        tapped: 0,
        dispatches: 0,
    });
    runtime.render();
    let focused = testing::ops_log();

    testing::press(Button::Confirm);
    runtime.loop_();
    testing::reset();
    runtime.render();
    let editing = testing::ops_log();

    assert_eq!(
        slider_state(&focused),
        xpui::ControlState::Focused,
        "the keys are on this row, and it says so"
    );
    assert_eq!(
        slider_state(&editing),
        xpui::ControlState::Editing,
        "and it says something else once the keys are moving the value"
    );
    assert_ne!(
        testing::render(&focused),
        testing::render(&editing),
        "entering the mode must change the frame — an identical repaint is a \
         second of a slow panel's life spent saying nothing"
    );

    // And leaving it. Cancel is the harder direction: it dispatches nothing at
    // all, so the *only* thing that can differ is what the mode itself paints.
    // If this frame matched the one before it, the refresh that closed the mode
    // bought nothing either.
    testing::press(Button::Back);
    runtime.loop_();
    testing::reset();
    runtime.render();
    let closed = testing::ops_log();

    assert_ne!(
        testing::render(&editing),
        testing::render(&closed),
        "and leaving it must change the frame back"
    );
    assert_eq!(
        slider_state(&closed),
        xpui::ControlState::Focused,
        "to the one the keys were on before it opened"
    );
}

/// A control's own number follows the keys while an edit is open.
///
/// The whole reason the readout is the control's rather than the screen's. A
/// screen builds its labels in `update`, and `update` is not called while an
/// edit is open — deliberately, since the framework is holding the value — so a
/// number the screen painted stands still while the track moves. Read here from
/// what was actually painted, because the screen's own field is exactly the
/// thing that must *not* have moved.
#[test]
fn a_readout_shows_the_value_the_keys_are_moving() {
    use xpui::testing::DrawOp;

    /// Every string the frame painted, newest frame last.
    fn texts(ops: &[DrawOp]) -> Vec<String> {
        ops.iter()
            .filter_map(|op| match op {
                DrawOp::Text { text, .. } => Some(text.clone()),
                _ => None,
            })
            .collect()
    }

    testing::install();
    testing::reset();
    testing::set_has_left_right_keys(false);

    struct Dialled {
        value: i32,
        dispatches: usize,
    }
    impl xpui::Screen for Dialled {
        type Message = DialMsg;
        fn body(&self) -> impl View<DialMsg> {
            vstack![10;
                Slider::new(self.value, 100)
                    .on_change(DialMsg::Set)
                    .title("Warmth")
                    .readout("%"),
            ]
        }
        fn update(&mut self, message: DialMsg) {
            self.dispatches += 1;
            if let DialMsg::Set(value) = message {
                self.value = value;
            }
        }
    }

    let mut runtime = Runtime::new(Dialled {
        value: 25,
        dispatches: 0,
    });
    runtime.render();
    assert!(
        texts(&testing::ops_log()).contains(&"25%".to_string()),
        "the control draws its own number before anything is opened"
    );

    testing::press(Button::Confirm);
    runtime.loop_();
    for _ in 0..3 {
        testing::press(Button::Up);
        runtime.loop_();
    }
    testing::reset();
    runtime.render();

    let painted = texts(&testing::ops_log());
    assert!(
        painted.contains(&"28%".to_string()),
        "three Ups have to reach the number, not only the knob: {painted:?}"
    );
    assert_eq!(
        runtime.screen().value,
        25,
        "and the screen is still holding what it started with"
    );
    assert_eq!(runtime.screen().dispatches, 0);

    // Cancel: the panel goes back to the screen's value, in one step.
    testing::press(Button::Back);
    runtime.loop_();
    testing::reset();
    runtime.render();
    assert!(
        texts(&testing::ops_log()).contains(&"25%".to_string()),
        "cancelling puts the number back with the knob"
    );
}

/// A control with nothing to choose between still says what it is.
///
/// `Slider::new`'s own doc calls a zero `max` an anticipated input — a screen
/// whose range comes from data has one the day the data holds a single item.
/// `measure` reserves the header line either way, so a `render` that gave up
/// before drawing it left an unexplained gap where the name and the reading
/// should be.
#[test]
fn an_empty_range_keeps_its_name_and_its_number() {
    use xpui::testing::DrawOp;

    for max in [0, -1] {
        testing::install();
        testing::reset();

        let mut view: Slider<DialMsg> = Slider::new(0, max).title("Volume").readout("%");
        view.measure(screen());
        view.render(Point::ORIGIN);

        let painted: Vec<String> = testing::ops_log()
            .iter()
            .filter_map(|op| match op {
                DrawOp::Text { text, .. } => Some(text.clone()),
                _ => None,
            })
            .collect();
        assert!(
            painted.contains(&"Volume".to_string()),
            "max={max}: the name is drawn whatever the range: {painted:?}"
        );
        assert!(
            painted.contains(&"0%".to_string()),
            "max={max}: and so is the reading: {painted:?}"
        );
        assert!(
            !testing::ops_log()
                .iter()
                .any(|op| matches!(op, DrawOp::Slider { .. })),
            "max={max}: but there is no track to draw"
        );
    }
}

/// A name with no number, and a number with no name, each draw only their own.
///
/// Every other test here passes both, so a header that drew the wrong one — or
/// reserved a line for a control that asked for neither — would go unseen.
#[test]
fn a_header_draws_only_what_it_was_given() {
    use xpui::testing::DrawOp;

    fn painted(build: impl FnOnce() -> Slider<DialMsg>) -> (Vec<String>, i32) {
        testing::install();
        testing::reset();
        let mut view = build();
        view.measure(screen());
        view.render(Point::ORIGIN);
        let texts = testing::ops_log()
            .iter()
            .filter_map(|op| match op {
                DrawOp::Text { text, .. } => Some(text.clone()),
                _ => None,
            })
            .collect();
        (texts, View::<DialMsg>::size(&view).height)
    }

    let (bare, bare_height) = painted(|| Slider::new(30, 100).on_change(DialMsg::Set));
    assert!(
        bare.is_empty(),
        "no name and no number means no line: {bare:?}"
    );

    let (named, named_height) =
        painted(|| Slider::new(30, 100).on_change(DialMsg::Set).title("Volume"));
    assert_eq!(named, vec!["Volume".to_string()], "a name and no number");

    let (numbered, _) = painted(|| Slider::new(30, 100).on_change(DialMsg::Set).readout("%"));
    assert_eq!(numbered, vec!["30%".to_string()], "a number and no name");

    assert!(
        named_height > bare_height,
        "and a control that asked for neither is not made taller for a line it \
         does not draw"
    );
}

/// A stepper's glyphs take a finger where they are drawn.
///
/// The row is painted at one origin and declared at another, and nothing but
/// this ties the two together. They are now offset by a header line, so a drift
/// is a whole line high rather than a few pixels: `-` stops responding and the
/// name above it starts nudging the value down. This repository has shipped
/// that fault once at 6px, with a test too loose to see it — so what is pinned
/// here is the *relationship*, each glyph's declared rect against the rect it
/// painted into, rather than either number alone.
#[test]
fn a_steppers_glyphs_are_touchable_where_they_are_drawn() {
    use xpui::testing::DrawOp;

    testing::install();
    testing::reset();

    let mut view: Stepper<DialMsg> = Stepper::new(50)
        .on_change(DialMsg::Set)
        .on_step(DialMsg::Step)
        .title("Brightness")
        .readout("%");
    view.measure(screen());

    let mut out = Interactions::new(0);
    view.interactions(Point::ORIGIN, &mut out);
    view.render(Point::ORIGIN);

    // Where the two glyphs were painted.
    let glyphs: Vec<(i32, i32)> = testing::ops_log()
        .iter()
        .filter_map(|op| match op {
            DrawOp::Text { origin, text, .. } if text == "-" || text == "+" => {
                Some((origin.x, origin.y))
            }
            _ => None,
        })
        .collect();
    assert_eq!(glyphs.len(), 2, "a stepper draws both glyphs");

    // Every painted glyph has to sit inside a region that takes a tap.
    for (x, y) in glyphs {
        let hit = out
            .items()
            .iter()
            .any(|item| item.mask.contains(InputMask::TAP) && item.rect.contains(Point::new(x, y)));
        assert!(
            hit,
            "a glyph painted at ({x},{y}) is outside every touch region — the row \
             was declared somewhere other than where it drew"
        );
    }

    // And the name above them is not one of those regions, or tapping the
    // label would nudge the value.
    let title = testing::ops_log()
        .iter()
        .find_map(|op| match op {
            DrawOp::Text { origin, text, .. } if text == "Brightness" => Some(*origin),
            _ => None,
        })
        .expect("the stepper draws its own name");
    assert!(
        !out.items()
            .iter()
            .any(|item| item.mask.contains(InputMask::TAP) && item.rect.contains(title)),
        "the header line must take no tap"
    );
}

/// A stepper's number and its own track never disagree.
///
/// A `Stepper` is a composite: the header carrying the number is drawn by the
/// stepper, and the track under it by an embedded `Slider` that learns the open
/// edit for itself. So there are **two** places that have to follow the working
/// copy, and only one of them is the one a slider test would cover. If the
/// header kept the screen's value, a person would watch the knob move under a
/// number that did not — which is the exact fault this readout exists to fix,
/// reintroduced one widget along.
#[test]
fn a_steppers_number_agrees_with_its_track() {
    use xpui::testing::DrawOp;

    testing::install();
    testing::reset();
    testing::set_has_left_right_keys(false);

    struct Stepped {
        value: i32,
    }
    impl xpui::Screen for Stepped {
        type Message = DialMsg;
        fn body(&self) -> impl View<DialMsg> {
            vstack![10;
                Stepper::new(self.value)
                    .on_change(DialMsg::Set)
                    .on_step(DialMsg::Step)
                    .title("Brightness")
                    .readout("%"),
            ]
        }
        fn update(&mut self, _message: DialMsg) {}
    }

    let mut runtime = Runtime::new(Stepped { value: 60 });
    runtime.render();

    testing::press(Button::Confirm);
    runtime.loop_();
    for _ in 0..4 {
        testing::press(Button::Up);
        runtime.loop_();
    }
    testing::reset();
    runtime.render();

    let ops = testing::ops_log();
    let track = painted_value(&ops);
    let number = ops
        .iter()
        .rev()
        .find_map(|op| match op {
            DrawOp::Text { text, .. } if text.ends_with('%') => Some(text.clone()),
            _ => None,
        })
        .expect("the stepper draws its own number");

    assert_eq!(track, 64, "four Ups move the track");
    assert_eq!(
        number,
        format!("{track}%"),
        "and the number over it has to say the same thing"
    );
}

/// The hint bar names the mode, and names it with the board's own words.
///
/// The runtime owns the edit and the screen paints the bar, so this is the one
/// path that has to carry a fact from one to the other without letting a screen
/// read it. What is asserted here is the vocabulary reaching the chrome —
/// `Hint::Edit`, `Hint::Done`, `Hint::Cancel` — because which *word* each of
/// those becomes is the board's business and is asserted where the board is.
#[test]
fn the_hint_bar_offers_edit_then_done_and_cancel() {
    use xpui::testing::DrawOp;

    /// The bar's Back and Confirm slots, as the chrome was told to paint them.
    fn slots(ops: &[DrawOp]) -> (Option<String>, Option<String>) {
        ops.iter()
            .rev()
            .find_map(|op| match op {
                DrawOp::Hints(slots) => Some((slots[0].clone(), slots[1].clone())),
                _ => None,
            })
            .expect("the screen paints a hint bar")
    }

    struct Bar {
        value: i32,
    }
    impl xpui::Screen for Bar {
        type Message = DialMsg;
        fn body(&self) -> impl View<DialMsg> {
            NavigationScreen::new(vstack![10;
                Stepper::new(self.value).on_change(DialMsg::Set).on_step(DialMsg::Step),
            ])
            .title("Light")
            .hints(Hint::Standard, Hint::text("Save"), Hint::None, Hint::None)
        }
        fn update(&mut self, message: DialMsg) {
            if let DialMsg::Set(value) = message {
                self.value = value;
            }
        }
    }

    testing::install();
    testing::reset();
    testing::set_has_left_right_keys(false);

    let mut runtime = Runtime::new(Bar { value: 50 });
    runtime.render();
    assert_eq!(
        slots(&testing::ops_log()),
        (None, Some("<edit>".to_string())),
        "focused on something openable, Confirm offers to open it — over the \
         screen's own \"Save\", which it does not get to keep"
    );

    testing::press(Button::Confirm);
    runtime.loop_();
    runtime.render();
    assert_eq!(
        slots(&testing::ops_log()),
        (Some("<cancel>".to_string()), Some("<done>".to_string())),
        "and with it open, both keys say what they now do"
    );

    testing::press(Button::Back);
    runtime.loop_();
    runtime.render();
    assert_eq!(
        slots(&testing::ops_log()),
        (None, Some("<edit>".to_string())),
        "closing puts the offer back"
    );
}

/// A board with a Left/Right pair is never offered the mode, so never the words.
///
/// Confirm does not open an edit there — the pair nudges the value in place —
/// and a bar promising Edit would name a key that does nothing.
#[test]
fn a_board_with_the_pair_is_never_offered_the_mode() {
    use xpui::testing::DrawOp;

    struct Bar {
        value: i32,
    }
    impl xpui::Screen for Bar {
        type Message = DialMsg;
        fn body(&self) -> impl View<DialMsg> {
            NavigationScreen::new(vstack![10;
                Stepper::new(self.value).on_change(DialMsg::Set).on_step(DialMsg::Step),
            ])
            .hints(Hint::Standard, Hint::text("Save"), Hint::None, Hint::None)
        }
        fn update(&mut self, _message: DialMsg) {}
    }

    testing::install();
    testing::reset();
    testing::set_has_left_right_keys(true);

    let mut runtime = Runtime::new(Bar { value: 50 });
    runtime.render();

    let confirm = testing::ops_log()
        .iter()
        .rev()
        .find_map(|op| match op {
            DrawOp::Hints(slots) => Some(slots[1].clone()),
            _ => None,
        })
        .expect("the screen paints a hint bar");
    assert_eq!(
        confirm,
        Some("Save".to_string()),
        "the screen's own word stands where the mode never opens"
    );
}

/// "The control wrapping you holds focus" crosses `map` as well.
///
/// A composite that owns one focus stop and embeds a track that takes none
/// tells the track through `set_parent_focused` — that is how `Stepper` works,
/// and the mechanism is `pub` so anything else can. Nothing in this repository
/// puts a `map` between the two, so this asks the collector directly rather
/// than through a screen: the contract is that a child sees what its parent
/// was told, and a child that did not would paint an embedded track unfocused
/// inside a focused control.
#[test]
fn a_mapped_child_inherits_the_focus_of_the_control_wrapping_it() {
    #[derive(Clone, Copy, PartialEq, Eq, Debug)]
    enum Outer {
        Inner(DialMsg),
    }

    testing::install();
    testing::reset();

    let mut view = Slider::new(20, 100)
        .on_change(DialMsg::Set)
        .without_focus()
        .map(Outer::Inner);
    view.measure(screen());

    let mut out: Interactions<Outer> = Interactions::new(0);
    out.set_parent_focused(true);
    view.interactions(Point::ORIGIN, &mut out);
    view.render(Point::ORIGIN);
    assert_eq!(
        slider_state_of(&testing::ops_log()),
        xpui::ControlState::Focused,
        "the track is inside a focused control, mapping or no mapping"
    );

    testing::reset();
    let mut out: Interactions<Outer> = Interactions::new(0);
    out.set_parent_focused(false);
    view.interactions(Point::ORIGIN, &mut out);
    view.render(Point::ORIGIN);
    assert_eq!(
        slider_state_of(&testing::ops_log()),
        xpui::ControlState::Idle,
        "and unfocused when it is not"
    );
}

/// A value control behind `map` gets the whole mode, not half of it.
///
/// `Interactions::child` builds the collector a sub-component declares into. It
/// used to build a bare one, so a `Slider` inside a mapped component learned
/// neither that an edit was open nor what the working value was: the knob stood
/// still while the keys moved a copy it could not see, the hint bar promised
/// Cancel and Done because the runtime reads its own state, and Confirm then
/// jumped the screen to a number the panel had never shown. That is exactly the
/// invisible mode this whole mechanism exists to remove, reintroduced for every
/// component that speaks its own message type.
#[test]
fn a_mapped_value_control_shows_the_open_edit() {
    testing::install();
    testing::reset();
    testing::set_has_left_right_keys(false);

    #[derive(Clone, Copy, PartialEq, Eq, Debug)]
    enum Outer {
        Inner(DialMsg),
    }

    struct Wrapped {
        value: i32,
        dispatches: usize,
    }
    impl xpui::Screen for Wrapped {
        type Message = Outer;
        fn body(&self) -> impl View<Outer> {
            vstack![10;
                Stepper::new(self.value)
                    .on_change(DialMsg::Set)
                    .on_step(DialMsg::Step)
                    .map(Outer::Inner),
            ]
        }
        fn update(&mut self, message: Outer) {
            self.dispatches += 1;
            let Outer::Inner(inner) = message;
            if let DialMsg::Set(value) = inner {
                self.value = value;
            }
        }
    }

    let mut runtime = Runtime::new(Wrapped {
        value: 50,
        dispatches: 0,
    });
    runtime.render();

    testing::press(Button::Confirm);
    runtime.loop_();
    runtime.render();

    for _ in 0..4 {
        testing::press(Button::Up);
        runtime.loop_();
        runtime.render();
    }

    let ops = testing::ops_log();
    assert_eq!(
        painted_value(&ops),
        54,
        "the knob has to follow the keys through the mapping too"
    );
    assert_eq!(
        slider_state_of(&ops),
        xpui::ControlState::Editing,
        "and the control has to look open, or the bar promises a mode nothing shows"
    );
    assert_eq!(runtime.screen().value, 50, "with the screen still untold");

    testing::press(Button::Confirm);
    runtime.loop_();
    assert_eq!(
        runtime.screen().value,
        54,
        "and Confirm commits what was shown"
    );
    assert_eq!(runtime.screen().dispatches, 1);
}

/// A mapped control does not paint itself focused when the keys are elsewhere.
///
/// `child` used to hand a sub-component `focus.saturating_sub(focusable)`,
/// which answers `0` when the focus is *behind* the subtree — so the first
/// control inside a mapped component believed it held a focus sitting on a row
/// above it. Two things looked selected and one was. Invisible while a focused
/// slider looked like an unfocused one; a wrong highlight now that it does not.
#[test]
fn a_mapped_control_is_not_focused_when_the_focus_is_above_it() {
    testing::install();
    testing::reset();

    #[derive(Clone, Copy, PartialEq, Eq, Debug)]
    enum Outer {
        Inner(DialMsg),
    }

    struct Above {
        value: i32,
    }
    impl xpui::Screen for Above {
        type Message = Outer;
        fn body(&self) -> impl View<Outer> {
            vstack![10;
                Text::new("a plain row").on_tap(Outer::Inner(DialMsg::Tapped)),
                Stepper::new(self.value)
                    .on_change(DialMsg::Set)
                    .on_step(DialMsg::Step)
                    .map(Outer::Inner),
            ]
        }
        fn update(&mut self, _message: Outer) {}
    }

    let mut runtime = Runtime::new(Above { value: 50 });
    runtime.render();

    assert_eq!(runtime.focused_index(), 0, "the keys are on the plain row");
    assert_eq!(
        slider_state_of(&testing::ops_log()),
        xpui::ControlState::Idle,
        "so the control below it must not look selected"
    );

    // And it does light up when the focus reaches it, so the fix is not simply
    // "a mapped control is never focused".
    testing::press(Button::Down);
    runtime.loop_();
    testing::reset();
    runtime.render();
    assert_eq!(
        slider_state_of(&testing::ops_log()),
        xpui::ControlState::Focused,
        "and it must light up when the keys arrive"
    );
}

/// A control with no room to move does not bring the device down.
///
/// `i32::clamp` panics when its low bound exceeds its high, which on a device is
/// an abort rather than a message. Everywhere else in the framework a
/// non-positive `max` is empty and harmless — `Slider::render` and the chrome's
/// `draw_slider` both decline to paint one — so the edit path has to agree
/// rather than fault. A list-backed control sized `len - 1` reaches zero the
/// day the list is empty, and negative the day it is built from a bad count.
#[test]
fn an_empty_control_can_be_opened_without_panicking() {
    for max in [-1, 0] {
        testing::install();
        testing::reset();
        testing::set_has_left_right_keys(false);

        struct Empty {
            max: i32,
        }
        impl xpui::Screen for Empty {
            type Message = DialMsg;
            fn body(&self) -> impl View<DialMsg> {
                vstack![10; Slider::new(0, self.max).on_change(DialMsg::Set)]
            }
            fn update(&mut self, _message: DialMsg) {}
        }

        let mut runtime = Runtime::new(Empty { max });
        runtime.render();

        testing::press(Button::Confirm);
        runtime.loop_();
        runtime.render();
        testing::press(Button::Up);
        runtime.loop_();
        runtime.render();
        testing::press(Button::Confirm);
        runtime.loop_();
        runtime.render();
    }
}

/// A composite puts "my control holds the keys" back when it is done.
///
/// `Stepper` sets the flag around its own row so the track inside learns it,
/// and has to clear it afterwards: the collector is one object walked over the
/// whole tree, so a flag left set makes **everything declared after** the
/// stepper believe it sits inside a focused control. A `Slider::without_focus`
/// further down the screen then paints itself selected while the keys are
/// elsewhere.
#[test]
fn a_composite_does_not_leave_its_focus_flag_set_behind_it() {
    testing::install();
    testing::reset();

    struct After {
        value: i32,
        trailing: i32,
    }
    impl xpui::Screen for After {
        type Message = DialMsg;
        fn body(&self) -> impl View<DialMsg> {
            vstack![10;
                Stepper::new(self.value).on_change(DialMsg::Set).on_step(DialMsg::Step),
                // A bare track with no stop of its own, outside the stepper.
                Slider::new(self.trailing, 100).on_change(DialMsg::Set).without_focus(),
            ]
        }
        fn update(&mut self, _message: DialMsg) {}
    }

    let mut runtime = Runtime::new(After {
        value: 50,
        trailing: 20,
    });
    runtime.render();
    assert_eq!(runtime.focused_index(), 0, "the keys are on the stepper");

    let painted: Vec<(i32, xpui::ControlState)> = testing::ops_log()
        .iter()
        .filter_map(|op| match op {
            xpui::testing::DrawOp::Slider { value, state, .. } => Some((*value, *state)),
            _ => None,
        })
        .collect();
    assert_eq!(
        painted,
        vec![
            (50, xpui::ControlState::Focused),
            (20, xpui::ControlState::Idle),
        ],
        "the stepper's own track is selected; the one after it is not"
    );
}

/// An open edit moves **its own** control and no other.
///
/// The working value reaches every widget through the collector, so a `Slider`
/// that painted it without first checking that it holds the focus would paint
/// *every* slider on the screen at the edited value. On the gallery's Controls
/// screen that is Brightness sliding from 60 to 27 while Warmth is being
/// edited — two controls moving for one set of keys, and nothing red.
#[test]
fn an_open_edit_leaves_the_other_controls_alone() {
    testing::install();
    testing::reset();
    testing::set_has_left_right_keys(false);

    struct Two {
        first: i32,
        second: i32,
    }
    impl xpui::Screen for Two {
        type Message = DialMsg;
        fn body(&self) -> impl View<DialMsg> {
            vstack![10;
                Slider::new(self.first, 100).on_change(DialMsg::Set),
                Slider::new(self.second, 100).on_change(DialMsg::Set),
            ]
        }
        fn update(&mut self, _message: DialMsg) {}
    }

    let mut runtime = Runtime::new(Two {
        first: 60,
        second: 25,
    });
    runtime.render();

    // Down to the second slider, open it, and move it well away from the first.
    testing::press(Button::Down);
    runtime.loop_();
    testing::press(Button::Confirm);
    runtime.loop_();
    for _ in 0..4 {
        testing::press(Button::Up);
        runtime.loop_();
    }
    testing::reset();
    runtime.render();

    let painted: Vec<i32> = testing::ops_log()
        .iter()
        .filter_map(|op| match op {
            xpui::testing::DrawOp::Slider { value, .. } => Some(*value),
            _ => None,
        })
        .collect();
    assert_eq!(
        painted,
        vec![60, 29],
        "only the control the keys are on moves; the other keeps the screen's value"
    );
}

/// Left and Right move an open edit too, where a host sends them anyway.
///
/// A board with the pair never opens an edit, so these keys should be
/// unreachable here — except that a host is free to send them while answering
/// that it has no pair, which the simulator's keyboard does on every board.
/// Dropping the arm makes the arrow keys dead inside the mode, and the human
/// check for this spec is done by clicking the drawn keys rather than typing,
/// so nothing would find it.
#[test]
fn arrow_keys_move_an_open_edit_when_a_host_sends_them() {
    let mut runtime = dial();
    tap(&mut runtime, Button::Confirm, 10);

    tap(&mut runtime, Button::Right, 20);
    tap(&mut runtime, Button::Right, 30);
    assert_eq!(
        painted_value(&testing::ops_log()),
        52,
        "Right raises inside an open edit"
    );

    tap(&mut runtime, Button::Left, 40);
    assert_eq!(painted_value(&testing::ops_log()), 51, "and Left lowers it");
    assert_eq!(
        runtime.screen().dispatches,
        0,
        "through the same held copy as every other key"
    );
}

/// A key that cannot move the value further spends no refresh.
///
/// Against the end of the track Up has nothing to do, and asking for a repaint
/// is a second of a slow panel's life spent painting the frame that is already
/// there. Step 6 of the spec, at the one place it is easy to lose.
#[test]
fn a_key_against_the_end_of_the_track_asks_for_no_repaint() {
    testing::install();
    testing::reset();
    testing::set_has_left_right_keys(false);

    let mut runtime = Runtime::new(ClampedDial {
        value: 100,
        dispatches: 0,
    });
    runtime.render();

    testing::press(Button::Confirm);
    runtime.loop_();
    runtime.render();

    let before = testing::updates();
    testing::press(Button::Up);
    runtime.loop_();
    assert_eq!(
        testing::updates(),
        before,
        "already at the maximum: nothing moved, so nothing is repainted"
    );

    // And a key that *can* move it still asks, so this is not simply "Up never
    // repaints".
    testing::press(Button::Down);
    runtime.loop_();
    assert!(
        testing::updates() > before,
        "a key that moves the value still asks for its frame"
    );
}

/// The bar offers Edit only where Confirm would really open something.
///
/// A `Stepper` with no `on_change` can be nudged and never opened — there is no
/// absolute setter, so Confirm has nothing to commit. Promising Edit over a key
/// that does nothing is worse than promising nothing. The runtime half of this
/// is covered by `a_control_that_cannot_be_committed_is_not_editable`; this is the
/// bar.
#[test]
fn the_bar_offers_no_edit_on_a_control_that_cannot_be_opened() {
    use xpui::testing::DrawOp;

    testing::install();
    testing::reset();
    testing::set_has_left_right_keys(false);

    struct NudgeOnly {
        value: i32,
    }
    impl xpui::Screen for NudgeOnly {
        type Message = DialMsg;
        fn body(&self) -> impl View<DialMsg> {
            NavigationScreen::new(vstack![10;
                Stepper::new(self.value).on_step(DialMsg::Step),
            ])
            .hints(Hint::Standard, Hint::text("Save"), Hint::None, Hint::None)
        }
        fn update(&mut self, _message: DialMsg) {}
    }

    let mut runtime = Runtime::new(NudgeOnly { value: 10 });
    runtime.render();

    let confirm = testing::ops_log()
        .iter()
        .rev()
        .find_map(|op| match op {
            DrawOp::Hints(slots) => Some(slots[1].clone()),
            _ => None,
        })
        .expect("the screen paints a hint bar");
    assert_eq!(
        confirm,
        Some("Save".to_string()),
        "the screen's own word stands where Confirm cannot open anything"
    );
}

/// A swipe cannot walk the focus out from under an open edit.
///
/// Touch and swipe are resolved before the keys and know nothing about the
/// edit, so a swipe used to move the highlight while the keys carried on
/// driving the control it left — the value and the thing that looks selected
/// disagreeing, with no way back to noticing it.
#[test]
fn a_swipe_is_declined_while_a_value_is_open() {
    testing::install();
    testing::reset();
    testing::set_has_left_right_keys(false);

    let mut runtime = Runtime::new(Dial {
        value: 50,
        tapped: 0,
        dispatches: 0,
    });
    runtime.render();

    testing::press(Button::Confirm);
    runtime.loop_();
    runtime.render();
    let focus = runtime.focused_index();

    testing::set_swipe(SwipeDir::Up);
    runtime.loop_();
    runtime.render();

    assert_eq!(
        runtime.focused_index(),
        focus,
        "the swipe must not move the highlight while a value is open"
    );

    // And the keys still reach the value they opened on.
    testing::press(Button::Up);
    runtime.loop_();
    assert_eq!(
        shown(&mut runtime),
        51,
        "the edit still owns the keys after the declined swipe"
    );
}

/// One nudge, five units — what a frontlight row does.
///
/// The other way a cancel computed from steps goes wrong, and the one spec 26
/// got right: a screen that scales reads an inverse total of `-4` as `-20`.
struct ScaledDial {
    value: i32,
    dispatches: usize,
}

impl xpui::Screen for ScaledDial {
    type Message = DialMsg;

    fn body(&self) -> impl View<DialMsg> {
        vstack![10;
            Stepper::new(self.value).on_change(DialMsg::Set).on_step(DialMsg::Step),
        ]
    }

    fn update(&mut self, message: DialMsg) {
        self.dispatches += 1;
        match message {
            DialMsg::Set(value) => self.value = value.clamp(0, 100),
            DialMsg::Step(delta) => self.value = (self.value + delta * 5).clamp(0, 100),
            DialMsg::Tapped => {}
        }
    }
}

/// Cancel costs nothing on a screen that scales a step.
///
/// A nudge means whatever the screen decides — five units here — so no count of
/// nudges could ever undo an edit: asking for `-4` back moves 20 the other way
/// only by luck of the scale. Nothing is dispatched at all now, so the scale
/// cannot enter into it: the screen's value never left 30 to be put back.
#[test]
fn cancelling_an_edit_is_free_when_a_step_is_scaled() {
    testing::install();
    testing::reset();
    testing::set_has_left_right_keys(false);

    let mut runtime = Runtime::new(ScaledDial {
        value: 30,
        dispatches: 0,
    });
    runtime.render();

    testing::press(Button::Confirm);
    runtime.loop_();
    runtime.render();

    for _ in 0..4 {
        testing::press(Button::Up);
        runtime.loop_();
        runtime.render();
    }
    assert_eq!(
        shown(&mut runtime),
        34,
        "the panel follows the keys: one unit of the track per press, not one \
         of the screen's five-unit nudges — inside an open edit the framework \
         owns the value"
    );
    assert_eq!(
        runtime.screen().value,
        30,
        "and the screen has not been told a thing"
    );

    testing::press(Button::Back);
    runtime.loop_();
    runtime.render();

    assert_eq!(
        runtime.screen().value,
        30,
        "cancel leaves it exactly where it was, whatever a step is worth"
    );
    assert_eq!(
        runtime.screen().dispatches,
        0,
        "and dispatches nothing at all — there is nothing to put back"
    );
}

/// A control that can only be nudged never opens an edit.
///
/// An edit that cannot be committed is worse than no edit: the keys change
/// meaning, the value moves on the panel, and Confirm has no message to send.
/// A `Stepper` with no `on_change` has no absolute setter, so Confirm leaves it
/// alone rather than opening a mode with no way out but cancelling.
#[test]
fn a_control_that_cannot_be_committed_is_not_editable() {
    testing::install();
    testing::reset();
    testing::set_has_left_right_keys(false);

    struct NudgeOnly {
        value: i32,
    }
    impl xpui::Screen for NudgeOnly {
        type Message = DialMsg;
        fn body(&self) -> impl View<DialMsg> {
            vstack![10; Stepper::new(self.value).on_step(DialMsg::Step)]
        }
        fn update(&mut self, message: DialMsg) {
            if let DialMsg::Step(delta) = message {
                self.value += delta;
            }
        }
    }

    let mut runtime = Runtime::new(NudgeOnly { value: 10 });
    runtime.render();

    testing::press(Button::Confirm);
    runtime.loop_();
    runtime.render();

    // No edit opened, so Up still walks the list rather than moving the value.
    testing::press(Button::Up);
    runtime.loop_();
    runtime.render();

    assert_eq!(
        runtime.screen().value,
        10,
        "Confirm must not open an edit on a control it could never commit"
    );
}

/// Cancel costs nothing on a screen that clamps, and the panel clamps too.
///
/// From 98, four Ups against a maximum of 100: two land and two are refused.
/// Cancelling used to dispatch the inverse of the four steps it had counted and
/// leave the value on **96** — below where the edit began, which is the one
/// thing a cancel must never do. There is no count and no dispatch now; the
/// clamp lives in the working copy, so the panel stops at 100 as well.
#[test]
fn cancelling_an_edit_costs_nothing_when_the_value_clamps() {
    testing::install();
    testing::reset();
    // The mode only exists on a board with no Left/Right pair.
    testing::set_has_left_right_keys(false);

    let mut runtime = Runtime::new(ClampedDial {
        value: 98,
        dispatches: 0,
    });
    runtime.render();

    testing::press(Button::Confirm);
    runtime.loop_();
    runtime.render();

    for _ in 0..4 {
        testing::press(Button::Up);
        runtime.loop_();
        runtime.render();
    }
    assert_eq!(
        shown(&mut runtime),
        100,
        "two of the four Ups run out of track and the panel says so"
    );
    assert_eq!(
        runtime.screen().value,
        98,
        "and none of them reached the screen"
    );

    testing::press(Button::Back);
    runtime.loop_();
    runtime.render();

    assert_eq!(
        runtime.screen().value,
        98,
        "cancel leaves exactly what the edit opened on"
    );
    assert_eq!(
        runtime.screen().dispatches,
        0,
        "having dispatched nothing at any point"
    );
    assert_eq!(
        testing::finishes(),
        0,
        "and it costs the edit, not the screen"
    );
}

/// Presses one key and lets the frame settle, with the clock moving as a real
/// loop's would.
fn tap(runtime: &mut Runtime<Dial>, key: Button, now: u32) {
    testing::set_millis(now);
    testing::press(key);
    runtime.loop_();
    runtime.render();
}

/// Confirm on something adjustable opens it; on an ordinary row it still fires.
#[test]
fn confirm_opens_an_adjustable_control_rather_than_firing_it() {
    let mut runtime = dial();
    assert_eq!(runtime.focused_index(), 0, "the stepper holds focus");

    tap(&mut runtime, Button::Confirm, 10);
    assert_eq!(
        runtime.screen().dispatches,
        0,
        "opening changes nothing by itself"
    );

    // The proof it opened: Up now moves the value, not the focus.
    tap(&mut runtime, Button::Up, 20);
    assert_eq!(painted_value(&testing::ops_log()), 51, "the panel moved");
    assert_eq!(
        runtime.screen().value,
        50,
        "and the screen has not heard about it"
    );
    assert_eq!(runtime.focused_index(), 0, "and focus stayed put");
}

/// Up raises and Down lowers — the opposite of what they do in a list.
#[test]
fn while_editing_up_raises_and_down_lowers() {
    let mut runtime = dial();
    tap(&mut runtime, Button::Confirm, 10);

    tap(&mut runtime, Button::Up, 20);
    tap(&mut runtime, Button::Up, 30);
    assert_eq!(
        painted_value(&testing::ops_log()),
        52,
        "Up walks the number upwards"
    );

    tap(&mut runtime, Button::Down, 40);
    assert_eq!(
        painted_value(&testing::ops_log()),
        51,
        "and Down walks it back"
    );
    assert_eq!(
        runtime.screen().dispatches,
        0,
        "and none of the three reached the screen"
    );
}

/// Confirm commits the working value, in **one** message.
///
/// The reason the value is held at all: a screen that persists on every change
/// writes once for an edit, not once per press, and what it writes is the
/// number the panel was showing when the key went down.
#[test]
fn confirm_commits_the_value_in_one_message() {
    let mut runtime = dial();
    tap(&mut runtime, Button::Confirm, 10);
    tap(&mut runtime, Button::Up, 20);
    tap(&mut runtime, Button::Up, 30);
    assert_eq!(painted_value(&testing::ops_log()), 52);
    assert_eq!(runtime.screen().value, 50, "still untold");

    tap(&mut runtime, Button::Confirm, 40);
    assert_eq!(
        runtime.screen().value,
        52,
        "the value the panel was showing"
    );
    assert_eq!(
        runtime.screen().dispatches,
        1,
        "one message for the whole edit, not one per press"
    );

    // The proof it closed: Down walks the list again instead of the value.
    tap(&mut runtime, Button::Down, 50);
    assert_eq!(runtime.focused_index(), 1, "focus moves once more");
    assert_eq!(runtime.screen().value, 52, "and the value is left alone");
    assert_eq!(runtime.screen().dispatches, 1, "and nothing else was sent");
}

/// Back drops the edit and does **not** leave the screen.
///
/// Back is the first key on the boards this mode exists for, so a stray press
/// has to cost the edit and nothing else.
#[test]
fn back_drops_the_edit_and_stays_on_the_screen() {
    let mut runtime = dial();
    tap(&mut runtime, Button::Confirm, 10);
    tap(&mut runtime, Button::Up, 20);
    tap(&mut runtime, Button::Up, 30);
    tap(&mut runtime, Button::Up, 40);
    assert_eq!(
        painted_value(&testing::ops_log()),
        53,
        "moved by more than one step"
    );

    tap(&mut runtime, Button::Back, 50);
    assert_eq!(
        painted_value(&testing::ops_log()),
        50,
        "the panel goes back to what the screen holds, all three steps at once"
    );
    assert_eq!(
        runtime.screen().dispatches,
        0,
        "cancel dispatches nothing at all"
    );
    assert_eq!(
        testing::finishes(),
        0,
        "and the screen is still here — Back closed the edit, nothing more"
    );

    // A second Back, with no edit open, is an ordinary Back again.
    tap(&mut runtime, Button::Back, 60);
    assert_eq!(testing::finishes(), 1, "now it leaves");
}

/// A value being edited on a slow panel moves by one press, not by several.
///
/// The same fault the list had: a refresh blinds the loop for most of a second
/// and the key still reads as down on the frame after. Editing runs through the
/// same `active_key`, so it inherits the guard — this is what says so, because
/// a value that jumps by four is far harder to notice than a list that does.
#[test]
fn editing_across_a_panel_refresh_moves_by_one_step() {
    let mut runtime = dial();
    tap(&mut runtime, Button::Confirm, 10);

    testing::set_millis(20);
    testing::hold(Button::Up);
    runtime.loop_();
    runtime.render();
    assert_eq!(
        painted_value(&testing::ops_log()),
        51,
        "the press itself moves one"
    );

    // The refresh, and the first frame the loop gets to look again.
    testing::set_millis(850);
    runtime.loop_();
    runtime.render();
    assert_eq!(
        painted_value(&testing::ops_log()),
        51,
        "a gap the loop could not see through must not walk the value"
    );
}

// -- auto-repeat --------------------------------------------------------------
//
// Repeat is the one piece of input that reads a clock, so it is the one piece
// that can be fooled by a frame that took a long time. Both of these drive the
// real `Runtime`, because the arithmetic is the whole behaviour.

/// One tap moves one row, even when the panel then blocks for most of a second.
///
/// The Badger's e-ink refresh is around 800 ms and the loop is blind for all of
/// it: the frame that presented sampled input *before* the press did anything,
/// and the finger comes off somewhere inside the refresh, so the level still
/// reads down on the frame after. Repeat used to credit that whole gap to the
/// hold and fire — a single press of Down walked the selection several rows,
/// which is what it did on real hardware.
#[test]
fn a_slow_panel_does_not_turn_one_press_into_many() {
    let mut runtime = runtime(false);
    assert_eq!(runtime.focused_index(), 0);

    testing::set_millis(0);
    testing::hold(Button::Down);
    runtime.loop_();
    assert_eq!(runtime.focused_index(), 1, "the press itself moves one row");

    // The refresh, and the first frame the loop gets to look again.
    testing::set_millis(830);
    runtime.loop_();
    assert_eq!(
        runtime.focused_index(),
        1,
        "a gap the loop could not see through is not a hold"
    );
}

/// And repeat still repeats when the loop can actually watch the button.
///
/// The other half of the pair: the guard above must cost nothing on a display
/// that draws straight through, which is every frame the Tufty and the
/// simulator run.
#[test]
fn a_button_held_across_frames_the_loop_can_see_still_repeats() {
    let mut runtime = runtime(false);

    testing::set_millis(0);
    testing::hold(Button::Down);
    runtime.loop_();
    assert_eq!(runtime.focused_index(), 1);

    // Ten-millisecond frames, which is what both boards' loops wait.
    let mut now = 0;
    while now < 490 {
        now += 10;
        testing::set_millis(now);
        runtime.loop_();
    }
    assert_eq!(
        runtime.focused_index(),
        1,
        "nothing repeats before the delay is up"
    );

    testing::set_millis(500);
    runtime.loop_();
    assert_eq!(runtime.focused_index(), 2, "past the delay it repeats");

    // And it stops when the finger does.
    testing::release();
    testing::set_millis(1_000);
    runtime.loop_();
    assert_eq!(
        runtime.focused_index(),
        2,
        "a released button does not repeat"
    );
}

/// Swiping up walks *down* the list — the direction the content moves under the
/// finger, and the inverse of what `Button::Up` does. `HomeActivity` maps swipe
/// Up to `nextIndex`; a Rust screen must feel identical.
#[test]
fn a_swipe_moves_focus_the_way_the_firmware_does() {
    let mut runtime = runtime(false);
    assert_eq!(runtime.focused_index(), 0);

    testing::set_swipe(SwipeDir::Up);
    runtime.loop_();
    assert_eq!(runtime.focused_index(), 1, "swipe up should advance");

    testing::set_swipe(SwipeDir::Down);
    runtime.loop_();
    assert_eq!(runtime.focused_index(), 0, "swipe down should go back");
}

/// Focus wraps at both ends, so a swipe never dead-ends.
#[test]
fn swipe_focus_wraps_at_both_ends() {
    let mut runtime = runtime(false);

    testing::set_swipe(SwipeDir::Down);
    runtime.loop_();
    assert_eq!(runtime.focused_index(), 2, "back from the first row wraps");

    testing::set_swipe(SwipeDir::Up);
    runtime.loop_();
    assert_eq!(runtime.focused_index(), 0, "forward from the last wraps");
}

/// A screen that wants swipes for itself — a reader paging, say — claims them
/// and the runtime leaves focus alone.
#[test]
fn a_screen_can_claim_the_swipe_instead() {
    let mut runtime = runtime(true);

    testing::set_swipe(SwipeDir::Up);
    runtime.loop_();

    assert_eq!(
        runtime.focused_index(),
        0,
        "a claimed swipe must not also move focus"
    );
}

/// Horizontal swipes belong to the back and home gestures; claiming them here
/// would break navigation.
#[test]
fn horizontal_swipes_do_not_move_focus() {
    let mut runtime = runtime(false);

    for direction in [SwipeDir::Left, SwipeDir::Right] {
        testing::set_swipe(direction);
        runtime.loop_();
        assert_eq!(
            runtime.focused_index(),
            0,
            "{direction:?} must not navigate"
        );
    }
}

// -- dialogs capture input ----------------------------------------------------
//
// A dialog shares the screen's tree with the list behind it. Without capture the
// side buttons walk straight out of the dialog and into those rows — invisible
// on screen, and the wrong thing entirely.

/// Three rows; tapping one opens a dialog, as the C++ Settings screen does.
struct WithDialog {
    open_row: Option<usize>,
    chose: Option<usize>,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum DlgMsg {
    OpenRow(usize),
    Chose(usize),
}

impl xpui::Screen for WithDialog {
    type Message = DlgMsg;

    fn body(&self) -> impl View<DlgMsg> {
        vstack![4;
            List::new()
                .push(ListRow::new("one").on_tap(DlgMsg::OpenRow(0)))
                .push(ListRow::new("two").on_tap(DlgMsg::OpenRow(1)))
                .push(ListRow::new("three").on_tap(DlgMsg::OpenRow(2))),
        ]
        .push_if(
            self.open_row.is_some(),
            Modal::picker("Pick", ["alpha", "beta"])
                .selected(1)
                .on_select(DlgMsg::Chose),
        )
    }

    fn update(&mut self, message: DlgMsg) {
        match message {
            DlgMsg::OpenRow(row) => self.open_row = Some(row),
            DlgMsg::Chose(index) => {
                self.chose = Some(index);
                self.open_row = None;
            }
        }
    }
}

fn dialog_runtime() -> Runtime<WithDialog> {
    testing::install();
    testing::reset();
    let mut runtime = Runtime::new(WithDialog {
        open_row: None,
        chose: None,
    });
    runtime.render();
    runtime
}

/// The whole round trip: walk to a row, open its dialog, walk inside the
/// dialog, choose — and land back on the row you came from.
#[test]
fn a_dialog_captures_focus_and_gives_it_back() {
    let mut runtime = dialog_runtime();

    // Move to the second row and open its dialog.
    testing::set_swipe(SwipeDir::Up);
    runtime.loop_();
    assert_eq!(runtime.focused_index(), 1);

    testing::press(Button::Confirm);
    runtime.loop_();
    // update() asked for a repaint; that is where focus settles into the dialog.
    runtime.render();
    assert_eq!(
        runtime.focused_index(),
        1,
        "the dialog opens on its selected option"
    );

    // Up/Down now walk the dialog's two options, never the three rows behind.
    testing::set_swipe(SwipeDir::Up);
    runtime.loop_();
    assert_eq!(runtime.focused_index(), 0, "wraps within the dialog");

    // Choosing closes it and returns focus to the row that opened it.
    testing::press(Button::Confirm);
    runtime.loop_();
    runtime.render();
    assert_eq!(
        runtime.focused_index(),
        1,
        "focus returns to the opening row"
    );
}

/// The point of capture: with a dialog up, the rows behind are unreachable.
#[test]
fn rows_behind_a_dialog_are_unreachable() {
    let mut runtime = dialog_runtime();

    testing::press(Button::Confirm);
    runtime.loop_(); // opens row 0's dialog

    // Two options in the dialog, three rows behind. Walking six times must
    // never land outside the dialog's range.
    for _ in 0..6 {
        testing::set_swipe(SwipeDir::Up);
        runtime.loop_();
        assert!(
            runtime.focused_index() < 2,
            "focus escaped the dialog to index {}",
            runtime.focused_index()
        );
    }
}

/// The bug this exists to prevent: a widget is told it holds focus *as it
/// declares*, so a list behind a dialog would record a focused row and keep
/// painting the highlight — which then appeared to move as the dialog was
/// navigated. Clearing the table afterwards is too late; the declaration itself
/// has to be ignored.
#[test]
fn views_behind_a_dialog_are_told_they_are_not_focused() {
    let mut out: Interactions<u8> = Interactions::capturing(0);

    let focused = out.declare(
        Rect::new(0, 0, 100, 40),
        InputMask::DEFAULT,
        Trigger::Message(1),
    );
    assert!(
        !focused,
        "a row behind the dialog must not paint as focused"
    );
    assert_eq!(out.focusable_count(), 0, "nor join the focus order");

    out.capture(0);

    let focused = out.declare(
        Rect::new(0, 0, 100, 40),
        InputMask::DEFAULT,
        Trigger::Message(2),
    );
    assert!(focused, "the dialog's own first option does hold focus");
    assert_eq!(out.focusable_count(), 1);
}

/// End to end, and the test that would have caught this on the device: with the
/// dialog open, the list behind must be told to highlight **nothing** (-1), no
/// matter how far focus travels inside the dialog. Asserting the focus index
/// alone does not catch it — the index stays in range either way, while the
/// list keeps painting a highlight that appears to move.
#[test]
fn the_list_behind_never_paints_a_focused_row() {
    let mut runtime = dialog_runtime();

    testing::press(Button::Confirm);
    runtime.loop_();

    for _ in 0..4 {
        testing::reset();
        runtime.render();

        let lists = testing::drawn_lists();
        assert!(!lists.is_empty(), "the list behind should still be drawn");
        for (rows, selected) in lists {
            assert_eq!(
                selected, -1,
                "a {rows}-row list behind the dialog was told to highlight row {selected}"
            );
        }

        testing::set_swipe(SwipeDir::Up);
        runtime.loop_();
    }
}

/// The arrows must actually move the highlight *inside* the dialog. Focus can
/// be moving perfectly while the dialog keeps painting whichever value the
/// screen passed to `selected` — which looks, from the device, exactly like the
/// arrows doing nothing.
#[test]
fn the_arrows_move_the_highlight_inside_the_dialog() {
    let mut runtime = dialog_runtime();

    testing::press(Button::Confirm);
    runtime.loop_();

    let highlight = |runtime: &mut Runtime<WithDialog>| {
        testing::reset();
        runtime.render();
        let popups = testing::drawn_popups();
        assert_eq!(popups.len(), 1, "the dialog should be drawn");
        popups[0].2
    };

    // Opens on the option the screen said was current.
    assert_eq!(highlight(&mut runtime), 1, "opens on the selected option");

    testing::press(Button::Up);
    runtime.loop_();
    assert_eq!(highlight(&mut runtime), 0, "Up moves the highlight");

    testing::press(Button::Down);
    runtime.loop_();
    assert_eq!(highlight(&mut runtime), 1, "Down moves it back");
}

/// A screen that changes a value the way its device allows.
///
/// The two paths a value control has: nudge it where it stands, which needs a
/// Left/Right pair, or enter it and leave again, which needs only Confirm and
/// the keys that walk the list. Which one a device can offer is not something
/// a screen may assume — [`Input::has_left_right_keys`] is how it asks.
struct Nudged {
    value: i32,
}

impl xpui::Screen for Nudged {
    type Message = i32;

    fn body(&self) -> impl View<i32> {
        Text::new(if Input::has_left_right_keys() {
            "Left and Right"
        } else {
            "Confirm to edit"
        })
    }

    fn on_key(&self, key: Button) -> Option<i32> {
        match key {
            Button::Left if Input::has_left_right_keys() => Some(self.value - 1),
            Button::Right if Input::has_left_right_keys() => Some(self.value + 1),
            _ => None,
        }
    }

    fn update(&mut self, message: i32) {
        self.value = message;
    }
}

/// A screen can find out what the device offers, and act on it.
///
/// Both halves matter and they fail differently. The label is what a person
/// reads before pressing anything, and a device that cannot nudge must not
/// promise it can; the key is whether the press does anything at all.
///
/// `Nudged` is a stand-in, not the shipped path: no widget branches on this
/// yet, so a value control still enters an edit mode on every device and a
/// reader's page-turn keys still move the value. Assuming rather than asking is
/// what put that there; this is the asking, and the branch is still to come.
#[test]
fn a_screen_asks_what_the_device_can_do() {
    for present in [false, true] {
        testing::install();
        testing::reset();
        testing::set_has_left_right_keys(present);

        let mut runtime = Runtime::new(Nudged { value: 5 });
        runtime.render();

        let labels = testing::render(&testing::ops_log());
        let promised = labels.contains("Left and Right");
        assert_eq!(
            promised, present,
            "a device with has_left_right_keys={present} must not be told otherwise"
        );

        testing::press(Button::Right);
        runtime.loop_();

        let expected = if present { 6 } else { 5 };
        assert_eq!(
            runtime.screen().value,
            expected,
            "Right with has_left_right_keys={present}"
        );
    }
}

/// Both readings of a vertical swipe are supported, because both are
/// defensible: by default the gesture drags the content (swipe up walks down
/// the list, matching the C++ home screen), and the preference reverses it so
/// the gesture drags the selection instead.
#[test]
fn swipe_direction_is_a_preference() {
    for moves_selection in [false, true] {
        testing::install();
        testing::reset();
        testing::set_swipe_moves_selection(moves_selection);

        let mut runtime = Runtime::new(Nav {
            claims_swipe: false,
            claimed: None,
        });
        runtime.render();
        assert_eq!(runtime.focused_index(), 0);

        testing::set_swipe(SwipeDir::Up);
        runtime.loop_();

        let expected = if moves_selection { 2 } else { 1 };
        assert_eq!(
            runtime.focused_index(),
            expected,
            "swipe up with swipe_moves_selection={moves_selection}"
        );
    }
}

/// A screen with buttons must say what they do. Only Back was labelled, so the
/// X3 showed a single hint and no sign that Select, Up or Down did anything.
#[test]
fn a_screen_labels_all_four_buttons_by_default() {
    testing::install();
    testing::reset();

    let mut root: NavigationScreen<()> = NavigationScreen::new(Text::new("content"));
    root.measure(screen());
    root.render(Point::ORIGIN);

    let hints = testing::drawn_hints();
    assert_eq!(hints.len(), 1, "the chrome should draw its hints once");
    assert_eq!(
        hints[0], [true; 4],
        "every button slot should carry a label by default"
    );
}

// -- scrolling -----------------------------------------------------------------
//
// A section title is not focusable, so nothing up there can pull the view back
// to it. Scrolling the bare minimum to reveal the first row therefore stranded
// the title just off the top edge, unreachable.

/// A screen taller than the panel: a titled section, then enough rows to scroll.
struct Tall;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TallMsg {
    Row(usize),
}

impl xpui::Screen for Tall {
    type Message = TallMsg;

    fn body(&self) -> impl View<TallMsg> {
        let mut rows = List::new();
        for index in 0..20 {
            rows = rows.push(ListRow::new("row").on_tap(TallMsg::Row(index)));
        }
        NavigationScreen::new(ScrollView::new(Section::new("A title", rows)))
    }

    fn update(&mut self, _message: TallMsg) {}
}

fn tall_runtime() -> Runtime<Tall> {
    testing::install();
    testing::reset();
    let mut runtime = Runtime::new(Tall);
    runtime.render();
    runtime
}

/// Walking to the bottom and back must land on the very top, title included -
/// not on the first row with the title clipped above it.
#[test]
fn returning_to_the_first_row_scrolls_to_the_very_top() {
    let mut runtime = tall_runtime();

    for _ in 0..19 {
        testing::press(Button::Down);
        runtime.loop_();
    }
    assert!(
        runtime.focused_index() > 0,
        "should have walked down the list"
    );

    for _ in 0..19 {
        testing::press(Button::Up);
        runtime.loop_();
    }
    assert_eq!(runtime.focused_index(), 0, "back at the first row");

    // The title sits above the first row, so any leftover scroll pushes it off
    // the top of the content band - where nothing focusable can bring it back.
    testing::reset();
    runtime.render();
    let titles = testing::drawn_sub_headers();
    let (rect, _) = titles
        .iter()
        .find(|(_, label)| label == "A title")
        .expect("the section title should have been drawn");
    assert!(
        rect.origin.y >= xpui::testing::CONTENT_TOP,
        "title drawn at y={}, above the content band at {} - it is clipped and unreachable",
        rect.origin.y,
        xpui::testing::CONTENT_TOP
    );
}

// -- a dialog must not scroll the screen behind it ----------------------------

/// A list long enough to scroll, with a dialog that opens over it.
struct TallWithDialog {
    open: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TallDlgMsg {
    Open,
    Chose(usize),
}

impl xpui::Screen for TallWithDialog {
    type Message = TallDlgMsg;

    fn body(&self) -> impl View<TallDlgMsg> {
        let mut rows = List::new();
        for _ in 0..20 {
            rows = rows.push(ListRow::new("row").on_tap(TallDlgMsg::Open));
        }
        NavigationScreen::new(ScrollView::new(rows)).overlay_if(
            self.open,
            Modal::picker("Units", ["B", "KB"])
                .selected(0)
                .on_select(TallDlgMsg::Chose),
        )
    }

    fn update(&mut self, message: TallDlgMsg) {
        match message {
            TallDlgMsg::Open => self.open = true,
            TallDlgMsg::Chose(_) => self.open = false,
        }
    }
}

/// Walking a dialog's options moved the list behind it.
///
/// While a view captures input, `self.focus` indexes the dialog's rows, not the
/// screen's - so settling scroll against it read "option 0" as "the top of the
/// content" and yanked the list up. The content behind an overlay is frozen; its
/// offset must not move until the dialog is gone.
#[test]
fn a_dialog_does_not_scroll_the_list_behind_it() {
    testing::install();
    testing::reset();
    let mut runtime = Runtime::new(TallWithDialog { open: false });
    runtime.render();

    // Walk far enough down that the list is genuinely scrolled.
    for _ in 0..19 {
        testing::press(Button::Down);
        runtime.loop_();
    }
    testing::reset();
    runtime.render();
    let scrolled = testing::drawn_indicators()
        .last()
        .copied()
        .expect("a scrolled list draws an indicator");
    assert!(
        scrolled.2 > 0,
        "the list should be scrolled away from the top"
    );

    // Open the dialog over it.
    testing::press(Button::Confirm);
    runtime.loop_();
    runtime.render();

    // Now walk the dialog's options. Nothing behind may move.
    for step in 0..4 {
        testing::press(Button::Down);
        runtime.loop_();
        testing::reset();
        runtime.render();

        let now = testing::drawn_indicators()
            .last()
            .copied()
            .expect("the list behind is still drawn");
        assert_eq!(
            now.2, scrolled.2,
            "step {step}: the list scrolled to {} while the dialog was open (was {})",
            now.2, scrolled.2
        );
    }
}

/// Space *within* a group must stay smaller than the space *between* groups, or
/// a heading reads as belonging to whatever sits above it. Both are constants,
/// so the relationship is enforced where it is decided: at compile time.
const _: () = assert!(xpui::testing::SPACING_SMALL < xpui::testing::VERTICAL_SPACING);

/// A heading is one line of its own, not a list row. Reserving a row's worth
/// left a hole beneath every heading, and made the next heading look like a
/// member of the list above it.
#[test]
fn a_section_heading_occupies_its_own_line_not_a_row() {
    testing::install();
    testing::reset();

    let mut section: Section<()> = Section::new("Heading", Text::new("content"));
    section.measure(screen());
    section.render(Point::ORIGIN);

    let (rect, _) = testing::drawn_sub_headers()
        .into_iter()
        .next()
        .expect("the heading should have been drawn");
    assert_eq!(
        rect.size.height,
        testing::SUB_HEADER_HEIGHT,
        "a heading band should be its own line, not {} px of list row",
        testing::LIST_ROW_HEIGHT
    );
}
