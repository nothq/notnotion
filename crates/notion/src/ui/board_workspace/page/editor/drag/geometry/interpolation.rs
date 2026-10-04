use gpui::{Bounds, Pixels};

use super::{
    LoadedCardPageData, PageBlockDragGeometry, PageBlockDragLayout, PageBlockDragRowGeometry,
};

#[derive(Clone, Copy)]
struct PageBlockDragMeasurement {
    document_unit_index: usize,
    bounds: Bounds<Pixels>,
}

struct PageBlockDragMeasurements {
    rows: Vec<PageBlockDragMeasurement>,
    estimated_height: f32,
}

pub(super) fn page_block_drag_geometry(
    data: &LoadedCardPageData,
    layout: &PageBlockDragLayout,
) -> Option<PageBlockDragGeometry> {
    let measurements = PageBlockDragMeasurements::new(layout)?;
    let mut geometry = PageBlockDragGeometry {
        document_units: vec![None; data.document_units.len()],
    };
    geometry.record_measurements(&measurements);
    geometry.extrapolate_before_measurements(&measurements)?;
    geometry.interpolate_between_measurements(&measurements)?;
    geometry.extrapolate_after_measurements(&measurements, data.document_units.len())?;
    Some(geometry)
}

impl PageBlockDragMeasurements {
    fn new(layout: &PageBlockDragLayout) -> Option<Self> {
        let rows = layout
            .document_unit_indices
            .iter()
            .copied()
            .zip(&layout.row_bounds)
            .map(|(document_unit_index, bounds)| PageBlockDragMeasurement {
                document_unit_index,
                bounds: *bounds,
            })
            .collect::<Vec<_>>();
        if rows.is_empty() {
            return None;
        }
        let estimated_height = rows
            .iter()
            .map(|measurement| measurement.bounds.size.height.as_f32())
            .sum::<f32>()
            / rows.len() as f32;
        Some(Self {
            rows,
            estimated_height,
        })
    }
}

impl PageBlockDragGeometry {
    fn record_measurements(&mut self, measurements: &PageBlockDragMeasurements) {
        for measurement in &measurements.rows {
            self.document_units[measurement.document_unit_index] = Some(PageBlockDragRowGeometry {
                visual_top: measurement.bounds.top().as_f32(),
                visual_bottom: measurement.bounds.bottom().as_f32(),
            });
        }
    }

    fn extrapolate_before_measurements(
        &mut self,
        measurements: &PageBlockDragMeasurements,
    ) -> Option<()> {
        let first_unit = measurements.rows.first()?.document_unit_index;
        let mut next_top = self.document_units[first_unit]?.visual_top;
        for document_unit_index in (0..first_unit).rev() {
            self.document_units[document_unit_index] = Some(PageBlockDragRowGeometry {
                visual_top: next_top - measurements.estimated_height,
                visual_bottom: next_top,
            });
            next_top -= measurements.estimated_height;
        }
        Some(())
    }

    fn interpolate_between_measurements(
        &mut self,
        measurements: &PageBlockDragMeasurements,
    ) -> Option<()> {
        for measured_pair in measurements.rows.windows(2) {
            let previous_unit = measured_pair[0].document_unit_index;
            let next_unit = measured_pair[1].document_unit_index;
            let mut top = self.document_units[previous_unit]?.visual_bottom;
            for document_unit_index in previous_unit + 1..next_unit {
                self.document_units[document_unit_index] = Some(PageBlockDragRowGeometry {
                    visual_top: top,
                    visual_bottom: top + measurements.estimated_height,
                });
                top += measurements.estimated_height;
            }
        }
        Some(())
    }

    fn extrapolate_after_measurements(
        &mut self,
        measurements: &PageBlockDragMeasurements,
        document_unit_count: usize,
    ) -> Option<()> {
        let last_unit = measurements.rows.last()?.document_unit_index;
        let mut top = self.document_units[last_unit]?.visual_bottom;
        for document_unit_index in last_unit + 1..document_unit_count {
            self.document_units[document_unit_index] = Some(PageBlockDragRowGeometry {
                visual_top: top,
                visual_bottom: top + measurements.estimated_height,
            });
            top += measurements.estimated_height;
        }
        Some(())
    }
}
