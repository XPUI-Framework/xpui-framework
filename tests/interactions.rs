//! Interaction regression tests.
//!
//! Widgets declare their interactive regions in a walk that mirrors `render`.
//! Nothing in the type system keeps those two in step, so these tests are what
//! catch a container whose implementations have drifted apart — the failure
//! mode being touches that land on the control next door.

use xpui::screen::{Driver, Runtime};
use xpui::testing::{self, MIN_TOUCH_SIZE, SCREEN_HEIGHT, SCREEN_WIDTH};
use xpui::{
    Alignment, Button, HStack, Input, InputMask, Interactions, List, ListRow, Modal, Modifiers,
    NavigationScreen, Point, Rect, ScrollView, Section, Size, Slider, Spacer, Stepper, SwipeDir,
    Text, Toggle, Trigger, VStack, View, ViewExt, hstack, value_at, vstack,
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
        match message {
            DialMsg::Set(value) => self.value = value,
            DialMsg::Step(delta) => self.value += delta,
            DialMsg::Tapped => self.tapped += 1,
        }
    }
}

fn dial() -> Runtime<Dial> {
    testing::install();
    testing::reset();
    let mut runtime = Runtime::new(Dial {
        value: 50,
        tapped: 0,
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
fn a_mapped_control_can_still_be_nudged_and_put_back() {
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
        "and the setter has to cross, or the control cannot be put back"
    );
    assert_eq!(
        item.trigger.resolve_step(1),
        Some(Outer::Inner(DialMsg::Step(1))),
        "a nudge arrives as the outer message"
    );
    assert_eq!(
        item.trigger.restore(40),
        None,
        "restoring to where it already reads asks for nothing"
    );
    assert_eq!(
        item.trigger.restore(25),
        Some(Outer::Inner(DialMsg::Set(25))),
        "and restoring sets the value outright, through the conversion"
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
}

impl xpui::Screen for ClampedDial {
    type Message = DialMsg;

    fn body(&self) -> impl View<DialMsg> {
        vstack![10;
            Stepper::new(self.value).on_change(DialMsg::Set).on_step(DialMsg::Step),
        ]
    }

    fn update(&mut self, message: DialMsg) {
        match message {
            DialMsg::Set(value) => self.value = value.clamp(0, 100),
            DialMsg::Step(delta) => self.value = (self.value + delta).clamp(0, 100),
            DialMsg::Tapped => {}
        }
    }
}

/// One nudge, five units — what a frontlight row does.
///
/// The other way a cancel computed from steps goes wrong, and the one spec 26
/// got right: a screen that scales reads an inverse total of `-4` as `-20`.
struct ScaledDial {
    value: i32,
}

impl xpui::Screen for ScaledDial {
    type Message = DialMsg;

    fn body(&self) -> impl View<DialMsg> {
        vstack![10;
            Stepper::new(self.value).on_change(DialMsg::Set).on_step(DialMsg::Step),
        ]
    }

    fn update(&mut self, message: DialMsg) {
        match message {
            DialMsg::Set(value) => self.value = value.clamp(0, 100),
            DialMsg::Step(delta) => self.value = (self.value + delta * 5).clamp(0, 100),
            DialMsg::Tapped => {}
        }
    }
}

/// Cancel is exact when a step is worth more than one unit.
///
/// A nudge means whatever the screen decides, so no count of nudges can undo an
/// edit: four Ups here move 20, and asking for `-4` back moves 20 the other way
/// only by luck of the scale. Cancel sets the value it opened on instead.
#[test]
fn cancelling_an_edit_is_exact_when_a_step_is_scaled() {
    testing::install();
    testing::reset();
    testing::set_has_left_right_keys(false);

    let mut runtime = Runtime::new(ScaledDial { value: 30 });
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
        runtime.screen().value,
        50,
        "four Ups of five units each — the scale is the point of this screen"
    );

    testing::press(Button::Back);
    runtime.loop_();
    runtime.render();

    assert_eq!(
        runtime.screen().value,
        30,
        "cancel must land on what the edit opened on, whatever a step is worth"
    );
}

/// A control that can only be nudged never opens an edit.
///
/// An edit that cannot be cancelled is worse than no edit: the keys change
/// meaning and Back cannot put the value back. A `Stepper` with no `on_change`
/// has no way to be set, so Confirm leaves it alone.
#[test]
fn a_control_with_no_way_back_is_not_editable() {
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
        "Confirm must not open an edit on a control it cannot put back"
    );
}

/// Cancel puts the value back exactly, even when the screen swallowed steps.
///
/// From 98, four Ups against a maximum of 100: two land and two are refused.
/// Cancelling used to dispatch the inverse of the four it had counted and leave
/// the value on **96** — below where the edit began, which is the one thing a
/// cancel must never do. The restore is computed from what the control reads
/// now, so the swallowed steps cannot enter into it.
#[test]
fn cancelling_an_edit_restores_a_clamped_value_exactly() {
    testing::install();
    testing::reset();
    // The mode only exists on a board with no Left/Right pair.
    testing::set_has_left_right_keys(false);

    let mut runtime = Runtime::new(ClampedDial { value: 98 });
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
        runtime.screen().value,
        100,
        "two of the four Ups should have been swallowed by the clamp"
    );

    testing::press(Button::Back);
    runtime.loop_();
    runtime.render();

    assert_eq!(
        runtime.screen().value,
        98,
        "cancel must land on exactly what the edit opened on"
    );
    assert_eq!(
        testing::finishes(),
        0,
        "and it must cost the edit, not the screen"
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
        runtime.screen().value,
        50,
        "opening changes nothing by itself"
    );

    // The proof it opened: Up now moves the value, not the focus.
    tap(&mut runtime, Button::Up, 20);
    assert_eq!(runtime.screen().value, 51);
    assert_eq!(runtime.focused_index(), 0, "and focus stayed put");
}

/// Up raises and Down lowers — the opposite of what they do in a list.
#[test]
fn while_editing_up_raises_and_down_lowers() {
    let mut runtime = dial();
    tap(&mut runtime, Button::Confirm, 10);

    tap(&mut runtime, Button::Up, 20);
    tap(&mut runtime, Button::Up, 30);
    assert_eq!(runtime.screen().value, 52, "Up walks the number upwards");

    tap(&mut runtime, Button::Down, 40);
    assert_eq!(runtime.screen().value, 51, "and Down walks it back");
}

/// Confirm leaves, keeping what the value now reads.
#[test]
fn confirm_keeps_the_value_and_closes_the_edit() {
    let mut runtime = dial();
    tap(&mut runtime, Button::Confirm, 10);
    tap(&mut runtime, Button::Up, 20);
    tap(&mut runtime, Button::Up, 30);
    assert_eq!(runtime.screen().value, 52);

    tap(&mut runtime, Button::Confirm, 40);
    assert_eq!(runtime.screen().value, 52, "the value is kept");

    // The proof it closed: Down walks the list again instead of the value.
    tap(&mut runtime, Button::Down, 50);
    assert_eq!(runtime.focused_index(), 1, "focus moves once more");
    assert_eq!(runtime.screen().value, 52, "and the value is left alone");
}

/// Back puts the value back, and does **not** leave the screen.
///
/// Back is the first key on the boards this mode exists for, so a stray press
/// has to cost the edit and nothing else.
#[test]
fn back_restores_the_value_and_stays_on_the_screen() {
    let mut runtime = dial();
    tap(&mut runtime, Button::Confirm, 10);
    tap(&mut runtime, Button::Up, 20);
    tap(&mut runtime, Button::Up, 30);
    tap(&mut runtime, Button::Up, 40);
    assert_eq!(runtime.screen().value, 53, "moved by more than one step");

    tap(&mut runtime, Button::Back, 50);
    assert_eq!(
        runtime.screen().value,
        50,
        "cancel undoes the whole run, not just the last step"
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
    assert_eq!(runtime.screen().value, 51, "the press itself moves one");

    // The refresh, and the first frame the loop gets to look again.
    testing::set_millis(850);
    runtime.loop_();
    runtime.render();
    assert_eq!(
        runtime.screen().value,
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
