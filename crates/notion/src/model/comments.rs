mod draft;
mod identities;
mod mutation;
mod projection;

pub use draft::{NotionCommentDraft, NotionCommentDraftSegment, NotionCommentWorkspaceUsers};
pub use identities::{NotionCommentId, NotionCommentTargetId, NotionDiscussionId};
pub(crate) use mutation::NotionCommentMutation;
pub use mutation::NotionCommentMutationRequest;
pub use projection::{
    CardPageComment, CardPageCommentAuthor, CardPageCommentRichSegment, CardPageDiscussion,
    CardPageDiscussionAccess,
};
