use super::super::super::{CardLocation, Tone};

#[derive(Clone, Copy)]
pub(super) struct CardRenderContext {
    pub(super) location: CardLocation,
    pub(super) default_fill: Option<u32>,
    pub(super) tone: Tone,
    pub(super) is_drag_source: bool,
}

impl CardRenderContext {
    pub(super) const fn new(
        location: CardLocation,
        default_fill: Option<u32>,
        tone: Tone,
        is_drag_source: bool,
    ) -> Self {
        Self {
            location,
            default_fill,
            tone,
            is_drag_source,
        }
    }
}
