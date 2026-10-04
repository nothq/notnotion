use crate::ui::tests::profile::*;

const SIXTY_HZ_FRAME_BUDGET_MS: f64 = 16.7;
const THIRTY_HZ_FRAME_BUDGET_MS: f64 = 33.3;

pub(crate) fn summarize_profile_frames(
    label: &str,
    frame_durations: &[Duration],
    total_elapsed: Duration,
) -> String {
    let summary = FrameProfileSummary::from_durations(frame_durations, total_elapsed);
    format!(
        "captured {} {label} frames in {:?} (mean {:.2} ms, p50 {:.2} ms, p95 {:.2} ms, p99 {:.2} ms, max {:.2} ms, spikes >{:.1} ms: {}, >{:.1} ms: {})",
        summary.frame_count,
        summary.total_elapsed,
        summary.mean_ms,
        summary.p50_ms,
        summary.p95_ms,
        summary.p99_ms,
        summary.max_ms,
        SIXTY_HZ_FRAME_BUDGET_MS,
        summary.spikes_over_60hz_budget,
        THIRTY_HZ_FRAME_BUDGET_MS,
        summary.spikes_over_30hz_budget,
    )
}

#[derive(Debug, PartialEq)]
struct FrameProfileSummary {
    frame_count: usize,
    total_elapsed: Duration,
    mean_ms: f64,
    p50_ms: f64,
    p95_ms: f64,
    p99_ms: f64,
    max_ms: f64,
    spikes_over_60hz_budget: usize,
    spikes_over_30hz_budget: usize,
}

impl FrameProfileSummary {
    fn from_durations(frame_durations: &[Duration], total_elapsed: Duration) -> Self {
        let mut frame_millis = frame_durations
            .iter()
            .map(Duration::as_secs_f64)
            .map(|seconds| seconds * 1000.0)
            .collect::<Vec<_>>();
        frame_millis.sort_by(|left, right| left.total_cmp(right));
        let frame_count = frame_millis.len();
        let mean_ms = if frame_count == 0 {
            0.0
        } else {
            frame_millis.iter().sum::<f64>() / frame_count as f64
        };
        let spikes_over_60hz_budget = frame_millis
            .iter()
            .filter(|millis| **millis > SIXTY_HZ_FRAME_BUDGET_MS)
            .count();
        let spikes_over_30hz_budget = frame_millis
            .iter()
            .filter(|millis| **millis > THIRTY_HZ_FRAME_BUDGET_MS)
            .count();
        Self {
            frame_count,
            total_elapsed,
            mean_ms,
            p50_ms: percentile_millis(&frame_millis, 0.50),
            p95_ms: percentile_millis(&frame_millis, 0.95),
            p99_ms: percentile_millis(&frame_millis, 0.99),
            max_ms: frame_millis.last().copied().unwrap_or(0.0),
            spikes_over_60hz_budget,
            spikes_over_30hz_budget,
        }
    }
}

fn percentile_millis(sorted_frame_millis: &[f64], percentile: f64) -> f64 {
    if sorted_frame_millis.is_empty() {
        return 0.0;
    }
    let clamped_percentile = percentile.clamp(0.0, 1.0);
    let rank = (clamped_percentile * sorted_frame_millis.len() as f64).ceil() as usize;
    let index = rank.saturating_sub(1).min(sorted_frame_millis.len() - 1);
    sorted_frame_millis[index]
}

#[gpui::test]
fn frame_profile_summary_reports_tail_latency_and_spikes() {
    let summary = FrameProfileSummary::from_durations(
        &[
            Duration::from_millis(5),
            Duration::from_millis(6),
            Duration::from_millis(7),
            Duration::from_millis(50),
            Duration::from_millis(80),
        ],
        Duration::from_millis(148),
    );

    assert_eq!(summary.frame_count, 5);
    assert_eq!(summary.mean_ms, 29.6);
    assert_eq!(summary.p50_ms, 7.0);
    assert_eq!(summary.p95_ms, 80.0);
    assert_eq!(summary.p99_ms, 80.0);
    assert_eq!(summary.max_ms, 80.0);
    assert_eq!(summary.spikes_over_60hz_budget, 2);
    assert_eq!(summary.spikes_over_30hz_budget, 2);
}
