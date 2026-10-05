use crate::live::board::Value;

#[derive(Default)]
pub(super) struct PropertyReferences<'a> {
    pub(super) block_ids: Vec<&'a str>,
    pub(super) user_ids: Vec<&'a str>,
}

#[derive(Clone, Copy)]
pub(super) enum PropertyReferenceScope {
    All,
    Title,
}

pub(super) fn property_references<'a>(
    value: &'a Value,
    block_id: &str,
    scope: PropertyReferenceScope,
) -> Result<PropertyReferences<'a>, String> {
    let mut references = PropertyReferences::default();
    let properties = value.get("properties");
    let source = match scope {
        PropertyReferenceScope::All => properties,
        PropertyReferenceScope::Title => properties
            .and_then(Value::as_object)
            .and_then(|properties| properties.get("title")),
    };
    if let Some(source) = source {
        collect_references(source, block_id, &mut references)?;
    }
    Ok(references)
}

fn collect_references<'a>(
    value: &'a Value,
    block_id: &str,
    references: &mut PropertyReferences<'a>,
) -> Result<(), String> {
    match value {
        Value::Array(values) => {
            if let Some(kind @ ("p" | "u")) = values.first().and_then(Value::as_str) {
                if let Some(id) = values.get(1).and_then(Value::as_str) {
                    if id.trim().is_empty() {
                        return Err(format!(
                            "Notion block {block_id} contains an empty {kind} reference"
                        ));
                    }
                    if kind == "p" {
                        references.block_ids.push(id);
                    } else {
                        references.user_ids.push(id);
                    }
                    return Ok(());
                }
            }
            for value in values {
                collect_references(value, block_id, references)?;
            }
        }
        Value::Object(values) => {
            for value in values.values() {
                collect_references(value, block_id, references)?;
            }
        }
        Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) => {}
    }
    Ok(())
}
