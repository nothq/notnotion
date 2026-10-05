use crate::ui::board_workspace::dialogs::filter::{prelude::*, types::*};

pub(super) fn database_filter_rule_count(group: &DatabaseFilterGroup) -> usize {
    group
        .filters()
        .iter()
        .map(|node| match node {
            DatabaseFilterNode::Property(_) => 1,
            DatabaseFilterNode::Group(group) => database_filter_rule_count(group),
        })
        .sum()
}

pub(super) fn database_advanced_filter_rows(
    group: &DatabaseFilterGroup,
) -> Vec<DatabaseAdvancedFilterRow> {
    fn collect(
        group: &DatabaseFilterGroup,
        parent_path: &[usize],
        rows: &mut Vec<DatabaseAdvancedFilterRow>,
    ) {
        for (index, node) in group.filters().iter().enumerate() {
            let mut path = parent_path.to_vec();
            path.push(index);
            rows.push(DatabaseAdvancedFilterRow {
                path: path.clone(),
                node: node.clone(),
            });
            if let DatabaseFilterNode::Group(group) = node {
                collect(group, &path, rows);
            }
        }
    }

    let mut rows = Vec::new();
    collect(group, &[], &mut rows);
    rows
}

pub(super) fn database_advanced_filter_group<'a>(
    group: &'a DatabaseFilterGroup,
    path: &[usize],
) -> Option<&'a DatabaseFilterGroup> {
    if path.is_empty() {
        return Some(group);
    }
    let DatabaseFilterNode::Group(group) = database_advanced_filter_node(group, path)? else {
        return None;
    };
    Some(group)
}

pub(super) fn database_advanced_filter_appending_node(
    group: &DatabaseFilterGroup,
    group_path: &[usize],
    node: DatabaseFilterNode,
) -> Option<DatabaseFilterGroup> {
    if group_path.is_empty() {
        let mut filters = group.filters().to_vec();
        filters.push(node);
        return Some(DatabaseFilterGroup::new(group.operator(), filters));
    }
    let child = database_advanced_filter_group(group, group_path)?;
    let mut child_filters = child.filters().to_vec();
    child_filters.push(node);
    database_advanced_filter_replacing_node(
        group,
        group_path,
        DatabaseFilterNode::Group(DatabaseFilterGroup::new(child.operator(), child_filters)),
    )
}

pub(super) fn database_advanced_filter_node<'a>(
    group: &'a DatabaseFilterGroup,
    path: &[usize],
) -> Option<&'a DatabaseFilterNode> {
    let (index, remainder) = path.split_first()?;
    let node = group.filters().get(*index)?;
    if remainder.is_empty() {
        return Some(node);
    }
    let DatabaseFilterNode::Group(group) = node else {
        return None;
    };
    database_advanced_filter_node(group, remainder)
}

pub(super) fn database_advanced_filter_replacing_node(
    group: &DatabaseFilterGroup,
    path: &[usize],
    replacement: DatabaseFilterNode,
) -> Option<DatabaseFilterGroup> {
    let (index, remainder) = path.split_first()?;
    let mut filters = group.filters().to_vec();
    if remainder.is_empty() {
        *filters.get_mut(*index)? = replacement;
    } else {
        let DatabaseFilterNode::Group(child) = filters.get(*index)? else {
            return None;
        };
        let child = database_advanced_filter_replacing_node(child, remainder, replacement)?;
        filters[*index] = DatabaseFilterNode::Group(child);
    }
    Some(DatabaseFilterGroup::new(group.operator(), filters))
}

pub(super) fn database_advanced_filter_removing_node(
    group: &DatabaseFilterGroup,
    path: &[usize],
) -> Option<DatabaseFilterGroup> {
    let (index, remainder) = path.split_first()?;
    let mut filters = group.filters().to_vec();
    if remainder.is_empty() {
        filters.remove(*index);
    } else {
        let DatabaseFilterNode::Group(child) = filters.get(*index)? else {
            return None;
        };
        let child = database_advanced_filter_removing_node(child, remainder)?;
        if child.filters().is_empty() {
            filters.remove(*index);
        } else {
            filters[*index] = DatabaseFilterNode::Group(child);
        }
    }
    Some(DatabaseFilterGroup::new(group.operator(), filters))
}

pub(super) fn database_advanced_filter_replacing_group_operator(
    group: &DatabaseFilterGroup,
    path: &[usize],
) -> Option<DatabaseFilterGroup> {
    let DatabaseFilterNode::Group(child) = database_advanced_filter_node(group, path)? else {
        return None;
    };
    let replacement = DatabaseFilterNode::Group(DatabaseFilterGroup::new(
        match child.operator() {
            DatabaseFilterGroupOperator::And => DatabaseFilterGroupOperator::Or,
            DatabaseFilterGroupOperator::Or => DatabaseFilterGroupOperator::And,
        },
        child.filters().to_vec(),
    ));
    database_advanced_filter_replacing_node(group, path, replacement)
}

pub(super) const fn database_filter_group_operator_label(
    operator: DatabaseFilterGroupOperator,
) -> &'static str {
    match operator {
        DatabaseFilterGroupOperator::And => "And",
        DatabaseFilterGroupOperator::Or => "Or",
    }
}

pub(super) fn advanced_filter_path_id(path: &[usize]) -> String {
    path.iter()
        .map(usize::to_string)
        .collect::<Vec<_>>()
        .join("-")
}
