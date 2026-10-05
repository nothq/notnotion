use std::sync::Arc;

use gpui::{px, size, Bounds, Pixels, Point, SharedString};

use crate::model::{BoardSnapshot, CardPageProperty, DatabaseStatusProperty};
use crate::ui::surface::{
    StatusPropertyPickerSource, StatusPropertyPickerState, StatusPropertyPickerTarget,
};

impl StatusPropertyPickerState {
    pub(super) fn for_table(
        board: &BoardSnapshot,
        page_id: SharedString,
        property_id: SharedString,
        current_value: SharedString,
        position: Point<Pixels>,
    ) -> Result<Option<Self>, String> {
        let Some(property) = board
            .database_properties
            .iter()
            .find(|property| property.property_id == property_id.as_ref())
        else {
            return Ok(None);
        };
        let property = DatabaseStatusProperty::try_from(property)?;
        let expected_view_id = board
            .view_tabs
            .iter()
            .find(|view| view.active)
            .map(|view| view.provider_view_id.clone())
            .ok_or_else(|| "status editing requires an active database view".to_string())?;
        let current_option_id = property
            .option_id_for_value(&current_value)
            .map(SharedString::from);
        Ok(Some(Self::new(
            StatusPropertyPickerTarget::DatabaseView {
                page_id,
                expected_view_id,
            },
            property,
            current_option_id,
            position,
        )))
    }

    pub(super) fn for_page(
        page_id: SharedString,
        property: CardPageProperty,
        position: Point<Pixels>,
    ) -> Result<Self, String> {
        let schema = DatabaseStatusProperty::try_from(&property)?;
        let current_option_id = schema
            .option_id_for_value(&property.value)
            .map(SharedString::from);
        Ok(Self::new(
            StatusPropertyPickerTarget::LoadedPage { page_id },
            schema,
            current_option_id,
            position,
        ))
    }

    fn new(
        target: StatusPropertyPickerTarget,
        property: DatabaseStatusProperty,
        current_option_id: Option<SharedString>,
        position: Point<Pixels>,
    ) -> Self {
        Self {
            session: Arc::new(()),
            anchor: Bounds::new(position, size(px(1.0), px(1.0))),
            source: StatusPropertyPickerSource::Current,
            target,
            property,
            current_option_id,
            commit_in_flight: false,
        }
    }
}
