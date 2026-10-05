use gpui::{Pixels, Point, SharedString};

use crate::model::CardPageProperty;
use crate::ui::surface::StatusPropertyPickerState;

pub(crate) enum StatusPropertyPickerAction {
    OpenTable(Box<OpenTableStatusPropertyPicker>),
    OpenPage(Box<OpenPageStatusPropertyPicker>),
    Dismiss,
    Select(SharedString),
    InstallHosted(Box<StatusPropertyPickerState>),
}

pub(crate) struct OpenTableStatusPropertyPicker {
    pub(crate) page_id: SharedString,
    pub(crate) property_id: SharedString,
    pub(crate) current_value: SharedString,
    pub(crate) position: Point<Pixels>,
}

pub(crate) struct OpenPageStatusPropertyPicker {
    pub(crate) page_id: SharedString,
    pub(crate) property: CardPageProperty,
    pub(crate) position: Point<Pixels>,
}

impl StatusPropertyPickerAction {
    pub(crate) fn open_table(
        page_id: SharedString,
        property_id: SharedString,
        current_value: SharedString,
        position: Point<Pixels>,
    ) -> Self {
        Self::OpenTable(Box::new(OpenTableStatusPropertyPicker {
            page_id,
            property_id,
            current_value,
            position,
        }))
    }

    pub(crate) fn open_page(
        page_id: SharedString,
        property: CardPageProperty,
        position: Point<Pixels>,
    ) -> Self {
        Self::OpenPage(Box::new(OpenPageStatusPropertyPicker {
            page_id,
            property,
            position,
        }))
    }
}
