use crate::model::NotionSidebarInboxFilter;

use super::super::{
    div, px, relative, rgb, AnyElement, FluentBuilder, FontWeight, InteractiveElement, IntoElement,
    PageShellInboxItem, ParentElement, SidebarRenderer, Styled,
};
use super::time::{inbox_date_bucket, InboxDateBucket};
use crate::ui::surface::NotionSidebarState;
use gpui::App;

impl SidebarRenderer {
    pub(super) fn render_notion_inbox_items(
        &self,
        state: &NotionSidebarState,
        items: &[PageShellInboxItem],
        cx: &mut App,
    ) -> AnyElement {
        let mut children = Vec::new();
        if state.inbox_filter == NotionSidebarInboxFilter::WorkspaceUpdates {
            children.push(self.render_notion_inbox_group_header(
                state,
                InboxDateBucket::Today,
                true,
                cx,
            ));
            for item in items {
                children.push(self.render_notion_inbox_item(state, item, cx));
            }
            return div().w_full().children(children).into_any_element();
        }
        let mut previous_bucket = None;
        let mut first_group = true;
        for item in items {
            let bucket = inbox_date_bucket(item.event_time_ms);
            if previous_bucket != Some(bucket) {
                children.push(self.render_notion_inbox_group_header(
                    state,
                    bucket,
                    first_group,
                    cx,
                ));
                previous_bucket = Some(bucket);
                first_group = false;
            }
            children.push(self.render_notion_inbox_item(state, item, cx));
        }
        div().w_full().children(children).into_any_element()
    }

    fn render_notion_inbox_group_header(
        &self,
        state: &NotionSidebarState,
        bucket: InboxDateBucket,
        first_group: bool,
        cx: &mut App,
    ) -> AnyElement {
        let label = if state.inbox_filter == NotionSidebarInboxFilter::WorkspaceUpdates {
            ""
        } else {
            bucket.label()
        };
        div()
            .id(format!("notion-inbox-group-{}", bucket.element_segment()))
            .w_full()
            .h(px(if first_group { 30.0 } else { 32.0 }))
            .when(!first_group, |this| this.pt(px(8.0)))
            .px(px(8.0))
            .flex()
            .items_center()
            .text_size(px(12.0))
            .font_weight(FontWeight::MEDIUM)
            .line_height(relative(1.0))
            .text_color(rgb(super::super::notion_sidebar_muted(
                self.appearance_mode,
            )))
            .child(div().flex_grow(1.0).child(label))
            .when(first_group, |this| {
                this.child(self.render_notion_inbox_header_actions(state, cx))
            })
            .into_any_element()
    }
}
