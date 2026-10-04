use std::sync::Arc;

use gpui::{App, AppContext, ElementId, IntoElement};
use gpui_components::text_input::{
    TextInput, TextInputAction, TextInputEnterBehavior, TextInputKeyAction, TextInputMode,
    TextInputProps, TextInputStateChange,
};

use super::super::{
    div, page_command_placeholder, page_composer_numbered_index, px, render_page_link_marker,
    render_page_toggle_marker, AppearanceMode, CardPageBlockColor, CardPageBlockKind,
    CardPageEditableBlock, Context, FluentBuilder, InteractiveElement, LoadedCardPageData,
    ParentElement, Styled, SurfaceState, Theme,
};
use super::commands::render::PageCommandRenderer;
use super::commands::state::{
    page_composer_on_backspace_when_empty, page_composer_on_change, page_composer_on_escape,
    page_composer_on_focus, page_composer_on_move, page_composer_on_submit,
};
use super::editor::{
    page_block_default_row_spacing, page_block_input_spec, page_block_text_input_style,
    render_page_block_editable_content, NumberedEditableBlock, PageBlockInputSpec,
    PageBlockVisualStyle, PageRenderAction,
};
use crate::model::CardPage;
use crate::ui::surface::{PageEditorState, PageInputResources};
use crate::ui::view_actions::ViewActionSink;
use crate::ui::{IconSet, PageComposerState};

#[derive(Clone)]
pub(crate) struct PageComposerRenderer {
    theme: Theme,
    appearance_mode: AppearanceMode,
    icons: Arc<IconSet>,
    inputs: PageInputResources,
    composer: PageComposerState,
    callbacks: PageComposerCallbacks,
    command_menu: Option<PageCommandRenderer>,
}

pub(crate) struct PageComposerVisualResources {
    theme: Theme,
    appearance_mode: AppearanceMode,
    icons: Arc<IconSet>,
}

#[derive(Clone)]
struct PageComposerCallbacks {
    on_change: TextInputStateChange,
    on_submit: TextInputKeyAction,
    on_escape: TextInputAction,
    on_focus: TextInputAction,
    on_backspace: TextInputAction,
    on_up: TextInputAction,
    on_down: TextInputAction,
}

impl PageComposerRenderer {
    pub(crate) fn capture(
        editor: &PageEditorState,
        visual: PageComposerVisualResources,
        page_id: &str,
        actions: ViewActionSink<PageRenderAction>,
        cx: &Context<SurfaceState>,
    ) -> Self {
        let commands = editor.filtered_page_commands();
        let selected_row_index = editor.selected_page_command_row_index(&commands);
        let composer = editor.input.composer.clone();
        let command_menu = (composer.page_id.as_deref() == Some(page_id)
            && composer.slash_command_open)
            .then(|| {
                PageCommandRenderer::new(
                    visual.theme,
                    Arc::clone(&visual.icons),
                    actions,
                    commands.into(),
                    selected_row_index,
                )
            });
        Self {
            theme: visual.theme,
            appearance_mode: visual.appearance_mode,
            icons: visual.icons,
            inputs: editor.input.resources(),
            composer,
            callbacks: PageComposerCallbacks {
                on_change: page_composer_on_change(page_id.to_string(), cx),
                on_submit: page_composer_on_submit(page_id.to_string(), cx),
                on_escape: page_composer_on_escape(page_id.to_string(), cx),
                on_focus: page_composer_on_focus(page_id.to_string(), cx),
                on_backspace: page_composer_on_backspace_when_empty(page_id.to_string(), cx),
                on_up: page_composer_on_move(page_id.to_string(), -1, cx),
                on_down: page_composer_on_move(page_id.to_string(), 1, cx),
            },
            command_menu,
        }
    }

    pub(crate) fn render(&self, data: &LoadedCardPageData, cx: &mut App) -> gpui::AnyElement {
        let page_id = &data.page.block_id;
        let is_target = self.composer.page_id.as_deref() == Some(page_id.as_str());
        let kind = if is_target {
            self.composer.block_kind
        } else {
            CardPageBlockKind::Text
        };
        let spec = page_block_input_spec(kind, data.page.format);
        div()
            .relative()
            .w_full()
            .child(self.render_preview(data, kind, is_target, cx))
            .when_some(self.command_menu.as_ref(), |this, command_menu| {
                this.child(
                    div()
                        .absolute()
                        .left(px(0.0))
                        .top(px(spec.top_padding
                            + spec.line_height
                            + spec.input_padding_y * 2.0
                            + 3.0))
                        .occlude()
                        .child(command_menu.render(cx)),
                )
            })
            .into_any_element()
    }

    fn render_preview(
        &self,
        data: &LoadedCardPageData,
        kind: CardPageBlockKind,
        is_target: bool,
        cx: &mut App,
    ) -> gpui::Div {
        let editable = CardPageEditableBlock::new(
            kind,
            if is_target {
                self.composer.text.clone()
            } else {
                String::new()
            },
            Vec::new(),
        );
        let mut preview_composer = self.composer.clone();
        preview_composer.block_kind = kind;
        let numbered_index = page_composer_numbered_index(data, &preview_composer);
        let marker = match kind {
            CardPageBlockKind::ToggleList => Some(
                render_page_toggle_marker(self.icons.page_toggle_collapsed.render(cx))
                    .into_any_element(),
            ),
            CardPageBlockKind::PageLink => {
                Some(render_page_link_marker(self.icons.page.render(cx)).into_any_element())
            }
            _ => None,
        };
        render_page_block_editable_content(
            NumberedEditableBlock {
                editable: &editable,
                numbered_index,
            },
            PageBlockVisualStyle::new(
                self.theme,
                self.appearance_mode,
                CardPageBlockColor::default(),
            ),
            page_block_default_row_spacing(kind),
            marker,
            self.input(&data.page, kind, is_target, cx)
                .into_any_element(),
        )
    }

    fn input(
        &self,
        page: &CardPage,
        kind: CardPageBlockKind,
        is_target: bool,
        cx: &mut App,
    ) -> gpui::Entity<TextInput> {
        let page_id = page.block_id.as_str();
        let props = self.input_props(
            page_id,
            kind,
            page_block_input_spec(kind, page.format),
            is_target,
        );
        let existing = self
            .inputs
            .state()
            .composer_inputs
            .borrow()
            .get(page_id)
            .cloned();
        if let Some(input) = existing {
            input.update(cx, |input, cx| input.apply_props(props, cx));
            return input;
        }
        let input = cx.new(|cx| TextInput::new(props, cx));
        self.inputs
            .state()
            .composer_inputs
            .borrow_mut()
            .insert(page_id.to_string(), input.clone());
        input
    }

    fn input_props(
        &self,
        page_id: &str,
        kind: CardPageBlockKind,
        spec: PageBlockInputSpec,
        is_target: bool,
    ) -> TextInputProps {
        let request_focus = take_page_composer_focus_request(&self.inputs, page_id);
        let mut props = TextInputProps::multiline(if is_target {
            self.composer.text.clone()
        } else {
            String::new()
        })
        .placeholder(if is_target && self.composer.active {
            page_command_placeholder(kind)
        } else {
            ""
        })
        .mode(TextInputMode::Multiline {
            max_visible_lines: None,
        })
        .style(page_block_text_input_style(
            spec,
            self.theme,
            CardPageBlockColor::default(),
            self.appearance_mode,
        ))
        .font_weight(spec.font_weight)
        .bordered(false)
        .request_focus(request_focus)
        .enter_behavior(TextInputEnterBehavior::SubmitOnEnter)
        .accessibility(
            ElementId::Name(format!("notion-page-composer-{page_id}").into()),
            "New Notion block",
        )
        .on_change_with_state(self.callbacks.on_change.clone())
        .on_submit_with_state(self.callbacks.on_submit.clone())
        .on_escape(self.callbacks.on_escape.clone())
        .on_focus(self.callbacks.on_focus.clone())
        .on_backspace_when_empty(self.callbacks.on_backspace.clone());
        if is_target && self.composer.slash_command_open {
            props = props
                .on_up(self.callbacks.on_up.clone())
                .on_down(self.callbacks.on_down.clone());
        }
        props
    }
}

impl PageComposerVisualResources {
    pub(crate) fn new(theme: Theme, appearance_mode: AppearanceMode, icons: Arc<IconSet>) -> Self {
        Self {
            theme,
            appearance_mode,
            icons,
        }
    }
}

fn take_page_composer_focus_request(inputs: &PageInputResources, page_id: &str) -> bool {
    let matches = inputs.state().composer_focus_request.borrow().as_deref() == Some(page_id);
    if matches {
        inputs.state().composer_focus_request.borrow_mut().take();
    }
    matches
}
