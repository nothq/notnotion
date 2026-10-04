use super::{
    inline_database_view_menu_options, Context, InlineDatabaseViewMenuSelection, SurfaceState,
};
use crate::ui::surface::NotionChromeState;

impl NotionChromeState {
    pub(super) fn set_inline_database_view_menu_query(
        &mut self,
        query: String,
        cx: &mut Context<SurfaceState>,
    ) {
        let Some(state) = self.inline_database_view_menu.as_mut() else {
            return;
        };
        state.query = query.into();
        let options = inline_database_view_menu_options(state);
        if !options.contains(&state.highlighted) {
            state.highlighted = options
                .into_iter()
                .next()
                .expect("inline database view menu always has creation actions");
        }
        state.new_view_picker_open = false;
        cx.notify();
    }

    pub(super) fn highlight_inline_database_view_menu_selection(
        &mut self,
        selection: InlineDatabaseViewMenuSelection,
        cx: &mut Context<SurfaceState>,
    ) {
        let Some(state) = self.inline_database_view_menu.as_mut() else {
            return;
        };
        if state.highlighted == selection {
            return;
        }
        state.highlighted = selection;
        cx.notify();
    }

    pub(super) fn move_inline_database_view_menu_highlight(
        &mut self,
        delta: isize,
        cx: &mut Context<SurfaceState>,
    ) {
        let Some(state) = self.inline_database_view_menu.as_mut() else {
            return;
        };
        let options = inline_database_view_menu_options(state);
        let current = options
            .iter()
            .position(|option| option == &state.highlighted)
            .unwrap_or(0);
        let next = (current as isize + delta).rem_euclid(options.len() as isize) as usize;
        state.highlighted = options[next].clone();
        state.new_view_picker_open = false;
        cx.notify();
    }

    pub(super) fn move_inline_database_view_menu_highlight_to_edge(
        &mut self,
        end: bool,
        cx: &mut Context<SurfaceState>,
    ) {
        let Some(state) = self.inline_database_view_menu.as_mut() else {
            return;
        };
        let options = inline_database_view_menu_options(state);
        state.highlighted = if end {
            options
                .last()
                .expect("inline database view menu always has creation actions")
                .clone()
        } else {
            options
                .first()
                .expect("inline database view menu always has creation actions")
                .clone()
        };
        state.new_view_picker_open = false;
        cx.notify();
    }

    pub(super) fn activate_inline_database_view_menu_highlight(
        &mut self,
        cx: &mut Context<SurfaceState>,
    ) {
        let Some(selection) = self
            .inline_database_view_menu
            .as_ref()
            .map(|state| state.highlighted.clone())
        else {
            return;
        };
        self.activate_inline_database_view_menu_selection(selection, cx);
    }

    pub(super) fn activate_inline_database_view_menu_selection(
        &mut self,
        selection: InlineDatabaseViewMenuSelection,
        cx: &mut Context<SurfaceState>,
    ) {
        match selection {
            InlineDatabaseViewMenuSelection::View(provider_view_id) => {
                let state = self
                    .inline_database_view_menu
                    .take()
                    .expect("view menu selection requires open state");
                state
                    .inline_view
                    .update(cx, move |view, cx| {
                        view.select_provider_view(provider_view_id, cx);
                    })
                    .expect("inline database view selection requires its inline host");
                cx.notify();
            }
            InlineDatabaseViewMenuSelection::NewView => {
                let state = self
                    .inline_database_view_menu
                    .as_mut()
                    .expect("new view action requires open state");
                state.highlighted = InlineDatabaseViewMenuSelection::NewView;
                state.new_view_picker_open = true;
                cx.notify();
            }
            InlineDatabaseViewMenuSelection::NewDataSource => {
                let state = self
                    .inline_database_view_menu
                    .as_mut()
                    .expect("new data source action requires open state");
                state.highlighted = InlineDatabaseViewMenuSelection::NewDataSource;
                cx.notify();
            }
        }
    }

    pub(super) fn escape_inline_database_view_menu(&mut self, cx: &mut Context<SurfaceState>) {
        let Some(state) = self.inline_database_view_menu.as_mut() else {
            return;
        };
        if state.new_view_picker_open {
            state.new_view_picker_open = false;
            cx.notify();
            return;
        }
        self.dismiss_inline_database_view_menu(cx);
    }
}
