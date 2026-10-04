use super::PageMutationPlan;
use crate::model::{
    CardPageBlock, CardPageEditableBlock, EditPageBlockTextRequest, PageTextAnnotation,
    PageTextAnnotationKind, PageTextEditTarget,
};

use super::{
    super::rich_text::{
        annotations::{apply_annotation_actions_to_range, apply_plain_text_annotation_replacement},
        PageProjectedText, PageWriteTextProjection,
    },
    PageBlockAnnotationTransition,
};

struct PageAnnotationEdit<'a> {
    page_id: &'a str,
    target: PageTextEditTarget,
    removal: Option<PageTextAnnotationKind>,
    addition: Option<PageTextAnnotation>,
}

impl PageMutationPlan {
    pub(in crate::ui::board_workspace::page::editor) fn persist_page_block_annotations(
        &mut self,
        page_id: &str,
        block: &CardPageBlock,
    ) {
        let Some(editable) = block.editable_content() else {
            return;
        };
        let mut projected = editable.clone();
        projected.annotations.clear();
        self.enqueue_page_block_annotation_additions(
            page_id,
            &block.block_id,
            editable,
            &mut projected,
        );
    }

    pub(super) fn persist_page_block_annotation_transition(
        &mut self,
        transition: PageBlockAnnotationTransition<'_>,
    ) {
        let target = transition
            .block
            .editable_content()
            .expect("annotation transition target must be editable");
        if !transition.text_changed && transition.current.annotations == target.annotations {
            return;
        }
        let mut projected = transition.current.clone();
        if transition.text_changed {
            apply_plain_text_annotation_replacement(
                &mut projected,
                &transition.current.text,
                &target.text,
            );
            projected.text.clone_from(&target.text);
        }
        if !transition.current.annotations.is_empty() && !target.text.is_empty() {
            self.enqueue_page_block_annotation_removals(
                &transition,
                target.text.len(),
                &mut projected,
            );
        }
        self.enqueue_page_block_annotation_additions(
            transition.page_id,
            &transition.block.block_id,
            target,
            &mut projected,
        );
    }

    fn enqueue_page_block_annotation_removals(
        &mut self,
        transition: &PageBlockAnnotationTransition<'_>,
        target_text_len: usize,
        projected: &mut CardPageEditableBlock,
    ) {
        let page_id = transition.page_id;
        let block_id = transition.block.block_id.as_str();
        let current = transition.current;
        assert!(
            target_text_len > 0,
            "annotated page blocks must contain text"
        );
        let mut kinds = Vec::new();
        for kind in current
            .annotations
            .iter()
            .map(|span| span.annotation.kind())
        {
            if !kinds.contains(&kind) {
                kinds.push(kind);
            }
        }
        for kind in kinds {
            apply_annotation_actions_to_range(projected, 0..target_text_len, &[kind], &[]);
            self.enqueue_page_annotation_edit(
                PageAnnotationEdit {
                    page_id,
                    target: PageTextEditTarget::Selection {
                        block_id: block_id.to_string(),
                        start_utf8: 0,
                        end_utf8: target_text_len,
                    },
                    removal: Some(kind),
                    addition: None,
                },
                block_id,
                projected,
            );
        }
    }

    fn enqueue_page_block_annotation_additions(
        &mut self,
        page_id: &str,
        block_id: &str,
        editable: &CardPageEditableBlock,
        projected: &mut CardPageEditableBlock,
    ) {
        for span in &editable.annotations {
            apply_annotation_actions_to_range(
                projected,
                span.start_utf8..span.end_utf8,
                &[],
                std::slice::from_ref(&span.annotation),
            );
            self.enqueue_page_annotation_edit(
                PageAnnotationEdit {
                    page_id,
                    target: PageTextEditTarget::Selection {
                        block_id: block_id.to_string(),
                        start_utf8: span.start_utf8,
                        end_utf8: span.end_utf8,
                    },
                    removal: None,
                    addition: Some(span.annotation.clone()),
                },
                block_id,
                projected,
            );
        }
    }

    fn enqueue_page_annotation_edit(
        &mut self,
        edit: PageAnnotationEdit<'_>,
        block_id: &str,
        projected: &CardPageEditableBlock,
    ) {
        let request = EditPageBlockTextRequest::new(
            edit.page_id.to_string(),
            vec![edit.target],
            edit.removal.into_iter().collect(),
            edit.addition.into_iter().collect(),
        )
        .expect("restored page annotation span must form a valid edit");
        let projection = PageWriteTextProjection::update(
            PageProjectedText::editable(block_id, projected)
                .expect("projected restored annotations must remain valid"),
        );
        self.enqueue_page_rich_text_edit_with_projection(request, projection);
    }
}
