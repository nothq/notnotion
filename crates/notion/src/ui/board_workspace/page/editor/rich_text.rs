use crate::model::{
    EditPageBlockTextRequest, PageMutation, PageTextAnnotation, PageTextAnnotationKind,
    PageTextColor,
};

mod actions;
pub(super) mod annotations;
mod conversion;
mod highlights;
mod input;
mod projection;
mod render;

pub(super) use annotations::mention_spans;
pub(super) use highlights::{
    page_mention_foreground, page_mention_ghost_foreground, page_mention_input_pill_background,
    page_simple_table_text_input_highlights, page_static_rich_text, page_text_input_highlights,
    PageStaticTextFont,
};
pub(super) use input::{page_rich_text_link_on_change, PageRichTextInputAction};
pub(crate) use projection::{PageProjectedText, PageTextProjectionTarget, PageWriteTextProjection};

#[derive(Clone, Debug)]
pub(crate) enum PageWriteOperation {
    Mutation(PageMutation),
    RichText(EditPageBlockTextRequest),
}

#[derive(Clone, Debug)]
pub(crate) struct PageWrite {
    pub(crate) operation: PageWriteOperation,
    pub(crate) text_projection: PageWriteTextProjection,
}

impl PageWrite {
    pub(crate) fn mutation(
        mutation: PageMutation,
        text_projection: PageWriteTextProjection,
    ) -> Self {
        Self {
            operation: PageWriteOperation::Mutation(mutation),
            text_projection,
        }
    }

    pub(crate) fn rich_text(
        request: EditPageBlockTextRequest,
        text_projection: PageWriteTextProjection,
    ) -> Self {
        Self {
            operation: PageWriteOperation::RichText(request),
            text_projection,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PageForcedTextAnnotations {
    pub(crate) block_id: String,
    pub(crate) offset_utf8: usize,
    pub(crate) removals: Vec<PageTextAnnotationKind>,
    pub(crate) additions: Vec<PageTextAnnotation>,
}

#[derive(Clone, Debug)]
pub(crate) struct PagePendingRichTextTyping {
    pub(crate) block_id: String,
    pub(crate) offset_utf8: usize,
    pub(crate) text: String,
    pub(crate) removals: Vec<PageTextAnnotationKind>,
    pub(crate) additions: Vec<PageTextAnnotation>,
}

#[derive(Clone, Debug)]
pub(crate) struct PagePendingRichTextComposition {
    pub(crate) block_id: String,
    pub(crate) offset_utf8: usize,
    pub(crate) baseline_text: String,
    pub(crate) text: String,
    pub(crate) removals: Vec<PageTextAnnotationKind>,
    pub(crate) additions: Vec<PageTextAnnotation>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PageRichTextDialog {
    Link,
    Color,
}

impl PageForcedTextAnnotations {
    fn new(block_id: String, offset_utf8: usize) -> Self {
        Self {
            block_id,
            offset_utf8,
            removals: Vec::new(),
            additions: Vec::new(),
        }
    }

    fn set(&mut self, annotation: PageTextAnnotation, enabled: bool) {
        let kind = annotation.kind();
        self.removals.retain(|candidate| *candidate != kind);
        self.additions.retain(|candidate| candidate.kind() != kind);
        if enabled {
            self.additions.push(annotation);
        } else {
            self.removals.push(kind);
        }
    }
}

const PAGE_TEXT_COLORS: [(PageTextColor, &str); 10] = [
    (PageTextColor::Default, "Default"),
    (PageTextColor::Gray, "Gray"),
    (PageTextColor::Brown, "Brown"),
    (PageTextColor::Orange, "Orange"),
    (PageTextColor::Yellow, "Yellow"),
    (PageTextColor::Green, "Green"),
    (PageTextColor::Blue, "Blue"),
    (PageTextColor::Purple, "Purple"),
    (PageTextColor::Pink, "Pink"),
    (PageTextColor::Red, "Red"),
];
