use super::{CrdtItemId, CrdtOperationId};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub(in crate::live::board) struct SerializedTextItemAnnotations {
    #[serde(rename = "b", default, skip_serializing_if = "Option::is_none")]
    before: Option<Vec<SerializedAnnotationOperation>>,
    #[serde(rename = "a", default, skip_serializing_if = "Option::is_none")]
    after: Option<Vec<SerializedAnnotationOperation>>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "t")]
pub(in crate::live::board) enum SerializedAnnotationOperation {
    #[serde(rename = "a")]
    Add {
        #[serde(rename = "x")]
        text_instance_id: String,
        #[serde(rename = "l")]
        search_label: String,
        #[serde(rename = "i")]
        id: CrdtOperationId,
        #[serde(rename = "s")]
        start: SerializedAnnotationBoundary,
        #[serde(rename = "e")]
        end: SerializedAnnotationBoundary,
        #[serde(rename = "a")]
        annotation: Vec<Value>,
    },
    #[serde(rename = "r")]
    Remove {
        #[serde(rename = "x")]
        text_instance_id: String,
        #[serde(rename = "l")]
        search_label: String,
        #[serde(rename = "i")]
        id: CrdtOperationId,
        #[serde(rename = "s")]
        start: SerializedAnnotationBoundary,
        #[serde(rename = "e")]
        end: SerializedAnnotationBoundary,
        #[serde(rename = "k")]
        annotation_key: String,
    },
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(in crate::live::board) struct SerializedAnnotationBoundary {
    #[serde(rename = "i")]
    pub(in crate::live::board) id: CrdtItemId,
    #[serde(rename = "a")]
    pub(in crate::live::board) anchor: SerializedAnnotationAnchor,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
pub(in crate::live::board) enum SerializedAnnotationAnchor {
    #[serde(rename = "b")]
    Before,
    #[serde(rename = "a")]
    After,
}

impl SerializedTextItemAnnotations {
    pub(in crate::live::board) fn before(&self) -> Option<&[SerializedAnnotationOperation]> {
        self.before.as_deref()
    }

    pub(in crate::live::board) fn after(&self) -> Option<&[SerializedAnnotationOperation]> {
        self.after.as_deref()
    }
}
