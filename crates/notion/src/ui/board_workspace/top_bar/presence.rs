use super::TopBarRenderer;
use crate::model::{PagePresenceProfile, PagePresenceSnapshot};
use crate::ui::{div, img, px, rgb, Div, StyledImage};
use gpui::prelude::FluentBuilder;
use gpui::{FontWeight, IntoElement, ObjectFit, ParentElement, Styled};

impl TopBarRenderer<'_> {
    pub(crate) fn render_top_bar_presence(&self, presence: &PagePresenceSnapshot) -> Div {
        let mut row = div().h(px(28.0)).flex().items_center();
        for (index, profile) in presence.profiles.iter().enumerate() {
            row = row.child(self.render_top_bar_presence_avatar(profile, index));
        }
        row.when(presence.overflow_count > 0, |this| {
            this.child(
                div()
                    .h(px(24.0))
                    .ml(px(4.0))
                    .mr(px(-10.0))
                    .px(px(8.0))
                    .rounded(px(6.0))
                    .flex()
                    .items_center()
                    .text_size(px(14.0))
                    .line_height(px(16.8))
                    .text_color(rgb(0x8e8b86))
                    .child(format!("+{}", presence.overflow_count)),
            )
        })
    }

    fn render_top_bar_presence_avatar(&self, profile: &PagePresenceProfile, index: usize) -> Div {
        let avatar = div()
            .relative()
            .size(px(22.0))
            .when(index > 0, |this| this.ml(px(-6.0)))
            .rounded_full()
            .border_1()
            .border_color(rgb(self.theme.app_bg))
            .overflow_hidden()
            .flex()
            .items_center()
            .justify_center()
            .bg(rgb(presence_avatar_color(&profile.user_id)))
            .text_size(px(10.0))
            .line_height(px(12.0))
            .font_weight(FontWeight::MEDIUM)
            .text_color(rgb(0xffffff))
            .child(presence_initial(&profile.name));
        match self
            .presence_images
            .get(index)
            .and_then(|rendered| rendered.as_ref())
        {
            Some(rendered) => avatar.child(
                img(rendered.clone())
                    .absolute()
                    .inset_0()
                    .size_full()
                    .object_fit(ObjectFit::Cover),
            ),
            None => avatar.when_some(profile.profile_photo.clone(), |avatar, url| {
                avatar.child(
                    img(url)
                        .absolute()
                        .inset_0()
                        .size_full()
                        .object_fit(ObjectFit::Cover)
                        .with_loading(|| div().size_full().into_any_element())
                        .with_fallback(|| div().size_full().into_any_element()),
                )
            }),
        }
    }
}

fn presence_initial(name: &str) -> String {
    name.chars()
        .find(|character| !character.is_whitespace())
        .map(|character| character.to_uppercase().collect())
        .unwrap_or_else(|| "?".to_string())
}

fn presence_avatar_color(user_id: &str) -> u32 {
    const COLORS: [u32; 8] = [
        0x4f83cc, 0x5a9b72, 0xb36b5e, 0x8b6eb7, 0xc28a42, 0x4f9696, 0xb15e87, 0x737a89,
    ];
    let hash = user_id.bytes().fold(0usize, |hash, byte| {
        hash.wrapping_mul(31).wrapping_add(usize::from(byte))
    });
    COLORS[hash % COLORS.len()]
}
