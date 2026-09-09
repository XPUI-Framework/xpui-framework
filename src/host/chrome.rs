//! The themed furniture: metrics the theme owns, and the parts it paints
//! itself.
//!
//! A list, a modal and a progress bar are drawn by the host rather than by the
//! framework, so a Rust screen looks identical to a native one and follows the
//! user's theme without the framework knowing what a theme is.

use alloc::string::String;
use core::sync::atomic::{AtomicBool, Ordering};

use crate::geometry::Rect;

/// A geometry value from the active theme.
///
/// Requested one at a time by tag rather than mirrored as a struct: a host's
/// metrics table has dozens of fields, and a copy of its layout here would
/// silently read the wrong values the moment one is inserted.
#[repr(u8)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ThemeMetric {
    /// Gap above the header band.
    TopPadding = 0,
    HeaderHeight = 1,
    /// The gap between stacked elements.
    VerticalSpacing = 2,
    /// The band reserved at the bottom for the button hints.
    ButtonHintsHeight = 3,
    ContentSidePadding = 4,
    /// First y below the header that content may use.
    ContentTop = 5,
    /// First y occupied by the button hints; content must stay above.
    ContentBottom = 6,
    ListRowHeight = 7,
    ListRowHeightWithSubtitle = 8,
    /// Space the theme leaves between one row and the next. A list that
    /// measured without it asks for less height than the host needs, and the
    /// host draws only rows that fully fit — so the last one silently vanishes.
    ListRowGap = 14,
    ProgressBarHeight = 9,
    /// The smallest comfortably tappable dimension.
    MinTouchSize = 10,
    /// A slider knob's width. The framework converts a touch to a value with
    /// this and [`SliderSideInset`](ThemeMetric::SliderSideInset), so both
    /// must be the numbers the host actually draws with.
    SliderKnobWidth = 11,
    SliderKnobHeight = 12,
    /// The padding a slider's track is inset by at each end.
    SliderSideInset = 13,
    /// Height of the band a sub-header needs: the heading's own line, not a
    /// list row. The theme draws the label top-aligned and ignores the rest,
    /// so reserving a row's worth leaves a hole under every heading.
    SubHeaderHeight = 15,
    /// The theme's small step, for space *within* a group. Separation between
    /// groups is the caller's stack spacing, which must stay the larger of the
    /// two or a heading reads as belonging to whatever sits above it.
    SpacingSmall = 16,
}

/// What the keys will do to a control next, so a person can see it: a value
/// row on a device with no Left/Right pair changes what four keys mean when
/// it opens, and the panel has to say so.
///
/// Crosses the C ABI as an integer, like the row field beside it, so a host
/// written in C++ can switch on it.
#[repr(u8)]
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub enum ControlState {
    /// The keys are elsewhere. Draw it as the value it holds and nothing more.
    #[default]
    Idle = 0,
    /// The keys would act on this control if they moved a value now. Other
    /// focusable things already show this — match whatever a focused list row
    /// does rather than inventing a second idiom.
    Focused = 1,
    /// The control is open: the keys that walked the list are moving this
    /// value, Confirm keeps it, and Back drops the edit rather than leaving the
    /// screen. Must be distinguishable from [`Focused`](ControlState::Focused)
    /// at a glance, or the mode is still invisible.
    Editing = 2,
}

/// Which piece of a list row is being asked for.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum RowField {
    Title,
    Subtitle,
    Value,
}

/// Chrome the host draws on the framework's behalf.
pub trait Chrome {
    fn metric(&self, metric: ThemeMetric) -> i32;

    /// The header band, including whatever the host puts in it (a battery
    /// indicator, say). `None` uses the screen's own title.
    fn draw_header(&self, title: Option<&str>, subtitle: Option<&str>);

    fn draw_sub_header(&self, rect: Rect, label: &str, right_label: Option<&str>);

    /// The four hints, given by meaning. The host reorders them to match the
    /// user's button layout; `None` means "your standard label for this slot".
    fn draw_button_hints(&self, back: &Hint, confirm: &Hint, previous: &Hint, next: &Hint);

    fn draw_progress_bar(&self, rect: Rect, current: u32, total: u32);

    /// The themed slider: a track, a fill up to `value`, and a knob over both.
    /// The host owns every dimension of it; the framework says only where it
    /// goes, how far along it is, and what the keys will do to it next.
    fn draw_slider(&self, rect: Rect, value: i32, max: i32, state: ControlState);

    /// The scroll indicator beside a scrolling region: how much of `content`
    /// the `rect`-sized window shows, and how far down it sits. The host draws
    /// nothing when everything already fits.
    fn draw_scroll_indicator(&self, rect: Rect, content: i32, visible: i32, offset: i32);

    /// The themed list. `row` is called back per visible row and field;
    /// returning `None` omits that field, which is how the host decides between
    /// a one- and two-line row.
    fn draw_list<'a>(
        &self,
        rect: Rect,
        rows: usize,
        selected: i32,
        row: &dyn Fn(usize, RowField) -> Option<&'a str>,
    );

    /// The themed modal. `option` is called back per row.
    fn draw_option_popup<'a>(
        &self,
        title: &str,
        options: &dyn Fn(usize) -> Option<&'a str>,
        count: usize,
        selected: i32,
    );

    /// Screen rect of one modal row, so hit-testing does not re-derive the
    /// dialog geometry the host already owns.
    fn option_popup_row_rect<'a>(
        &self,
        title: &str,
        options: &dyn Fn(usize) -> Option<&'a str>,
        count: usize,
        index: usize,
    ) -> Option<Rect>;

    /// Asks for a repaint. E-ink does not refresh on its own.
    ///
    /// On the painting trait, not [`Navigator`](super::Navigator): a repaint
    /// is a display concern, and a host that installed no navigator must
    /// still refresh.
    fn request_update(&self);
}

/// One button-hint slot.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum Hint {
    /// The host's own translated label for this slot.
    #[default]
    Standard,
    /// Leave the slot blank.
    None,
    /// A label this screen supplies.
    Text(String),
    /// The host's word for opening a value control.
    ///
    /// **Not a string.** The framework does not know what language its reader
    /// uses, which is the same reason [`Standard`](Hint::Standard) is not one;
    /// a host answers with its own word. The runtime asks for this over the
    /// Confirm key when the focused control can be edited.
    Edit,
    /// The host's word for keeping what a value now reads. Over Confirm while
    /// a control is open.
    Done,
    /// The host's word for putting a value back. Over Back while a control is
    /// open — the key that would otherwise leave the screen.
    Cancel,
}

impl Hint {
    pub fn text(label: impl Into<String>) -> Self {
        Hint::Text(label.into())
    }

    /// What the host should draw: `None` means "a word of your own", and
    /// [`word`](Hint::word) says which.
    pub fn label(&self) -> Option<&str> {
        match self {
            Hint::Standard | Hint::Edit | Hint::Done | Hint::Cancel => None,
            Hint::None => Some(""),
            Hint::Text(text) => Some(text),
        }
    }

    /// Which of the host's own words this slot wants, when
    /// [`label`](Hint::label) says it wants one.
    ///
    /// Crossing the C ABI as an integer beside the label pointer, so a host can
    /// switch on it: 0 is the standard label for whichever key the slot is,
    /// and the rest are words the four standard ones have no room for.
    pub fn word(&self) -> HintWord {
        match self {
            Hint::Edit => HintWord::Edit,
            Hint::Done => HintWord::Done,
            Hint::Cancel => HintWord::Cancel,
            Hint::Standard | Hint::None | Hint::Text(_) => HintWord::Standard,
        }
    }
}

/// Which of a host's own words a hint slot is asking for.
///
/// The four standard labels are chosen by the key a slot sits over — Back,
/// Select, Up, Down. These three are chosen by what the framework is *doing*,
/// and no key implies them.
#[repr(u8)]
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub enum HintWord {
    /// Whatever this slot's key is normally called.
    #[default]
    Standard = 0,
    /// "This control can be opened" — offered over Confirm when the keys are on
    /// a value the framework could take over.
    Edit = 1,
    /// "Keep what this now reads" — over Confirm while a value is open.
    Done = 2,
    /// "Leave it as you found it" — over Back while a value is open, where Back
    /// would otherwise leave the screen.
    Cancel = 3,
}

/// The active theme, as widgets reach for it.
pub struct Theme;

impl Theme {
    pub fn metric(metric: ThemeMetric) -> i32 {
        super::current().metric(metric)
    }

    /// The region between the header and the button hints, already inset
    /// horizontally by the theme's side padding.
    pub fn content_area() -> Rect {
        let top = Self::metric(ThemeMetric::ContentTop);
        let bottom = Self::metric(ThemeMetric::ContentBottom);
        let side = Self::metric(ThemeMetric::ContentSidePadding);
        let width = super::current().screen_size().width;

        Rect::new(side, top, width - side * 2, bottom - top)
    }

    pub fn draw_sub_header(rect: Rect, label: &str, right_label: Option<&str>) {
        super::current().draw_sub_header(rect, label, right_label)
    }

    pub fn draw_progress_bar(rect: Rect, current: u32, total: u32) {
        super::current().draw_progress_bar(rect, current, total)
    }

    pub fn draw_slider(rect: Rect, value: i32, max: i32, state: ControlState) {
        super::current().draw_slider(rect, value, max, state)
    }

    pub fn draw_scroll_indicator(rect: Rect, content: i32, visible: i32, offset: i32) {
        super::current().draw_scroll_indicator(rect, content, visible, offset)
    }

    /// The themed list — see [`Chrome::draw_list`].
    pub fn draw_list<'a>(
        rect: Rect,
        rows: usize,
        selected: i32,
        row: &dyn Fn(usize, RowField) -> Option<&'a str>,
    ) {
        super::current().draw_list(rect, rows, selected, row)
    }

    pub fn draw_option_popup<'a>(
        title: &str,
        options: &dyn Fn(usize) -> Option<&'a str>,
        count: usize,
        selected: i32,
    ) {
        super::current().draw_option_popup(title, options, count, selected)
    }

    pub fn option_popup_row_rect<'a>(
        title: &str,
        options: &dyn Fn(usize) -> Option<&'a str>,
        count: usize,
        index: usize,
    ) -> Option<Rect> {
        super::current().option_popup_row_rect(title, options, count, index)
    }
}

/// The screen's own furniture: header band and button hints.
pub struct ScreenChrome;

impl ScreenChrome {
    /// Draws the header with an explicit title.
    pub fn draw_header(title: &str) {
        super::current().draw_header(Some(title), None)
    }

    /// Draws the header with the screen's own translated title, fetched here
    /// rather than passed as `None`: to a `Chrome` implementation `None`
    /// means no title, and the band comes out empty.
    pub fn draw_screen_header() {
        super::current().draw_header(Some(Self::screen_title()), None)
    }

    pub fn draw_button_hints(back: &Hint, confirm: &Hint, previous: &Hint, next: &Hint) {
        super::current().draw_button_hints(back, confirm, previous, next)
    }

    /// The running screen's own title, from the installed
    /// [`Navigator`](super::Navigator).
    pub fn screen_title() -> &'static str {
        super::navigator().screen_title()
    }
}

/// Set whenever a repaint is asked for, and cleared when one happens.
///
/// The host has a flag of its own, set by `Chrome::request_update`; this one
/// is for [`App`](crate::App), which owns the loop and cannot read the
/// host's. Load and store only: neither bare-metal target has atomic
/// compare-and-swap, so `swap` and `fetch_or` do not compile there.
static NEEDS_PAINT: AtomicBool = AtomicBool::new(false);

/// Asks for a repaint. E-ink does not refresh on its own.
///
/// Anything that changes what the panel should show calls this — moving the
/// focus, nudging a slider, a screen's own `update`. Navigation does not need
/// to: pushing or popping is a repaint by definition.
pub fn request_update() {
    NEEDS_PAINT.store(true, Ordering::Relaxed);
    super::current().request_update()
}

/// Whether a repaint has been asked for and not yet done.
pub(crate) fn needs_paint() -> bool {
    NEEDS_PAINT.load(Ordering::Relaxed)
}

/// Marks the pending repaint as satisfied.
pub(crate) fn clear_needs_paint() {
    NEEDS_PAINT.store(false, Ordering::Relaxed);
}
