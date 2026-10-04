use super::super::{
    alpha, div, img, px, render_svg_image, rgb, svg_from_body, AnyElement, FluentBuilder,
    InteractiveElement, IntoElement, MouseButton, MouseDownEvent, ParentElement, Styled,
};
use super::{NotionAiAction, NotionAiRenderer};
use gpui::App;

const AI_TEXT_PRIMARY: u32 = 0x2c2c2b;

impl NotionAiRenderer<'_> {
    pub(super) fn render_notion_ai_suggestions(&self, cx: &mut App) -> AnyElement {
        let suggestions = [
            ("Personalize your Notion AI", false),
            ("Create HTML", true),
            ("Translate this page", false),
            ("Analyze for insights", false),
        ];
        div()
            .mt(px(22.0))
            .flex()
            .flex_col()
            .gap(px(7.0))
            .children(
                suggestions
                    .into_iter()
                    .enumerate()
                    .map(|(index, (label, is_new))| {
                        self.render_notion_ai_suggestion(index, label, is_new, cx)
                    }),
            )
            .into_any_element()
    }

    fn render_notion_ai_suggestion(
        &self,
        index: usize,
        label: &str,
        is_new: bool,
        cx: &mut App,
    ) -> AnyElement {
        let prompt = label.to_string();
        div()
            .id(format!("notion-ai-suggestion-{index}"))
            .h(px(28.0))
            .px(px(8.0))
            .rounded(px(6.0))
            .flex()
            .items_center()
            .gap(px(8.0))
            .cursor_pointer()
            .hover(|style| style.bg(alpha(0x2a1c00, 0.055)))
            .on_mouse_down(
                MouseButton::Left,
                self.actions.listener(move |_: &MouseDownEvent, _, _| {
                    NotionAiAction::SetPrompt(prompt.clone())
                }),
            )
            .child(self.render_notion_ai_suggestion_icon(index, cx))
            .child(
                div()
                    .text_size(px(14.0))
                    .line_height(px(20.0))
                    .text_color(rgb(AI_TEXT_PRIMARY))
                    .child(label.to_string()),
            )
            .when(is_new, |this| {
                this.child(
                    div()
                        .ml(px(2.0))
                        .h(px(20.0))
                        .px(px(4.0))
                        .rounded(px(4.0))
                        .bg(alpha(0x2383e2, 0.1))
                        .flex()
                        .items_center()
                        .text_size(px(12.0))
                        .text_color(rgb(0x2383e2))
                        .child("New"),
                )
            })
            .into_any_element()
    }

    fn render_notion_ai_suggestion_icon(&self, index: usize, cx: &mut App) -> AnyElement {
        let body = match index {
            0 => {
                r##"<path d="M8.04761 1.40332L8.29565 1.41797C9.5354 1.51917 10.7877 2.05748 11.5691 3.03418C12.4248 4.10414 12.576 5.59336 12.2527 6.84082C12.1648 7.17964 12.0368 7.51547 11.8699 7.83496C12.4573 7.95393 13.014 8.1496 13.5232 8.44336C14.1514 8.80586 14.6705 9.29599 15.0779 9.90723C15.4453 9.55352 15.8874 9.31134 16.3923 9.24805H16.3933C17.0674 9.16392 17.7001 9.43393 18.1248 9.95508C18.5282 10.4502 18.698 11.1051 18.698 11.79C18.698 12.8535 18.6409 14.7357 17.2517 16.2207C15.8442 17.7246 13.3476 18.5467 9.17065 18.2373C7.03585 18.0791 5.40971 17.5784 4.25854 16.792C3.08147 15.9878 2.43669 14.9037 2.28296 13.7256C2.05362 11.9659 2.93173 10.2243 4.21851 9.04883C3.75332 9.08904 3.22319 9.06701 2.60327 9.00293H2.60229C2.07328 8.94808 1.55858 8.78222 1.15112 8.47656L0.983154 8.33691C0.529882 7.91784 0.255671 7.2623 0.478271 6.55469C0.679436 5.91656 1.20941 5.4862 1.77222 5.2207C2.19407 5.0218 2.70477 4.87921 3.29761 4.79395C3.48184 4.15662 3.79961 3.54947 4.24194 3.01855C5.2426 1.81786 6.70976 1.35719 8.04761 1.40332Z" fill="white" stroke="white" stroke-width="1.2"/><path d="M2.71145 7.95867C0.896282 7.771 0.784523 5.97447 4.07815 5.7696C4.79543 6.02556 6.62315 6.09221 5.94924 6.85552C5.10685 7.80966 4.8839 8.18328 2.71145 7.95867Z" fill="white" stroke="black"/><path d="M9.24829 17.1903C1.14829 16.5903 2.64829 10.8903 5.94829 9.09026C3.84832 7.89022 3.66342 5.35224 5.04832 3.69029C6.54832 1.89022 9.54832 2.19024 10.7483 3.69024C11.9483 5.19024 11.3483 8.19021 9.24829 8.79026C12.2483 8.49026 14.3483 9.39026 14.9483 12.3903C15.5483 9.69026 17.6483 9.69019 17.6483 11.7902C17.6483 13.8902 17.3483 17.7903 9.24829 17.1903Z" fill="#FFB110" stroke="black" stroke-linejoin="round"/><path d="M5.76867 6.1671C5.52984 5.87387 5.90069 4.67884 6.46816 5.14099C7.11453 5.6674 6.37472 6.91121 5.76867 6.1671Z" fill="black"/>"##
            }
            1 => {
                r##"<path d="M5.25 3.125A2.125 2.125 0 0 0 3.125 5.25v9.5c0 1.174.951 2.125 2.125 2.125H7.5v-1.25H5.25a.875.875 0 0 1-.875-.875v-9.5c0-.483.392-.875.875-.875h9.5c.483 0 .875.392.875.875V7.5h1.25V5.25a2.125 2.125 0 0 0-2.125-2.125zm9.667 10.362a.827.827 0 1 0 .264-1.632.827.827 0 0 0-.264 1.632"/><path d="M11.508 12.097a.827.827 0 1 1-1.633-.264.827.827 0 0 1 1.633.264m1.716-1.807a2.935 2.935 0 0 1 5.052.026.625.625 0 1 1-1.08.63 1.685 1.685 0 0 0-2.9-.014L10.65 17.02l2.67.432a.625.625 0 0 1-.2 1.234l-3.58-.58a.625.625 0 0 1-.436-.938zm-.86-1.802a2.94 2.94 0 0 0-3.464.329.625.625 0 0 0 .833.932 1.686 1.686 0 0 1 2.397.153.625.625 0 0 0 .945-.819 3 3 0 0 0-.71-.595"/>"##
            }
            2 => {
                r##"<path d="M14.776 5.217a.625.625 0 1 0-1.25 0v.967H9.524a.625.625 0 1 0 0 1.25h6.688c-.32.87-.963 2.091-2.06 3.375a12.8 12.8 0 0 1-1.48-2.122.625.625 0 1 0-1.095.603c.415.754.978 1.589 1.717 2.438a16.3 16.3 0 0 1-3.341 2.512.625.625 0 0 0 .62 1.085 17.6 17.6 0 0 0 3.58-2.688 17.6 17.6 0 0 0 3.577 2.688.625.625 0 1 0 .622-1.085 16.3 16.3 0 0 1-3.342-2.512c1.42-1.632 2.196-3.216 2.52-4.294h1.251a.625.625 0 1 0 0-1.25h-4.005zm-8.014 7.16.958 2.62a.625.625 0 0 0 1.174-.43L5.645 5.683a.94.94 0 0 0-1.765 0L.632 14.568a.625.625 0 1 0 1.174.43l.958-2.621zm-.457-1.25H3.221l1.542-4.219z"/>"##
            }
            3 => {
                r##"<path d="M8.387 5.933c.115-.523.861-.523.976 0l.202.918a1.75 1.75 0 0 0 1.334 1.334l.918.202c.523.115.523.861 0 .976l-.918.202a1.75 1.75 0 0 0-1.334 1.334l-.202.918c-.115.523-.861.523-.976 0l-.202-.918a1.75 1.75 0 0 0-1.334-1.334l-.918-.202c-.523-.115-.523-.861 0-.976l.918-.202a1.75 1.75 0 0 0 1.334-1.334z"/><path d="M8.875 2.625a6.25 6.25 0 1 0 3.955 11.09l3.983 3.982a.625.625 0 1 0 .884-.884l-3.983-3.982a6.25 6.25 0 0 0-4.84-10.205m-5 6.25a5 5 0 1 1 10 0 5 5 0 0 1-10 0"/>"##
            }
            _ => unreachable!("Notion AI suggestion icon index must be in the inventory"),
        };
        let body = if index == 0 {
            body.to_string()
        } else {
            body.replace("/>", &format!(" fill=\"#{AI_TEXT_PRIMARY:06x}\"/>"))
        };
        img(render_svg_image(svg_from_body("0 0 20 20", body), cx))
            .size(px(20.0))
            .into_any_element()
    }
}
