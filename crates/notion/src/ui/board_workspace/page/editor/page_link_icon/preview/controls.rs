use gpui::{App, Window};

use super::super::{PageLinkIconController, PageLinkIconPickerTab};

impl PageLinkIconController {
    pub(in crate::ui::board_workspace::page::editor::page_link_icon) fn clear_upload_preview(
        &mut self,
        cx: &mut App,
    ) -> bool {
        let Some(picker) = self.picker.as_mut() else {
            return false;
        };
        if picker.tab != PageLinkIconPickerTab::Upload || picker.upload_committed {
            return false;
        }
        picker.upload_generation = picker.upload_generation.wrapping_add(1);
        picker.upload_preview = None;
        picker.upload_pending = false;
        picker.upload_committed = false;
        picker.upload_add_to_library = false;
        picker.upload_name.clear();
        picker.upload_name_input.update(cx, |input, cx| {
            input.set_text(String::new(), cx);
        });
        true
    }

    pub(in crate::ui::board_workspace::page::editor::page_link_icon) fn toggle_upload_library(
        &mut self,
        window: &mut Window,
        cx: &mut App,
    ) -> bool {
        let Some(picker) = self.picker.as_mut() else {
            return false;
        };
        if picker.tab != PageLinkIconPickerTab::Upload
            || picker.upload_pending
            || picker.upload_preview.is_none()
            || !picker.custom_emoji_creation_allowed
            || picker.custom_emoji_limit_reached()
        {
            return false;
        }
        picker.upload_add_to_library = !picker.upload_add_to_library;
        if picker.upload_add_to_library {
            picker
                .upload_name_input
                .read(cx)
                .focus_handle_clone()
                .focus(window, cx);
        }
        true
    }

    pub(in crate::ui::board_workspace::page::editor::page_link_icon) fn set_upload_name(
        &mut self,
        value: String,
        cx: &mut App,
    ) -> bool {
        let Some(picker) = self.picker.as_mut() else {
            return false;
        };
        if picker.tab != PageLinkIconPickerTab::Upload
            || picker.upload_committed
            || !picker.upload_add_to_library
        {
            return false;
        }
        let value = value.replace(' ', "-");
        if picker.upload_name == value {
            return false;
        }
        picker.upload_name.clone_from(&value);
        if picker.upload_name_input.read(cx).text() != value {
            picker.upload_name_input.update(cx, |input, cx| {
                input.set_text(value, cx);
            });
        }
        true
    }
}
