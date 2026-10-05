use std::sync::Arc;

use gpui::{Bounds, Pixels};

use crate::model::{
    CardPage, CardPageColumnEffectiveShare, PageColumnPair, ResizePageColumnsRequest,
};
use crate::ui::LoadedCardPageData;

use super::{PageFlowLayoutAuthority, PageFlowSurfaceKey};

#[derive(Clone)]
pub(crate) struct PageColumnResizePreview {
    pub(crate) left_share: CardPageColumnEffectiveShare,
    pub(crate) right_share: CardPageColumnEffectiveShare,
}

pub(crate) struct PageColumnResizeSession {
    data: Arc<LoadedCardPageData>,
    surface: PageFlowSurfaceKey,
    authority: PageFlowLayoutAuthority,
    pair: PageColumnPair,
    source_left_share: CardPageColumnEffectiveShare,
    geometry: PageColumnResizeGeometry,
    preview: Option<PageColumnResizePreview>,
}

/// The left column's share and the pointer geometry when a resize began.
#[derive(Clone, Copy)]
pub(crate) struct PageColumnResizeOrigin {
    pub(crate) source_left_share: CardPageColumnEffectiveShare,
    pub(crate) geometry: PageColumnResizeGeometry,
}

/// The page, flow surface, and column pair a resize drag started on.
pub(crate) struct PageColumnResizeSource<'a> {
    pub(crate) data: &'a Arc<LoadedCardPageData>,
    pub(crate) surface: PageFlowSurfaceKey,
    pub(crate) column_list_id: &'a str,
    pub(crate) left_column_id: &'a str,
    pub(crate) right_column_id: &'a str,
}

#[derive(Clone, Copy)]
pub(crate) struct PageColumnResizeGeometry {
    start_pointer_x: f32,
    start_left_width: f32,
    pair_width: f32,
    pair_share_total: f64,
}

impl PageColumnResizeGeometry {
    pub(crate) fn new(
        start_pointer_x: f32,
        left: Bounds<Pixels>,
        right: Bounds<Pixels>,
        pair_share_total: f64,
    ) -> Option<Self> {
        let start_left_width = left.size.width.as_f32();
        let right_width = right.size.width.as_f32();
        let pair_width = start_left_width + right_width;
        (start_pointer_x.is_finite()
            && start_left_width.is_finite()
            && start_left_width > 0.0
            && right_width.is_finite()
            && right_width > 0.0
            && pair_width.is_finite()
            && pair_share_total.is_finite()
            && pair_share_total > 0.0)
            .then_some(Self {
                start_pointer_x,
                start_left_width,
                pair_width,
                pair_share_total,
            })
    }

    fn shares(
        self,
        pointer_x: f32,
    ) -> Option<(CardPageColumnEffectiveShare, CardPageColumnEffectiveShare)> {
        if !pointer_x.is_finite() {
            return None;
        }
        let candidate_left = self.start_left_width + pointer_x - self.start_pointer_x;
        let left_fraction = self.pair_share_total * f64::from(candidate_left / self.pair_width);
        let left = CardPageColumnEffectiveShare::from_fraction(left_fraction).ok()?;
        let right =
            CardPageColumnEffectiveShare::from_fraction(self.pair_share_total - left.fraction())
                .ok()?;
        Some((left, right))
    }

    fn is_origin(self, pointer_x: f32) -> bool {
        pointer_x.is_finite() && pointer_x.to_bits() == self.start_pointer_x.to_bits()
    }
}

impl PageColumnResizeSession {
    pub(crate) fn new(
        data: Arc<LoadedCardPageData>,
        surface: PageFlowSurfaceKey,
        authority: PageFlowLayoutAuthority,
        pair: PageColumnPair,
        origin: PageColumnResizeOrigin,
    ) -> Self {
        let PageColumnResizeOrigin {
            source_left_share,
            geometry,
        } = origin;
        Self {
            data,
            surface,
            authority,
            pair,
            source_left_share,
            geometry,
            preview: None,
        }
    }

    pub(crate) fn update(&mut self, pointer_x: f32) -> bool {
        if self.geometry.is_origin(pointer_x) {
            return self.preview.take().is_some();
        }
        let Some((left_share, right_share)) = self.geometry.shares(pointer_x) else {
            return false;
        };
        if left_share == self.source_left_share {
            return self.preview.take().is_some();
        }
        if self.preview.as_ref().is_some_and(|preview| {
            preview.left_share == left_share && preview.right_share == right_share
        }) {
            return false;
        }
        self.preview = Some(PageColumnResizePreview {
            left_share,
            right_share,
        });
        true
    }

    pub(crate) fn preview_for(
        &self,
        data: &Arc<LoadedCardPageData>,
        surface: PageFlowSurfaceKey,
        column_list_id: &str,
    ) -> Option<&PageColumnResizePreview> {
        (Arc::ptr_eq(&self.data, data)
            && self.surface == surface
            && self.pair.column_list_block_id() == column_list_id)
            .then_some(self.preview.as_ref())
            .flatten()
    }

    pub(crate) fn effective_share_for(
        &self,
        data: &Arc<LoadedCardPageData>,
        surface: PageFlowSurfaceKey,
        column_list_id: &str,
        column_id: &str,
    ) -> Option<CardPageColumnEffectiveShare> {
        let preview = self.preview_for(data, surface, column_list_id)?;
        if self.pair.left_column_block_id() == column_id {
            Some(preview.left_share)
        } else if self.pair.right_column_block_id() == column_id {
            Some(preview.right_share)
        } else {
            None
        }
    }

    pub(crate) fn matches_source(&self, source: PageColumnResizeSource<'_>) -> bool {
        let PageColumnResizeSource {
            data,
            surface,
            column_list_id,
            left_column_id,
            right_column_id,
        } = source;
        Arc::ptr_eq(&self.data, data)
            && self.surface == surface
            && self.pair.column_list_block_id() == column_list_id
            && self.pair.left_column_block_id() == left_column_id
            && self.pair.right_column_block_id() == right_column_id
    }

    pub(crate) fn authority(&self) -> &PageFlowLayoutAuthority {
        &self.authority
    }

    pub(crate) fn pair(&self) -> &PageColumnPair {
        &self.pair
    }

    pub(crate) fn preview(&self) -> Option<&PageColumnResizePreview> {
        self.preview.as_ref()
    }

    pub(crate) fn apply_final_request(
        &self,
        current: &CardPage,
    ) -> Result<(CardPage, ResizePageColumnsRequest), String> {
        let request = self.final_request()?;
        request.validate_source_on(current)?;
        let mut page = current.clone();
        request.apply_to_page(&mut page)?;
        Ok((page, request))
    }

    pub(crate) fn final_request(&self) -> Result<ResizePageColumnsRequest, String> {
        let preview = self
            .preview
            .as_ref()
            .ok_or_else(|| "a Column resize must retain a valid preview".to_string())?;
        ResizePageColumnsRequest::new(
            &self.data.page,
            self.pair.column_list_block_id(),
            self.pair.left_column_block_id(),
            self.pair.right_column_block_id(),
            preview.left_share,
        )
    }
}
