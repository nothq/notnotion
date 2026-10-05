use super::{
    alpha, div, img, notion_ai_face_image, px, rgb, AiAutofillMarketingArt, AiAutofillView,
    AnyElement, AppearanceMode, Div, FontWeight, IntoElement, ParentElement, Styled, Theme,
};

impl AiAutofillView<'_> {
    pub(super) fn render_ai_autofill_trial_marketing(&self) -> AnyElement {
        div()
            .w(px(480.0))
            .h_full()
            .flex_none()
            .flex()
            .flex_col()
            .gap(px(12.0))
            .child(self.render_ai_autofill_marketing_row(
                (
                    "Custom Agents automate",
                    "work for your team",
                    AiAutofillMarketingArt::CustomAgents,
                ),
                (
                    "Eliminate manual",
                    "work with Notion AI",
                    AiAutofillMarketingArt::NotionAi,
                ),
            ))
            .child(self.render_ai_autofill_marketing_row(
                (
                    "Never miss a detail",
                    "with AI Meeting Notes",
                    AiAutofillMarketingArt::MeetingNotes,
                ),
                (
                    "Answers from Slack,",
                    "Github and more",
                    AiAutofillMarketingArt::Integrations,
                ),
            ))
            .into_any_element()
    }

    fn render_ai_autofill_marketing_row(
        &self,
        left: (&'static str, &'static str, AiAutofillMarketingArt),
        right: (&'static str, &'static str, AiAutofillMarketingArt),
    ) -> AnyElement {
        div()
            .h(px(258.5))
            .flex_none()
            .flex()
            .gap(px(12.0))
            .child(self.render_ai_autofill_marketing_card(left.0, left.1, left.2))
            .child(self.render_ai_autofill_marketing_card(right.0, right.1, right.2))
            .into_any_element()
    }

    fn render_ai_autofill_marketing_card(
        &self,
        first_line: &'static str,
        second_line: &'static str,
        art: AiAutofillMarketingArt,
    ) -> AnyElement {
        let card_background = match self.appearance_mode {
            AppearanceMode::Light => 0xffffff,
            AppearanceMode::Dark => 0x202020,
        };
        let card_border = match self.appearance_mode {
            AppearanceMode::Light => alpha(0x37352f, 0.12),
            AppearanceMode::Dark => alpha(0x2c2c2b, 1.0),
        };
        div()
            .relative()
            .w(px(234.0))
            .h(px(258.5))
            .flex_none()
            .overflow_hidden()
            .rounded(px(10.0))
            .border_1()
            .border_color(card_border)
            .bg(rgb(card_background))
            .child(marketing_card_title(
                first_line,
                second_line,
                self.theme.text_primary,
            ))
            .child(self.render_ai_autofill_marketing_art(art))
            .into_any_element()
    }

    fn render_ai_autofill_marketing_art(&self, art: AiAutofillMarketingArt) -> AnyElement {
        match art {
            AiAutofillMarketingArt::CustomAgents => custom_agents_art(),
            AiAutofillMarketingArt::NotionAi => notion_ai_art(*self.theme),
            AiAutofillMarketingArt::MeetingNotes => meeting_notes_art(),
            AiAutofillMarketingArt::Integrations => integrations_art(*self.theme),
        }
    }
}

fn marketing_card_title(first_line: &'static str, second_line: &'static str, color: u32) -> Div {
    div()
        .absolute()
        .top(px(41.0))
        .left(px(12.0))
        .right(px(12.0))
        .h(px(44.0))
        .flex()
        .flex_col()
        .items_center()
        .text_size(px(14.0))
        .line_height(px(20.0))
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(rgb(color))
        .child(first_line)
        .child(second_line)
}

fn custom_agents_art() -> AnyElement {
    div()
        .absolute()
        .top(px(112.0))
        .left(px(57.0))
        .w(px(118.0))
        .h(px(80.0))
        .child(
            div()
                .absolute()
                .left(px(0.0))
                .top(px(24.0))
                .size(px(36.0))
                .rounded_full()
                .bg(rgb(0xf64932))
                .flex()
                .items_center()
                .justify_center()
                .text_size(px(18.0))
                .text_color(rgb(0xffffff))
                .child("◆"),
        )
        .child(
            div()
                .absolute()
                .right(px(0.0))
                .top(px(24.0))
                .size(px(36.0))
                .rounded_full()
                .bg(rgb(0x097fe8))
                .flex()
                .items_center()
                .justify_center()
                .text_size(px(19.0))
                .text_color(rgb(0xffffff))
                .child("⌁"),
        )
        .child(
            img(notion_ai_face_image())
                .absolute()
                .left(px(31.0))
                .top(px(0.0))
                .size(px(58.0)),
        )
        .into_any_element()
}

fn notion_ai_art(theme: Theme) -> AnyElement {
    div()
        .absolute()
        .top(px(126.0))
        .left(px(10.0))
        .w(px(212.0))
        .h(px(52.0))
        .flex()
        .items_center()
        .gap(px(10.0))
        .child(img(notion_ai_face_image()).size(px(52.0)))
        .child(
            div()
                .w(px(150.0))
                .h(px(36.0))
                .rounded(px(18.0))
                .border_1()
                .border_color(rgb(0x2783de))
                .bg(rgb(theme.elevated_surface_bg))
                .px(px(12.0))
                .flex()
                .items_center()
                .text_size(px(14.0))
                .text_color(rgb(theme.text_hint))
                .child("Ask AI anything..."),
        )
        .into_any_element()
}

fn meeting_notes_art() -> AnyElement {
    div()
        .absolute()
        .top(px(130.0))
        .left(px(0.0))
        .w(px(234.0))
        .h(px(75.0))
        .child(
            div()
                .absolute()
                .left(px(0.0))
                .top(px(18.0))
                .w(px(133.0))
                .h(px(40.0))
                .flex()
                .items_center()
                .justify_between()
                .children(
                    [
                        4.0, 5.0, 7.0, 10.0, 14.0, 20.0, 27.0, 35.0, 28.0, 20.0, 13.0, 8.0,
                    ]
                    .map(|height| {
                        div()
                            .w(px(2.0))
                            .h(px(height))
                            .rounded_full()
                            .bg(rgb(0x777771))
                    }),
                ),
        )
        .child(
            img(notion_ai_face_image())
                .absolute()
                .left(px(132.0))
                .top(px(0.0))
                .size(px(75.0)),
        )
        .into_any_element()
}

fn integrations_art(theme: Theme) -> AnyElement {
    div()
        .absolute()
        .top(px(121.0))
        .left(px(12.0))
        .w(px(208.0))
        .h(px(48.0))
        .flex()
        .items_center()
        .gap(px(7.0))
        .child(ai_autofill_integration_badge(theme, "✣", 0x36c5f0))
        .child(ai_autofill_integration_badge(theme, "GH", 0xf0efed))
        .child(img(notion_ai_face_image()).size(px(48.0)))
        .child(ai_autofill_integration_badge(theme, "▲", 0x34a853))
        .child(ai_autofill_integration_badge(theme, "◢", 0x2684ff))
        .into_any_element()
}

fn ai_autofill_integration_badge(theme: Theme, label: &'static str, color: u32) -> Div {
    div()
        .size(px(36.0))
        .flex_none()
        .rounded_full()
        .border_1()
        .border_color(alpha(theme.text_primary, 0.10))
        .bg(rgb(theme.elevated_surface_bg))
        .flex()
        .items_center()
        .justify_center()
        .text_size(px(if label == "GH" { 10.0 } else { 17.0 }))
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(rgb(color))
        .child(label)
}
