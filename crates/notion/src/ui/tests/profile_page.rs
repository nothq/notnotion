use crate::ui::tests::*;

#[gpui::test]
#[ignore]
fn profile_selected_page_scroll_render_loop() {
    profile_selected_page_scroll_loop(true);
}

#[gpui::test]
#[ignore]
fn profile_selected_page_scroll_update_loop() {
    profile_selected_page_scroll_loop(false);
}

fn profile_selected_page_scroll_loop(capture_frames: bool) {
    let _guard = acquire_headless_test_lock();
    let board = interaction_test_board();
    let pages = interaction_test_pages();
    let page = highest_content_test_page(&board, &pages);
    let label = if capture_frames {
        "page-scroll-render"
    } else {
        "page-scroll-update"
    };

    eprintln!(
        "profiling page '{}' with {} blocks and {} properties",
        page.title,
        page.blocks.len(),
        page.properties.len()
    );

    let mut cx = headless_test_context();
    let window = open_page_profile_window(&mut cx, board, page);
    let elapsed = run_page_profile_loop(&mut cx, window, capture_frames, label);

    eprintln!("{elapsed}");
}

fn open_page_profile_window(
    cx: &mut HeadlessAppContext,
    board: BoardSnapshot,
    page: CardPage,
) -> gpui::WindowHandle<SurfaceState> {
    cx.open_window(app_size(), |_, cx| {
        let page = page.clone();
        let board = board.clone();
        cx.new(move |_| {
            let mut app = SurfaceState::fixture_snapshot(
                board.clone(),
                HashMap::new(),
                AppearanceMode::Dark,
                Viewport::default(),
            );
            app.page_documents.selected_page =
                Some(CardPeekState::Loaded(LoadedCardPage::new(page)));
            app
        })
    })
    .expect("failed to open headless profile window")
}

fn run_page_profile_loop(
    cx: &mut HeadlessAppContext,
    window: gpui::WindowHandle<SurfaceState>,
    capture_frames: bool,
    label: &str,
) -> String {
    cx.run_until_parked();

    let root = window.root(cx).expect("failed to access profile root");
    let frame_count = 120usize;
    let start = Instant::now();
    let mut frame_durations = Vec::with_capacity(frame_count);
    for step in 0..frame_count {
        let direction = if step % 40 < 20 { 1.0 } else { -1.0 };
        let frame_start = Instant::now();
        root.update(cx, |app, cx| {
            if let Some(CardPeekState::Loaded(page)) = app.page_documents.selected_page.as_mut() {
                page.list_state.scroll_by(px(72.0 * direction));
                cx.notify();
            }
        });
        cx.run_until_parked();
        if capture_frames {
            let _ = cx
                .capture_screenshot(window.into())
                .expect("failed to capture profile screenshot");
        }
        frame_durations.push(frame_start.elapsed());
    }
    super::profile::summarize_profile_frames(label, &frame_durations, start.elapsed())
}

fn highest_content_test_page(board: &BoardSnapshot, pages: &HashMap<String, CardPage>) -> CardPage {
    board
        .columns
        .iter()
        .flat_map(|column| column.cards.iter())
        .map(|card| {
            let page = pages
                .get(&card.block_id)
                .expect("profile fixture missing card page")
                .clone();
            (page_profile_score(&page), page)
        })
        .max_by_key(|(score, _)| *score)
        .map(|(_, page)| page)
        .expect("board has no cards to profile")
}

fn page_profile_score(page: &CardPage) -> usize {
    page.blocks.len() * 1_000
        + page.properties.len() * 100
        + page
            .blocks
            .iter()
            .filter_map(CardPageBlock::editable_content)
            .map(|editable| editable.text.len())
            .sum::<usize>()
}
