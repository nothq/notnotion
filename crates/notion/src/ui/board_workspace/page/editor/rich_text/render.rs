use gpui::{
    px, App, Bounds, ElementId, MouseButton, MouseDownEvent, ParentElement, Pixels, Role, Styled,
};

use self::shadow::page_rich_text_shadow;
use super::PageRichTextDialog;
use crate::model::{PageTextAnnotation, PageTextColor};
use crate::ui::surface::PageEditorState;
use crate::ui::view_actions::ViewActionSink;
use crate::ui::{
    alpha, div, rgb, AnyElement, AppearanceMode, Div, FluentBuilder, InteractiveElement,
    IntoElement, StatefulInteractiveElement, Theme,
};
use gpui_components::text_input::TextInput;

mod color;
mod host;
mod shadow;

#[derive(Clone)]
enum ToolbarAction {
    NormalText,
    ApplyLink,
    RemoveLink,
    SetColor {
        color: PageTextColor,
        background: bool,
    },
    Toggle(PageTextAnnotation),
    Link,
    Color,
    Clear,
}

struct ToolbarButtonSpec {
    id: &'static str,
    label: &'static str,
    glyph: &'static str,
    action: ToolbarAction,
}

fn toolbar_primary_specs() -> [ToolbarButtonSpec; 5] {
    [
        ToolbarButtonSpec {
            id: "color",
            label: "Text and background color",
            glyph: "A",
            action: ToolbarAction::Color,
        },
        ToolbarButtonSpec {
            id: "bold",
            label: "Bold (Command-B)",
            glyph: "B",
            action: ToolbarAction::Toggle(PageTextAnnotation::Bold),
        },
        ToolbarButtonSpec {
            id: "italic",
            label: "Italic (Command-I)",
            glyph: "I",
            action: ToolbarAction::Toggle(PageTextAnnotation::Italic),
        },
        ToolbarButtonSpec {
            id: "underline",
            label: "Underline (Command-U)",
            glyph: "U",
            action: ToolbarAction::Toggle(PageTextAnnotation::Underline),
        },
        ToolbarButtonSpec {
            id: "clear",
            label: "Clear formatting",
            glyph: "Tx",
            action: ToolbarAction::Clear,
        },
    ]
}

fn toolbar_secondary_specs() -> [ToolbarButtonSpec; 3] {
    [
        ToolbarButtonSpec {
            id: "link",
            label: "Link (Command-K)",
            glyph: "↗",
            action: ToolbarAction::Link,
        },
        ToolbarButtonSpec {
            id: "strike",
            label: "Strike (Command-Shift-X)",
            glyph: "S̶",
            action: ToolbarAction::Toggle(PageTextAnnotation::Strike),
        },
        ToolbarButtonSpec {
            id: "code",
            label: "Code (Command-E)",
            glyph: "<>",
            action: ToolbarAction::Toggle(PageTextAnnotation::Code),
        },
    ]
}

struct PageRichTextRenderer {
    theme: Theme,
    appearance_mode: AppearanceMode,
    link_input: Option<gpui::Entity<TextInput>>,
    actions: ViewActionSink<ToolbarAction>,
}

struct PageRichTextToolbarLayout {
    anchor: Bounds<Pixels>,
    viewport_width: f32,
    viewport_height: f32,
    has_selection: bool,
    dialog: Option<PageRichTextDialog>,
}

impl PageRichTextRenderer {
    fn render_page_rich_text_toolbar(&self, layout: PageRichTextToolbarLayout) -> AnyElement {
        let PageRichTextToolbarLayout {
            anchor,
            viewport_width,
            viewport_height,
            has_selection,
            dialog,
        } = layout;
        let left = anchor
            .left()
            .as_f32()
            .clamp(8.0, (viewport_width - 224.0).max(8.0));
        let top = (anchor.bottom().as_f32() + 8.0).clamp(8.0, (viewport_height - 390.0).max(8.0));
        div()
            .absolute()
            .left(px(left))
            .top(px(top))
            .w(px(224.0))
            .p(px(16.0))
            .flex()
            .flex_col()
            .gap(px(8.0))
            .on_mouse_down(MouseButton::Left, |_: &MouseDownEvent, _, cx| {
                cx.stop_propagation()
            })
            .when(has_selection, |root| {
                root.child(self.render_page_rich_text_panel())
            })
            .when_some(dialog, |root, dialog| {
                root.child(self.render_page_rich_text_dialog(dialog))
            })
            .into_any_element()
    }

    fn render_page_rich_text_panel(&self) -> Div {
        self.page_rich_text_surface()
            .p(px(8.0))
            .flex()
            .flex_col()
            .gap(px(4.0))
            .child(self.render_normal_text_button())
            .child(self.render_toolbar_button_row(toolbar_primary_specs()))
            .child(self.render_toolbar_button_row(toolbar_secondary_specs()))
    }

    fn render_normal_text_button(&self) -> AnyElement {
        div()
            .id("notion-rich-text-normal-text")
            .w_full()
            .h(px(28.0))
            .px(px(8.0))
            .rounded(px(6.0))
            .role(Role::Button)
            .aria_label("Turn into normal text")
            .flex()
            .items_center()
            .cursor_pointer()
            .text_size(px(13.0))
            .text_color(rgb(self.theme.text_primary))
            .hover(|style| style.bg(alpha(self.theme.text_primary, 0.08)))
            .on_mouse_down(
                MouseButton::Left,
                self.actions.listener(|_: &MouseDownEvent, _, cx| {
                    cx.stop_propagation();
                    ToolbarAction::NormalText
                }),
            )
            .child("Normal Text")
            .into_any_element()
    }

    fn render_toolbar_button_row(&self, specs: impl IntoIterator<Item = ToolbarButtonSpec>) -> Div {
        div()
            .w_full()
            .h(px(32.0))
            .flex()
            .items_center()
            .gap(px(4.0))
            .children(
                specs
                    .into_iter()
                    .map(|spec| self.render_toolbar_button(spec)),
            )
    }

    fn render_toolbar_button(&self, spec: ToolbarButtonSpec) -> AnyElement {
        let action = spec.action;
        div()
            .id(ElementId::Name(
                format!("notion-rich-text-{}", spec.id).into(),
            ))
            .w(px(32.0))
            .h(px(28.0))
            .rounded(px(6.0))
            .role(Role::Button)
            .aria_label(spec.label)
            .flex()
            .items_center()
            .justify_center()
            .cursor_pointer()
            .text_size(px(13.0))
            .text_color(rgb(self.theme.text_primary))
            .hover(|style| style.bg(alpha(self.theme.text_primary, 0.08)))
            .on_mouse_down(
                MouseButton::Left,
                self.actions.listener(move |_: &MouseDownEvent, _, cx| {
                    cx.stop_propagation();
                    action.clone()
                }),
            )
            .child(spec.glyph)
            .into_any_element()
    }

    fn render_page_rich_text_dialog(&self, dialog: PageRichTextDialog) -> AnyElement {
        match dialog {
            PageRichTextDialog::Link => self.render_page_rich_text_link_dialog(),
            PageRichTextDialog::Color => self.render_page_rich_text_color_menu(),
        }
    }

    fn render_page_rich_text_link_dialog(&self) -> AnyElement {
        self.page_rich_text_surface()
            .p(px(8.0))
            .flex()
            .flex_col()
            .gap(px(8.0))
            .when_some(self.link_input.clone(), |panel, input| panel.child(input))
            .child(
                div()
                    .flex()
                    .justify_end()
                    .gap(px(6.0))
                    .child(self.render_link_dialog_button("Remove", false))
                    .child(self.render_link_dialog_button("Apply", true)),
            )
            .into_any_element()
    }

    fn render_link_dialog_button(&self, label: &'static str, apply: bool) -> AnyElement {
        div()
            .id(ElementId::Name(
                format!("notion-rich-text-link-{}", label.to_lowercase()).into(),
            ))
            .px(px(9.0))
            .h(px(28.0))
            .rounded(px(6.0))
            .role(Role::Button)
            .aria_label(label)
            .flex()
            .items_center()
            .cursor_pointer()
            .text_size(px(12.0))
            .text_color(rgb(self.theme.text_primary))
            .hover(|style| style.bg(alpha(self.theme.text_primary, 0.08)))
            .on_mouse_down(
                MouseButton::Left,
                self.actions.listener(move |_: &MouseDownEvent, _, cx| {
                    cx.stop_propagation();
                    if apply {
                        ToolbarAction::ApplyLink
                    } else {
                        ToolbarAction::RemoveLink
                    }
                }),
            )
            .child(label)
            .into_any_element()
    }

    fn page_rich_text_surface(&self) -> Div {
        div()
            .w(px(192.0))
            .rounded(px(14.0))
            .bg(rgb(self.theme.elevated_surface_bg))
            .shadow(page_rich_text_shadow(self.appearance_mode))
    }
}

impl PageEditorState {
    fn page_rich_text_anchor(&self, cx: &App) -> Option<Bounds<Pixels>> {
        let (block_id, offset) = if let Some(selection) = self.page_text_selection.as_ref() {
            (&selection.focus_block_id, selection.focus_offset)
        } else {
            let block_id = self.active_page_block.as_ref()?;
            let input = self
                .input
                .resource_state()
                .block_inputs
                .borrow()
                .get(block_id)?
                .clone();
            let offset = input.read(cx).selection_range().end;
            (block_id, offset)
        };
        let input = self
            .input
            .resource_state()
            .block_inputs
            .borrow()
            .get(block_id)?
            .clone();
        let text_len = input.read(cx).text().len();
        input
            .read(cx)
            .window_bounds_for_byte_range(offset.min(text_len)..offset.min(text_len))
    }
}
