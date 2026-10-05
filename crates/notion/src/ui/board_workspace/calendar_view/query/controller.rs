use super::types::{DateViewContext, DateViewEffect, DateViewRequest};
use crate::model::BoardItem;
use crate::ui::surface::NotionDateViewState;

impl NotionDateViewState {
    pub(in crate::ui::board_workspace) fn ensure_query_effects(
        &mut self,
        context: &DateViewContext,
        board_items: &[BoardItem],
    ) -> Vec<DateViewEffect> {
        let Some(view_identity) = context.active_identity() else {
            return self.reset_inactive_query_effects(board_items, context.workspace_available);
        };
        let month =
            self.ensure_calendar_items(view_identity, board_items, context.calendar_is_active());
        let mut effects = if self.undated.view_identity.as_deref() == Some(view_identity) {
            Vec::new()
        } else {
            self.reset_undated_for_view_effects(
                Some(view_identity.to_string()),
                context.workspace_available,
            )
        };
        if let Some(month) = month {
            effects.push(DateViewEffect::Request(Box::new(
                DateViewRequest::CalendarMonth(month),
            )));
        }
        if let Some(effect) = self.undated.prepare_count_effect(
            view_identity.to_string(),
            self.query_session.clone(),
            context.workspace_available,
        ) {
            effects.push(effect);
        }
        effects
    }

    pub(in crate::ui::board_workspace) fn refresh_query_effects(
        &mut self,
        context: &DateViewContext,
        board_items: &[BoardItem],
    ) -> Vec<DateViewEffect> {
        let Some(view_identity) = context.active_identity() else {
            return self.reset_inactive_query_effects(board_items, context.workspace_available);
        };
        if self.calendar_items.view_identity.as_deref() != Some(view_identity) {
            self.reset_calendar_items(Some(view_identity.to_string()), board_items);
        }
        let mut effects = if self.undated.view_identity.as_deref() == Some(view_identity) {
            Vec::new()
        } else {
            self.reset_undated_for_view_effects(Some(view_identity.to_string()), false)
        };
        self.undated.reset_count_request();
        effects.push(DateViewEffect::ClearUndatedCount);
        if context.calendar_is_active() {
            effects.push(DateViewEffect::Request(Box::new(
                DateViewRequest::CalendarMonth(self.visible_month),
            )));
        }
        if self.undated.open {
            let limit = self.undated.limit;
            if let Some(effect) =
                self.undated
                    .prepare_query_effect(context, self.query_session.clone(), limit, false)
            {
                effects.push(effect);
            }
        }
        if let Some(effect) = self.undated.prepare_count_effect(
            view_identity.to_string(),
            self.query_session.clone(),
            context.workspace_available,
        ) {
            effects.push(effect);
        }
        effects
    }

    pub(in crate::ui::board_workspace) fn reset_inactive_query_effects(
        &mut self,
        board_items: &[BoardItem],
        workspace_available: bool,
    ) -> Vec<DateViewEffect> {
        if self.calendar_items.view_identity.is_none() && self.undated.view_identity.is_none() {
            return Vec::new();
        }
        self.reset_calendar_items(None, board_items);
        self.reset_undated_for_view_effects(None, workspace_available)
    }

    pub(in crate::ui::board_workspace) fn reset_undated_for_view_effects(
        &mut self,
        view_identity: Option<String>,
        clear_count: bool,
    ) -> Vec<DateViewEffect> {
        let session = self.query_session.clone();
        self.undated.reset_for_view(view_identity);
        let mut effects = vec![DateViewEffect::ClearUndatedDialog(session)];
        if clear_count {
            effects.insert(0, DateViewEffect::ClearUndatedCount);
        }
        effects
    }
}
