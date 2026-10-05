use super::super::callbacks::{page_title_on_change, page_title_on_focus, page_title_on_submit};
use super::super::support::page_title_text_input_style;
use super::super::{
    div, AnyElement, App, AppContext, CardPage, Context, ElementId, FontWeight, IntoElement,
    ParentElement, Styled, SurfaceState, TextInput, TextInputAction, TextInputEnterBehavior,
    TextInputMode, TextInputProps, TextInputStateChange,
};
use crate::ui::surface::PageInputResources;
use crate::ui::Theme;

#[derive(Clone)]
pub(in crate::ui::board_workspace) struct PageTitleRenderer {
    theme: Theme,
    inputs: PageInputResources,
    callbacks: PageTitleCallbacks,
}

#[derive(Clone)]
struct PageTitleCallbacks {
    on_change: TextInputStateChange,
    on_focus: TextInputAction,
    on_submit: TextInputAction,
}

impl PageTitleRenderer {
    pub(in crate::ui::board_workspace) fn new(
        theme: Theme,
        inputs: PageInputResources,
        page_id: String,
        cx: &Context<SurfaceState>,
    ) -> Self {
        Self {
            theme,
            inputs,
            callbacks: PageTitleCallbacks {
                on_change: page_title_on_change(page_id.clone(), cx),
                on_focus: page_title_on_focus(cx),
                on_submit: page_title_on_submit(page_id, cx),
            },
        }
    }

    pub(in crate::ui::board_workspace) fn render(
        &self,
        page: &CardPage,
        font_size: f32,
        cx: &mut App,
    ) -> AnyElement {
        let page_id = page.block_id.clone();
        let props = TextInputProps::multiline(page.title.clone())
            .placeholder("Untitled")
            .mode(TextInputMode::Multiline {
                max_visible_lines: None,
            })
            .fill_width(true)
            .wrap_at_hyphens(true)
            .style(page_title_text_input_style(
                font_size,
                page.format,
                self.theme,
            ))
            .font_weight(FontWeight::BOLD)
            .bordered(false)
            .enter_behavior(TextInputEnterBehavior::SubmitOnEnter)
            .accessibility(
                ElementId::Name(format!("notion-page-title-{page_id}").into()),
                "Page title",
            )
            .on_change_with_state(self.callbacks.on_change.clone())
            .on_focus(self.callbacks.on_focus.clone())
            .on_submit(self.callbacks.on_submit.clone());
        let input = self
            .inputs
            .state()
            .title_inputs
            .borrow()
            .get(&page_id)
            .cloned();
        let input = if let Some(input) = input {
            input.update(cx, |input, cx| input.apply_props(props, cx));
            input
        } else {
            let input = cx.new(|cx| TextInput::new(props, cx));
            self.inputs
                .state()
                .title_inputs
                .borrow_mut()
                .insert(page_id, input.clone());
            input
        };
        div().w_full().flex().child(input).into_any_element()
    }
}
