use crate::ui::tests::cases::*;

#[derive(Clone)]
struct MockNotionWorkspaceApi {
    workspaces: HashMap<String, BoardSnapshot>,
    requested_board_urls: Arc<Mutex<Vec<String>>>,
}

impl crate::model::NotionWorkspaceApi for MockNotionWorkspaceApi {
    fn load_card_page(&self, block_id: &str) -> crate::model::NotionWorkspaceResult<CardPage> {
        Err(format!("unexpected card page request {block_id}").into())
    }

    fn set_favorited(&self, _is_favorited: bool) -> crate::model::NotionWorkspaceResult<()> {
        Err("favorites are not supported in notion test api"
            .to_string()
            .into())
    }

    fn create_page_in_column(
        &self,
        _target_column_title: &str,
    ) -> crate::model::NotionWorkspaceResult<String> {
        Err("page creation is not supported in notion test api"
            .to_string()
            .into())
    }

    fn move_card(&self, _request: MoveCardRequest) -> crate::model::NotionWorkspaceResult<()> {
        Err("card drag is not supported in notion test api"
            .to_string()
            .into())
    }

    fn load_notion_workspace(
        &self,
        board_url: &str,
    ) -> crate::model::NotionWorkspaceResult<crate::model::NotionWorkspaceLoad> {
        self.requested_board_urls
            .lock()
            .expect("notion request log mutex poisoned")
            .push(board_url.to_string());
        let workspace = self.workspaces.get(board_url).cloned().ok_or_else(|| {
            crate::model::NotionWorkspaceOperationFailure::new(format!(
                "missing test notion workspace {board_url}"
            ))
        })?;
        Ok(crate::model::NotionWorkspaceLoad {
            workspace,
            workspace_api: Arc::new(self.clone()),
            code_settings: crate::model::CardPageCodeSettingsCapability::memory(),
        })
    }
}

#[gpui::test]
fn notion_sidebar_navigation_replaces_workspace_and_preserves_page_shell_state() {
    let target_url = "https://www.notion.so/acme/our-documents".to_string();
    let requested_board_urls = Arc::new(Mutex::new(Vec::new()));
    let api = Arc::new(MockNotionWorkspaceApi {
        workspaces: HashMap::from([(target_url.clone(), target_sidebar_target_board())]),
        requested_board_urls: requested_board_urls.clone(),
    });

    with_notion_workspace_app(
        api,
        |app: &mut NotionTestApp, cx: &mut gpui::Context<NotionTestApp>| {
            app.root.set_sidebar_visible(false, cx);
            app.root.collapse_page_section("General".to_string(), cx);
            app.root
                .open_notion_workspace(
                    crate::model::NotionLaunchRoute::board(
                        target_url
                            .parse()
                            .expect("test Notion board URL should be valid"),
                    ),
                    cx,
                )
                .expect("ready Notion surface should accept navigation");
        },
        |app: &mut NotionTestApp, cx| {
            let board = app.root.board(cx);
            assert_eq!(board.page_title, "Our Documents");
            assert_eq!(board.database_title, "Our Documents");
            assert!(board.page_shell.is_some());
            assert!(!app.root.sidebar_visible(cx));
            assert!(app.root.page_section_collapsed("General", cx));
            assert!(app.root.pending_navigation(cx).is_none());
            assert!(app.root.navigation_error(cx).is_none());
        },
    );

    assert_eq!(
        requested_board_urls
            .lock()
            .expect("notion request log mutex poisoned")
            .as_slice(),
        &[target_url]
    );
}

#[gpui::test]
fn notion_builtin_home_navigation_replaces_workspace() {
    let target_url = "acme://notion/home".to_string();
    let requested_board_urls = Arc::new(Mutex::new(Vec::new()));
    let api = Arc::new(MockNotionWorkspaceApi {
        workspaces: HashMap::from([(target_url.clone(), notion_home_test_board())]),
        requested_board_urls: requested_board_urls.clone(),
    });

    with_notion_workspace_app(
        api,
        |app: &mut NotionTestApp, cx: &mut gpui::Context<NotionTestApp>| {
            app.root
                .open_notion_workspace(
                    crate::model::NotionLaunchRoute::board(
                        target_url
                            .parse()
                            .expect("test Notion home URL should be valid"),
                    ),
                    cx,
                )
                .expect("ready Notion surface should accept navigation")
        },
        |app: &mut NotionTestApp, cx| {
            let board = app.root.board(cx);
            assert_eq!(board.page_title, "Home");
            assert_eq!(board.database_title, "Home");
            assert!(app.root.standalone_page_content_loaded(cx));
            assert!(board.page_shell.is_some());
            assert!(app.root.pending_navigation(cx).is_none());
            assert!(app.root.navigation_error(cx).is_none());
        },
    );

    assert_eq!(
        requested_board_urls
            .lock()
            .expect("notion request log mutex poisoned")
            .as_slice(),
        &[target_url]
    );
}

fn with_notion_workspace_app(
    api: Arc<MockNotionWorkspaceApi>,
    navigate: impl FnOnce(&mut NotionTestApp, &mut gpui::Context<NotionTestApp>),
    assert_app: impl FnOnce(&mut NotionTestApp, &mut gpui::Context<NotionTestApp>),
) {
    let _guard = acquire_headless_test_lock();
    let mut cx = headless_test_context();
    let window = cx
        .open_window(app_size(), |_, cx| {
            let api = api.clone();
            let board = notion_page_shell_test_board();
            cx.new(move |_| NotionTestApp::from_workspace(board, api, Viewport::default()))
        })
        .expect("failed to open notion navigation test window");
    cx.run_until_parked();

    let root = window
        .root(&mut cx)
        .expect("failed to access notion navigation test root");
    root.update(&mut cx, |app, cx| navigate(app, cx));
    cx.run_until_parked();
    root.update(&mut cx, |app, cx| assert_app(app, cx));
}

fn target_sidebar_target_board() -> BoardSnapshot {
    let mut board = notion_page_shell_test_board();
    board.page_title = "Our Documents".to_string();
    board.database_title = "Our Documents".to_string();
    if let Some(page_shell) = board.page_shell.as_mut() {
        for section in &mut page_shell.sidebar_sections {
            for item in &mut section.items {
                item.active = item.title == "Our Documents";
            }
        }
    }
    board
}
