use crate::ui::board_workspace::PageDocumentAction;
use crate::ui::tests::cases::*;

#[gpui::test]
fn activate_view_tab_switches_active_view() {
    with_interaction_app(
        interaction_test_board(),
        interaction_test_pages(),
        |app: &mut SurfaceState, cx: &mut gpui::Context<SurfaceState>| {
            app.activate_view_tab(1, cx);
            assert_eq!(app.board.active_view_kind(), ViewTabKind::Timeline);
        },
    );
}

#[gpui::test]
fn card_click_opens_page_through_shared_drag_path() {
    with_interaction_app(
        interaction_test_board(),
        interaction_test_pages(),
        |app: &mut SurfaceState, cx: &mut gpui::Context<SurfaceState>| {
            let position = point(
                px(app.board_drag_layout().card_left(0) + 12.0),
                px(app.board_drag_layout().card_top(0, 0) + 12.0),
            );
            app.handle_card_mouse_down(CardLocation::new(0, 0), position, cx);
            app.finish_drag(cx);
            assert!(matches!(
                app.page_documents.selected_page.as_ref(),
                Some(CardPeekState::Loaded(page)) if page.data.page.title == "Card One"
            ));
        },
    );
}

#[gpui::test]
fn new_page_row_click_creates_blank_page_in_clicked_column() {
    with_interaction_app(
        interaction_test_board(),
        interaction_test_pages(),
        |app: &mut SurfaceState, cx: &mut gpui::Context<SurfaceState>| {
            app.dispatch_page_document_action(PageDocumentAction::CreateInColumn(0), cx);
            assert_eq!(app.columns[0].cards.len(), 2);
            let created_card = app.columns[0]
                .cards
                .last()
                .expect("missing created card")
                .clone();
            assert_eq!(created_card.title, "");
            assert!(!created_card.has_content);
            assert!(matches!(
                app.page_documents.selected_page.as_ref(),
                Some(CardPeekState::Loaded(page))
                    if page.data.page.block_id == created_card.block_id
                        && page.data.page.title.is_empty()
                        && page.data.page.status.as_deref() == Some("Todo")
            ));
        },
    );
}

#[gpui::test]
fn primary_new_button_creates_blank_page_in_first_column() {
    with_interaction_app(
        interaction_test_board(),
        interaction_test_pages(),
        |app: &mut SurfaceState, cx: &mut gpui::Context<SurfaceState>| {
            app.dispatch_page_document_action(PageDocumentAction::CreatePrimary, cx);
            assert_eq!(app.columns[0].cards.len(), 2);
            let created_card = app.columns[0]
                .cards
                .last()
                .expect("missing created card")
                .clone();
            assert!(matches!(
                app.page_documents.selected_page.as_ref(),
                Some(CardPeekState::Loaded(page))
                    if page.data.page.block_id == created_card.block_id
                        && page.data.page.status.as_deref() == Some("Todo")
            ));
        },
    );
}

#[gpui::test]
fn table_row_click_opens_page() {
    with_interaction_app(
        table_test_board(),
        interaction_test_pages(),
        |app: &mut SurfaceState, cx: &mut gpui::Context<SurfaceState>| {
            let item = app.board.items.first().expect("missing table item").clone();
            app.dispatch_page_document_action(
                PageDocumentAction::OpenBoardItem((item).clone()),
                cx,
            );
            assert!(matches!(
                app.page_documents.selected_page.as_ref(),
                Some(CardPeekState::Loaded(page)) if page.data.page.title == "Card One"
            ));
        },
    );
}

#[gpui::test]
fn enter_on_blank_page_starts_text_composer() {
    with_interaction_app(
        interaction_test_board(),
        interaction_test_pages(),
        |app: &mut SurfaceState, cx: &mut gpui::Context<SurfaceState>| {
            app.dispatch_page_document_action(PageDocumentAction::CreateInColumn(0), cx);
            assert!(app.handle_selected_page_key_down(&named_key_down_event("enter"), cx));
            assert!(app.page_editor.input.composer.active);
            assert!(!app.page_editor.input.composer.slash_command_open);
            assert_eq!(
                app.page_editor.input.composer.block_kind,
                CardPageBlockKind::Text
            );
            assert!(app.page_editor.input.composer.text.is_empty());
        },
    );
}

#[gpui::test]
fn slash_command_selection_and_enter_commit_append_local_page_block() {
    with_interaction_window(
        interaction_test_board(),
        interaction_test_pages(),
        |window, cx| {
            let created_page_id = create_interaction_page(window, cx);
            activate_interaction_page_composer(window, cx);
            dispatch_interaction_keystroke(window, "/", cx);
            draw_interaction_window(window, cx);
            assert_native_slash_menu(window, "", 0, "/", cx);

            dispatch_interaction_keystroke(window, "2", cx);
            assert_native_slash_menu(window, "2", 0, "/2", cx);

            dispatch_interaction_keystroke(window, "enter", cx);
            draw_interaction_window(window, cx);
            assert_native_slash_block(window, CardPageBlockKind::SubSubHeader, "", cx);

            type_interaction_text(window, "Next", cx);
            assert_persisted_interaction_block(
                window,
                &created_page_id,
                CardPageBlockKind::SubSubHeader,
                "Next",
                cx,
            );
        },
    );
}

#[gpui::test]
fn slash_command_arrow_navigation_selects_active_row_on_enter() {
    with_interaction_window(
        interaction_test_board(),
        interaction_test_pages(),
        |window, cx| {
            let created_page_id = create_interaction_page(window, cx);
            activate_interaction_page_composer(window, cx);
            dispatch_interaction_keystroke(window, "/", cx);
            draw_interaction_window(window, cx);
            assert_native_slash_menu(window, "", 0, "/", cx);

            dispatch_interaction_keystroke(window, "down", cx);
            assert_native_slash_selection(window, 1, cx);
            dispatch_interaction_keystroke(window, "down", cx);
            assert_native_slash_selection(window, 2, cx);

            dispatch_interaction_keystroke(window, "enter", cx);
            draw_interaction_window(window, cx);
            assert_native_slash_block(window, CardPageBlockKind::SubSubHeader, "", cx);
            assert_persisted_interaction_block(
                window,
                &created_page_id,
                CardPageBlockKind::SubSubHeader,
                "",
                cx,
            );
        },
    );
}

#[gpui::test]
fn open_toolbar_dialog_keeps_selected_page_open() {
    with_interaction_app(
        interaction_test_board(),
        interaction_test_pages(),
        |app: &mut SurfaceState, cx: &mut gpui::Context<SurfaceState>| {
            let card = app.columns[0].cards[0].clone();
            app.dispatch_page_document_action(PageDocumentAction::OpenCard(card), cx);
            app.open_toolbar_dialog(ToolbarDialogKind::Filter, cx);
            assert!(app.notion_chrome.toolbar_dialog == Some(ToolbarDialogKind::Filter));
            assert!(matches!(
                app.page_documents.selected_page.as_ref(),
                Some(CardPeekState::Loaded(page)) if page.data.page.title == "Card One"
            ));
        },
    );
}

#[gpui::test]
fn toggle_toolbar_search_keeps_selected_page_open() {
    with_interaction_app(
        interaction_test_board(),
        interaction_test_pages(),
        |app: &mut SurfaceState, cx: &mut gpui::Context<SurfaceState>| {
            let card = app.columns[0].cards[0].clone();
            app.dispatch_page_document_action(PageDocumentAction::OpenCard(card), cx);
            app.toggle_toolbar_search(cx);
            assert!(app.database_search.open);
            assert!(matches!(
                app.page_documents.selected_page.as_ref(),
                Some(CardPeekState::Loaded(page)) if page.data.page.title == "Card One"
            ));
        },
    );
}

#[gpui::test]
fn selected_page_back_restores_previous_page() {
    let mut board = interaction_test_board();
    board.columns[0].cards.push(CardSummary {
        block_id: "card-2".to_string(),
        title: "Card Two".to_string(),
        height: 72.0,
        has_content: true,
        icon: None,
    });
    let mut pages = interaction_test_pages();
    pages.insert(
        "card-2".to_string(),
        CardPage {
            block_id: "card-2".to_string(),
            title: "Card Two".to_string(),
            status: Some("Todo".to_string()),
            properties: Vec::new(),
            discussions: Vec::new(),
            comments_writable: false,
            format: Default::default(),
            blocks: vec![CardPageBlock::editable(
                "card-2-text",
                "card-2",
                0,
                CardPageBlockKind::Text,
                "Second page",
            )],
        },
    );

    with_interaction_app(
        board,
        pages,
        |app: &mut SurfaceState, cx: &mut gpui::Context<SurfaceState>| {
            let first_card = app.columns[0].cards[0].clone();
            let second_card = app.columns[0].cards[1].clone();
            app.dispatch_page_document_action(PageDocumentAction::OpenCard(first_card), cx);
            app.dispatch_page_document_action(PageDocumentAction::OpenCard(second_card), cx);
            assert!(matches!(
                app.page_documents.selected_page.as_ref(),
                Some(CardPeekState::Loaded(page)) if page.data.page.title == "Card Two"
            ));
            app.dispatch_page_document_action(PageDocumentAction::NavigateBack, cx);
            assert!(matches!(
                app.page_documents.selected_page.as_ref(),
                Some(CardPeekState::Loaded(page)) if page.data.page.title == "Card One"
            ));
            assert!(app.page_documents.selected_page_history.is_empty());
        },
    );
}
use crate::model::CardSummary;
