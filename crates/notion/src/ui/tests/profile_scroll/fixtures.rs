use crate::model::{
    CardPage, CardPageBlock, CardPageBlockKind, CardPageEditableBlock, CardPageStructuralBlock,
    CardPageToDoState,
};

const LINEAR_FIXTURE_GROUP_COUNT: usize = 30;
const LINEAR_PAGE_ID: &str = "notion-scroll-profile-page";

/// A column-free document mixing the block kinds a real page carries, so the
/// linear render path pays for headings, wrapped paragraphs, nested lists,
/// to-dos, quotes, callouts, code, dividers, and toggles.
pub(super) fn linear_page() -> CardPage {
    let mut blocks = Vec::new();
    for group in 0..LINEAR_FIXTURE_GROUP_COUNT {
        push_linear_prose(&mut blocks, group);
        push_linear_lists(&mut blocks, group);
        push_linear_decorated(&mut blocks, group);
    }
    CardPage {
        block_id: LINEAR_PAGE_ID.to_string(),
        title: "Long linear profile document".to_string(),
        status: None,
        properties: Vec::new(),
        blocks,
        discussions: Vec::new(),
        comments_writable: false,
        format: Default::default(),
    }
}

fn push_linear_prose(blocks: &mut Vec<CardPageBlock>, group: usize) {
    push_linear_block(
        blocks,
        group,
        0,
        None,
        CardPageBlockKind::SubHeader,
        format!("Section {group}: rollout notes and follow-ups"),
    );
    push_linear_block(
        blocks,
        group,
        1,
        None,
        CardPageBlockKind::Text,
        format!(
            "Paragraph {group}: this deterministic paragraph is long enough to wrap across \
             several lines at the Notion document width, so text shaping, line wrapping, \
             and multi-line input layout all contribute to the frame cost during scrolling."
        ),
    );
    let bullet = push_linear_block(
        blocks,
        group,
        2,
        None,
        CardPageBlockKind::BulletedList,
        format!("Bullet {group}: the first item of a short nested list"),
    );
    push_linear_block(
        blocks,
        group,
        3,
        Some(&bullet),
        CardPageBlockKind::BulletedList,
        format!("Nested bullet {group}.1 with a little more detail"),
    );
    push_linear_block(
        blocks,
        group,
        4,
        Some(&bullet),
        CardPageBlockKind::BulletedList,
        format!("Nested bullet {group}.2 closing out the nested list"),
    );
}

fn push_linear_lists(blocks: &mut Vec<CardPageBlock>, group: usize) {
    push_linear_block(
        blocks,
        group,
        5,
        None,
        CardPageBlockKind::NumberedList,
        format!("Step {group}.1: collect the inputs"),
    );
    push_linear_block(
        blocks,
        group,
        6,
        None,
        CardPageBlockKind::NumberedList,
        format!("Step {group}.2: verify the outputs"),
    );
    let state = if group % 2 == 0 {
        CardPageToDoState::Checked
    } else {
        CardPageToDoState::Unchecked
    };
    blocks.push(CardPageBlock::from_editable(
        linear_block_id(group, 7),
        LINEAR_PAGE_ID,
        0,
        CardPageEditableBlock::to_do(
            format!("Task {group}: confirm the rollout window"),
            Vec::new(),
            state,
        ),
    ));
    push_linear_block(
        blocks,
        group,
        8,
        None,
        CardPageBlockKind::Quote,
        format!("Quote {group}: a short pull quote from the discussion"),
    );
}

fn push_linear_decorated(blocks: &mut Vec<CardPageBlock>, group: usize) {
    push_linear_block(
        blocks,
        group,
        9,
        None,
        CardPageBlockKind::Callout,
        format!("Callout {group}: remember to update the shared checklist"),
    );
    push_linear_block(
        blocks,
        group,
        10,
        None,
        CardPageBlockKind::Code,
        format!("fn rollout_{group}() -> bool {{\n    true\n}}"),
    );
    blocks.push(CardPageBlock::structural(
        linear_block_id(group, 11),
        LINEAR_PAGE_ID,
        0,
        CardPageStructuralBlock::Divider,
    ));
    push_linear_block(
        blocks,
        group,
        12,
        None,
        CardPageBlockKind::Text,
        format!("Short closing line for section {group}."),
    );
    push_linear_block(
        blocks,
        group,
        13,
        None,
        CardPageBlockKind::ToggleList,
        format!("Toggle {group}: collapsed details"),
    );
}

fn push_linear_block(
    blocks: &mut Vec<CardPageBlock>,
    group: usize,
    slot: usize,
    parent: Option<&str>,
    kind: CardPageBlockKind,
    text: String,
) -> String {
    let block_id = linear_block_id(group, slot);
    let (parent_id, depth) = match parent {
        Some(parent) => (parent.to_string(), 1),
        None => (LINEAR_PAGE_ID.to_string(), 0),
    };
    blocks.push(CardPageBlock::editable(
        block_id.clone(),
        parent_id,
        depth,
        kind,
        text,
    ));
    block_id
}

fn linear_block_id(group: usize, slot: usize) -> String {
    format!("notion-scroll-profile-{group:03}-{slot:02}")
}
