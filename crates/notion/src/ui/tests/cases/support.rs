use crate::ui::board_workspace::PageDocumentAction;
use crate::ui::tests::cases::*;

type InteractionWindow = gpui::WindowHandle<SurfaceState>;

pub(crate) fn with_interaction_app(
    board: BoardSnapshot,
    pages: HashMap<String, CardPage>,
    assert_app: impl FnOnce(&mut SurfaceState, &mut gpui::Context<SurfaceState>),
) {
    with_interaction_window(board, pages, |window, cx| {
        window
            .update(cx, |app, _, cx| assert_app(app, cx))
            .expect("failed to update interaction test root");
    });
}

pub(crate) fn with_interaction_window(
    board: BoardSnapshot,
    pages: HashMap<String, CardPage>,
    interact: impl FnOnce(InteractionWindow, &mut HeadlessAppContext),
) {
    let _guard = acquire_headless_test_lock();
    let mut cx = headless_test_context();
    let window = cx
        .open_window(app_size(), |_, cx| {
            cx.new(move |_| {
                SurfaceState::fixture_snapshot(
                    board,
                    pages,
                    AppearanceMode::Dark,
                    Viewport::default(),
                )
            })
        })
        .expect("failed to open interaction test window");
    cx.run_until_parked();
    draw_interaction_window(window, &mut cx);
    interact(window, &mut cx);
}

pub(crate) fn create_interaction_page(
    window: InteractionWindow,
    cx: &mut HeadlessAppContext,
) -> String {
    let page_id = window
        .update(cx, |app, _, cx| {
            app.dispatch_page_document_action(PageDocumentAction::CreateInColumn(0), cx);
            app.columns[0]
                .cards
                .last()
                .expect("created card should exist")
                .block_id
                .clone()
        })
        .expect("create page for interaction");
    cx.run_until_parked();
    draw_interaction_window(window, cx);
    page_id
}

pub(crate) fn activate_interaction_page_composer(
    window: InteractionWindow,
    cx: &mut HeadlessAppContext,
) {
    let handled = window
        .update(cx, |app, _, cx| {
            app.handle_selected_page_key_down(&named_key_down_event("enter"), cx)
        })
        .expect("activate page composer through its keyboard handler");
    assert!(handled, "Enter should activate the selected page composer");
    cx.run_until_parked();
    draw_interaction_window(window, cx);
}

pub(crate) fn draw_interaction_window(window: InteractionWindow, cx: &mut HeadlessAppContext) {
    cx.update_window(window.into(), |_, window, cx| window.draw(cx).clear())
        .expect("draw interaction test window");
    cx.run_until_parked();
}

pub(crate) fn dispatch_interaction_keystroke(
    window: InteractionWindow,
    keystroke: &str,
    cx: &mut HeadlessAppContext,
) {
    let parsed = gpui::Keystroke::parse(keystroke).expect("interaction keystroke should parse");
    cx.update_window(window.into(), |_, window, cx| {
        window.dispatch_keystroke(parsed, cx)
    })
    .expect("dispatch interaction keystroke");
    cx.run_until_parked();
}

pub(crate) fn type_interaction_text(
    window: InteractionWindow,
    text: &str,
    cx: &mut HeadlessAppContext,
) {
    for character in text.chars() {
        let keystroke = character.to_string();
        dispatch_interaction_keystroke(window, &keystroke, cx);
    }
}

pub(crate) fn assert_native_slash_menu(
    window: InteractionWindow,
    expected_query: &str,
    expected_index: usize,
    expected_text: &str,
    cx: &HeadlessAppContext,
) {
    window
        .read_with(cx, |app, _| {
            let menu = app
                .page_editor
                .page_slash_menu
                .as_ref()
                .expect("native slash menu should remain open");
            assert_eq!(menu.query, expected_query);
            assert_eq!(menu.selected_command_index, expected_index);
            let appended_block = selected_page_last_block(app);
            assert_eq!(menu.block_id, appended_block.block_id);
            assert_eq!(
                appended_block
                    .editable_content()
                    .expect("slash input should materialize an editable page block")
                    .text,
                expected_text
            );
        })
        .expect("read native slash menu");
}

pub(crate) fn assert_native_slash_selection(
    window: InteractionWindow,
    expected_index: usize,
    cx: &HeadlessAppContext,
) {
    window
        .read_with(cx, |app, _| {
            let menu = app
                .page_editor
                .page_slash_menu
                .as_ref()
                .expect("native slash menu should remain open");
            assert_eq!(menu.selected_command_index, expected_index);
        })
        .expect("read native slash menu selection");
}

pub(crate) fn assert_native_slash_block(
    window: InteractionWindow,
    expected_kind: CardPageBlockKind,
    expected_text: &str,
    cx: &HeadlessAppContext,
) {
    window
        .read_with(cx, |app, _| {
            assert!(app.page_editor.page_slash_menu.is_none());
            let editable = selected_page_last_block(app)
                .editable_content()
                .expect("selected slash command should keep an editable page block");
            assert_eq!(editable.kind, expected_kind);
            assert_eq!(editable.text, expected_text);
        })
        .expect("read converted native slash block");
}

pub(crate) fn assert_persisted_interaction_block(
    window: InteractionWindow,
    created_page_id: &str,
    expected_kind: CardPageBlockKind,
    expected_text: &str,
    cx: &HeadlessAppContext,
) {
    window
        .read_with(cx, |app, _| {
            assert_committed_fixture_page_block(app, created_page_id, expected_kind, expected_text);
        })
        .expect("read persisted interaction block");
}

fn selected_page_last_block(app: &SurfaceState) -> &CardPageBlock {
    let Some(CardPeekState::Loaded(page)) = app.page_documents.selected_page.as_ref() else {
        panic!("selected page should stay loaded");
    };
    page.data
        .page
        .blocks
        .last()
        .expect("slash input should append a page block")
}

pub(crate) fn assert_committed_fixture_page_block(
    app: &SurfaceState,
    created_block_id: &str,
    expected_kind: CardPageBlockKind,
    expected_text: &str,
) {
    let Some(CardPeekState::Loaded(page)) = app.page_documents.selected_page.as_ref() else {
        panic!("selected page should stay loaded");
    };
    assert_eq!(page.data.page.block_id, created_block_id);
    let appended_block = page
        .data
        .page
        .blocks
        .last()
        .expect("draft commit should append a block");
    let editable = appended_block
        .editable_content()
        .expect("draft commit should append editable content");
    assert_eq!(editable.kind, expected_kind);
    assert_eq!(editable.text, expected_text);
    assert!(!app.page_editor.input.composer.should_render());
    assert!(
        app.columns[0]
            .cards
            .last()
            .expect("created card should exist")
            .has_content
    );

    let pages = app
        .snapshot_pages
        .as_ref()
        .expect("fixture page source should stay active");
    assert_eq!(
        pages
            .get(created_block_id)
            .and_then(|page| page.blocks.last())
            .and_then(CardPageBlock::editable_content)
            .expect("fixture page should store committed block")
            .text,
        expected_text
    );
}
