//! The doubles held to the contracts they stand in for: a recording that says
//! what the fake says, and input that reads the same all frame.

use super::host::TEST_HOST;
use super::{
    Recorder, hold, next_frame, ops_log, press, release, reset, set_millis, set_swipe, updates,
};
use crate::geometry::Rect;
use crate::host::{Button, Canvas, Chrome, InputSource, SwipeDir};

#[test]
fn a_recorded_dither_marks_the_shade_the_fake_marks() {
    reset();
    let recorder = Recorder::wrap(&TEST_HOST);
    recorder.fill_rect_dither(Rect::new(0, 0, 10, 10), true);
    recorder.fill_rect_dither(Rect::new(10, 0, 10, 10), false);

    // Wrapping the fake records each call twice: the recorder's, then the fake's.
    let ops = ops_log();
    assert_eq!(ops.len(), 4);
    assert_eq!(
        ops[0], ops[1],
        "a golden taken through `Ui` must read as the fake's"
    );
    assert_eq!(ops[2], ops[3]);
    assert_eq!(ops[0].to_line(), "rect        (0,0 10x10) dither light");
    assert_eq!(ops[2].to_line(), "rect        (10,0 10x10) dither");
}

#[test]
fn a_recorder_counts_the_repaints_it_forwards() {
    reset();
    Recorder::wrap(&TEST_HOST).request_update();
    assert_eq!(
        updates(),
        2,
        "counted once by the recorder and once by the fake it forwards to"
    );
}

#[test]
fn a_press_reads_the_same_until_the_frame_ends() {
    reset();
    press(Button::Down);
    assert!(TEST_HOST.was_pressed(Button::Down));
    assert!(
        TEST_HOST.was_pressed(Button::Down),
        "reading an edge twice in one frame must not consume it"
    );
    assert!(!TEST_HOST.was_pressed(Button::Up));

    next_frame();
    assert!(
        !TEST_HOST.was_pressed(Button::Down),
        "an edge lasts one frame"
    );
}

#[test]
fn a_swipe_reads_the_same_until_the_frame_ends() {
    reset();
    set_swipe(SwipeDir::Up);
    assert_eq!(TEST_HOST.swipe(), SwipeDir::Up);
    assert_eq!(TEST_HOST.swipe(), SwipeDir::Up, "not consumed by reading");

    next_frame();
    assert_eq!(TEST_HOST.swipe(), SwipeDir::None);
}

#[test]
fn a_write_after_a_read_starts_the_next_frame() {
    reset();
    hold(Button::Up);
    assert!(TEST_HOST.was_pressed(Button::Up));

    set_millis(500);
    assert!(
        !TEST_HOST.was_pressed(Button::Up),
        "the edge was last frame's"
    );
    assert!(
        TEST_HOST.is_pressed(Button::Up),
        "and the level is still down"
    );

    // Two writes before any read describe one frame together.
    press(Button::Confirm);
    set_swipe(SwipeDir::Left);
    assert!(TEST_HOST.was_pressed(Button::Confirm));
    assert_eq!(TEST_HOST.swipe(), SwipeDir::Left);

    release();
    assert!(!TEST_HOST.was_pressed(Button::Confirm));
    assert!(!TEST_HOST.is_pressed(Button::Up));
    set_millis(0);
}
