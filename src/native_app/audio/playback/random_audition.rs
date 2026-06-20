const MIN_RANDOM_AUDITION_SECONDS: f32 = 0.25;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::native_app) enum RandomAuditionSource {
    WholeSample,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(in crate::native_app) struct RandomAuditionUnits {
    pub(in crate::native_app) start: f32,
    pub(in crate::native_app) length: f32,
}

impl RandomAuditionUnits {
    pub(in crate::native_app) const fn new(start: f32, length: f32) -> Self {
        Self { start, length }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(in crate::native_app) struct RandomAuditionSpan {
    pub(in crate::native_app) start: f32,
    pub(in crate::native_app) end: f32,
    pub(in crate::native_app) source: RandomAuditionSource,
}

impl RandomAuditionSpan {
    pub(in crate::native_app) fn status_message(self, file_name: &str) -> String {
        match self.source {
            RandomAuditionSource::WholeSample => {
                format!(
                    "Random audition {file_name} from {:.1}%",
                    self.start * 100.0
                )
            }
        }
    }
}

pub(in crate::native_app) fn random_audition_span_for_units(
    duration_seconds: f32,
    units: RandomAuditionUnits,
) -> (f32, f32) {
    if !duration_seconds.is_finite() || duration_seconds <= MIN_RANDOM_AUDITION_SECONDS {
        return (0.0, 1.0);
    }

    let min_width = (MIN_RANDOM_AUDITION_SECONDS / duration_seconds).clamp(0.0, 1.0);
    if min_width >= 1.0 {
        return (0.0, 1.0);
    }

    let max_start = 1.0 - min_width;
    let start = units.start.clamp(0.0, 1.0) * max_start;
    let max_width = 1.0 - start;
    let width = min_width + units.length.clamp(0.0, 1.0) * (max_width - min_width);
    (start, start + width)
}
