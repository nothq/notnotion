use crate::model::BoardItemProperty;
use crate::ui::tests::*;

pub(crate) fn notion_page_shell_test_board() -> BoardSnapshot {
    let mut board = interaction_test_board();
    board.page_title = "Administration".to_string();
    board.database_title = "Accounts".to_string();
    board.page_shell = Some(crate::ui::PageShellSnapshot {
        workspace_name: "Acme".to_string(),
        page_icon: named_icon("page"),
        page_icon_is_explicit: true,
        builtin_links: vec![
            page_shell_link("Search", "search", None),
            page_shell_link("Home", "home", Some("acme://notion/home")),
        ],
        sidebar_sections: vec![crate::ui::PageShellSidebarSection {
            identity: None,
            title: "General".to_string(),
            icon: named_icon("home"),
            items: notion_page_shell_sidebar_items(),
        }],
        links: vec![page_shell_link(
            "Finance",
            "page",
            Some("https://www.notion.so/acme/finance"),
        )],
    });
    board
}

pub(crate) fn notion_home_test_board() -> BoardSnapshot {
    let mut board = notion_page_shell_test_board();
    board.page_title = "Home".to_string();
    board.database_title = "Home".to_string();
    clear_board_views(&mut board);
    board.page_content = Some(CardPage {
        block_id: "acme-builtin-home".to_string(),
        title: "Home".to_string(),
        status: None,
        properties: vec![
            CardPageProperty {
                property_id: "workspace".to_string(),
                label: "Workspace".to_string(),
                property_type: "text".to_string(),
                value: "Acme".to_string(),
                status_options: Vec::new(),
            },
            CardPageProperty {
                property_id: "shared_pages".to_string(),
                label: "Shared pages".to_string(),
                property_type: "number".to_string(),
                value: "3".to_string(),
                status_options: Vec::new(),
            },
        ],
        discussions: Vec::new(),
        comments_writable: false,
        format: Default::default(),
        blocks: vec![
            CardPageBlock::editable(
                "home-intro",
                "acme-builtin-home",
                0,
                CardPageBlockKind::Text,
                "Jump into the shared pages Acme can reach from Notion Desktop.",
            ),
            CardPageBlock::editable(
                "home-workspace-pages",
                "acme-builtin-home",
                0,
                CardPageBlockKind::Heading3,
                "Workspace pages",
            ),
        ],
    });
    board
}

pub(crate) fn table_test_board() -> BoardSnapshot {
    let mut board = interaction_test_board();
    board.page_title = "Our Documents".to_string();
    board.database_title = "Our Documents".to_string();
    board.view_tabs = vec![
        ViewTab {
            provider_view_id: "test-table-view"
                .parse()
                .expect("test table view ID must be valid"),
            label: "Table".to_string(),
            kind: ViewTabKind::Table,
            active: true,
            filters: None,
        },
        ViewTab {
            provider_view_id: "test-board-view"
                .parse()
                .expect("test board view ID must be valid"),
            label: "Board".to_string(),
            kind: ViewTabKind::Board,
            active: false,
            filters: None,
        },
    ];
    board.items = vec![table_test_item()];
    board.table_view_columns = table_test_columns();
    board
}

pub(crate) fn interaction_test_pages() -> HashMap<String, CardPage> {
    HashMap::from([(
        "card-1".to_string(),
        CardPage {
            block_id: "card-1".to_string(),
            title: "Card One".to_string(),
            status: Some("Todo".to_string()),
            properties: Vec::new(),
            discussions: Vec::new(),
            comments_writable: false,
            format: Default::default(),
            blocks: vec![CardPageBlock::editable(
                "card-1-text",
                "card-1",
                0,
                CardPageBlockKind::Text,
                "Hello",
            )],
        },
    )])
}

pub(crate) fn clear_board_views(board: &mut BoardSnapshot) {
    board.columns.clear();
    board.view_tabs.clear();
    board.items.clear();
    board.table_view_columns.clear();
    board.timeline_view = None;
    board.calendar_view = None;
}

fn notion_page_shell_sidebar_items() -> Vec<crate::ui::PageShellSidebarItem> {
    vec![
        sidebar_item("Administration", true, Some(TEST_BOARD_URL)),
        sidebar_item(
            "Our Documents",
            false,
            Some("https://www.notion.so/acme/our-documents"),
        ),
        sidebar_item("Finance", false, Some("https://www.notion.so/acme/finance")),
    ]
}

fn page_shell_link(
    title: &str,
    icon_name: &str,
    target_board_url: Option<&str>,
) -> crate::ui::PageShellLink {
    crate::ui::PageShellLink {
        identity: None,
        title: title.to_string(),
        icon: named_icon(icon_name),
        target_board_url: target_board_url.map(str::to_string),
    }
}

fn sidebar_item(
    title: &str,
    active: bool,
    target_board_url: Option<&str>,
) -> crate::ui::PageShellSidebarItem {
    crate::ui::PageShellSidebarItem {
        identity: None,
        title: title.to_string(),
        icon: named_icon("page"),
        active,
        target_board_url: target_board_url.map(str::to_string),
        children: Vec::new(),
        child_block_ids: Vec::new(),
        unresolved_child_block_ids: Vec::new(),
        sidebar_children_resolved: true,
    }
}

fn named_icon(name: &str) -> crate::ui::PageShellIcon {
    crate::ui::PageShellIcon::named(name)
}

fn table_test_item() -> BoardItem {
    BoardItem {
        block_id: "card-1".to_string(),
        title: "Card One".to_string(),
        status: Some("Todo".to_string()),
        icon: None,
        properties: vec![
            BoardItemProperty {
                property_id: "tags".to_string(),
                label: "Tags".to_string(),
                property_type: "multi_select".to_string(),
                value: "Product Update Memo".to_string(),
                date: None,
            },
            BoardItemProperty {
                property_id: "created_by".to_string(),
                label: "Created by".to_string(),
                property_type: "created_by".to_string(),
                value: "Alan Turing".to_string(),
                date: None,
            },
            BoardItemProperty {
                property_id: "last_edited_time".to_string(),
                label: "Last edited time".to_string(),
                property_type: "last_edited_time".to_string(),
                value: "April 9, 2026 6:02 PM".to_string(),
                date: None,
            },
        ],
    }
}

fn table_test_columns() -> Vec<TableViewColumn> {
    vec![
        TableViewColumn {
            property_id: "title".to_string(),
            label: "Title".to_string(),
            width: 240.0,
            wrap: false,
        },
        TableViewColumn {
            property_id: "tags".to_string(),
            label: "Tags".to_string(),
            width: 148.0,
            wrap: false,
        },
        TableViewColumn {
            property_id: "created_by".to_string(),
            label: "Created by".to_string(),
            width: 148.0,
            wrap: false,
        },
        TableViewColumn {
            property_id: "last_edited_time".to_string(),
            label: "Last edited time".to_string(),
            width: 180.0,
            wrap: false,
        },
    ]
}
