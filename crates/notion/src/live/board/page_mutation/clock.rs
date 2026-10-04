use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};

use crate::live::board::page_state::{CrdtItemId, CrdtOperationId};

#[derive(Clone, Debug)]
pub(in crate::live::board) struct CrdtClock {
    session_id: String,
    next_tick: u64,
}

impl CrdtClock {
    pub(super) fn new() -> Self {
        let unique_session = uuid::Uuid::new_v4();
        let session_id = URL_SAFE_NO_PAD.encode(&unique_session.as_bytes()[..9]);
        Self {
            session_id,
            next_tick: 1,
        }
    }

    pub(super) fn reserve(&mut self, length: usize) -> Result<CrdtOperationId, String> {
        let length = u64::try_from(length).map_err(|_| "CRDT text is too long".to_string())?;
        let id = CrdtOperationId(self.session_id.clone(), self.next_tick);
        self.next_tick = self
            .next_tick
            .checked_add(length)
            .ok_or_else(|| "CRDT operation clock overflow".to_string())?;
        Ok(id)
    }

    pub(super) fn reserve_after(
        &mut self,
        origin_id: &CrdtItemId,
        length: usize,
    ) -> Result<CrdtOperationId, String> {
        self.advance_after(origin_id)?;
        self.reserve(length)
    }

    pub(super) fn reserve_after_with_clock_floor(
        &mut self,
        origin_id: &CrdtItemId,
        clock_floor: &CrdtOperationId,
        length: usize,
    ) -> Result<CrdtOperationId, String> {
        self.advance_after(origin_id)?;
        self.advance_after_operation(clock_floor)?;
        self.reserve(length)
    }

    fn advance_after(&mut self, origin_id: &CrdtItemId) -> Result<(), String> {
        match origin_id {
            CrdtItemId::Boundary(boundary) if boundary == "start" => {}
            CrdtItemId::Boundary(boundary) => {
                return Err(format!(
                    "cannot reserve a CRDT operation after boundary {boundary}"
                ));
            }
            CrdtItemId::Operation(origin) => self.advance_after_operation(origin)?,
        }
        Ok(())
    }

    fn advance_after_operation(&mut self, origin: &CrdtOperationId) -> Result<(), String> {
        let next_origin_tick = origin
            .1
            .checked_add(1)
            .ok_or_else(|| "CRDT operation origin clock overflow".to_string())?;
        self.next_tick = self.next_tick.max(next_origin_tick);
        Ok(())
    }

    pub(super) fn skip(&mut self, ticks: u64) -> Result<(), String> {
        self.next_tick = self
            .next_tick
            .checked_add(ticks)
            .ok_or_else(|| "CRDT operation clock overflow".to_string())?;
        Ok(())
    }
}
