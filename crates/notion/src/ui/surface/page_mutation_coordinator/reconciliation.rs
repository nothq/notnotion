use super::{PageMutationCoordinator, PageMutationLane, PageMutationRunToken};
use crate::model::CardPage;
use crate::ui::Arc;

impl PageMutationCoordinator {
    pub(crate) fn capture_optimistic_page(&mut self, page: CardPage) {
        let Some(lane) = self.lanes.get_mut(&page.block_id) else {
            return;
        };
        if !lane.is_idle() {
            lane.optimistic_page = Some(Arc::new(page));
        }
    }

    pub(crate) fn retain_offscreen_result(
        &mut self,
        page_id: &str,
        token: PageMutationRunToken,
        authoritative_page: Arc<CardPage>,
        optimistic_page: Option<Arc<CardPage>>,
    ) {
        let lane = self
            .lanes
            .get_mut(page_id)
            .expect("committed page mutation must retain its lane");
        if lane
            .latest_committed_token
            .is_none_or(|latest| latest.write_id < token.write_id)
        {
            lane.latest_committed_token = Some(token);
        }
        lane.optimistic_page = optimistic_page;
        lane.deferred_authoritative_page = Some((token, authoritative_page));
    }

    pub(crate) fn reconcile_mutation_result(
        &mut self,
        page: &mut CardPage,
        token: PageMutationRunToken,
        visible: bool,
    ) {
        let page_id = page.block_id.clone();
        let lane = self
            .lanes
            .get_mut(&page_id)
            .expect("page mutation result must retain its lane");
        if lane
            .deferred_authoritative_page
            .as_ref()
            .is_some_and(|(deferred_token, _)| token.acknowledges(*deferred_token))
        {
            lane.deferred_authoritative_page = None;
        }
        lane.committed_text.overlay(page, Some(token));
        if lane.is_idle() {
            lane.optimistic_page = None;
            if visible
                && lane.committed_text.is_empty()
                && lane.deferred_authoritative_page.is_none()
            {
                lane.latest_committed_token = None;
            }
        }
        if visible {
            self.remove_lane_if_acknowledged(&page_id);
        }
    }

    pub(crate) fn reconcile_external_load(
        &mut self,
        page: &mut CardPage,
        authority: Option<PageMutationRunToken>,
    ) -> Arc<CardPage> {
        let page_id = page.block_id.clone();
        let Some(lane) = self.lanes.get_mut(&page_id) else {
            return Arc::new(page.clone());
        };
        if !lane.is_idle() && lane.optimistic_page.is_some() {
            return Self::reconcile_external_load_with_optimism(lane, page, authority);
        }
        let deferred_is_acknowledged = lane
            .deferred_authoritative_page
            .as_ref()
            .is_some_and(|(token, _)| authority.is_some_and(|value| value.acknowledges(*token)));
        if deferred_is_acknowledged {
            lane.deferred_authoritative_page = None;
        } else if let Some((_, deferred)) = lane.deferred_authoritative_page.as_ref() {
            page.clone_from(deferred.as_ref());
        }
        lane.committed_text.overlay(page, authority);
        let usable_authority = Arc::new(page.clone());
        Self::acknowledge_external_watermark(lane, authority);
        self.remove_lane_if_acknowledged(&page_id);
        usable_authority
    }

    fn reconcile_external_load_with_optimism(
        lane: &mut PageMutationLane,
        page: &mut CardPage,
        authority: Option<PageMutationRunToken>,
    ) -> Arc<CardPage> {
        lane.committed_text.overlay(page, authority);
        if lane
            .deferred_authoritative_page
            .as_ref()
            .is_some_and(|(token, _)| authority.is_some_and(|value| value.acknowledges(*token)))
        {
            lane.deferred_authoritative_page = None;
        }
        let usable_authority = Arc::new(page.clone());
        page.clone_from(
            lane.optimistic_page
                .as_ref()
                .expect("active page mutation work must retain its optimistic page")
                .as_ref(),
        );
        lane.committed_text.overlay(page, None);
        usable_authority
    }

    fn acknowledge_external_watermark(
        lane: &mut PageMutationLane,
        authority: Option<PageMutationRunToken>,
    ) {
        let watermark_is_acknowledged = authority.is_some_and(|authority| {
            lane.latest_committed_token
                .is_some_and(|latest| authority.acknowledges(latest))
        });
        if watermark_is_acknowledged
            && lane.committed_text.is_empty()
            && lane.deferred_authoritative_page.is_none()
        {
            lane.latest_committed_token = None;
        }
    }

    fn remove_lane_if_acknowledged(&mut self, page_id: &str) {
        if self
            .lanes
            .get(page_id)
            .is_some_and(PageMutationLane::can_remove)
        {
            self.lanes.remove(page_id);
        }
    }
}

impl PageMutationRunToken {
    fn acknowledges(self, token: Self) -> bool {
        self.epoch == token.epoch && self.write_id >= token.write_id
    }
}
