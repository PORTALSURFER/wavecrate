const RANDOM_AUDITION_SECONDS: f32 = 4.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::native_app) enum RandomAuditionSource {
    FixedWindow,
    MarkedRange,
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
            RandomAuditionSource::FixedWindow => {
                format!(
                    "Random audition {file_name} from {:.1}%",
                    self.start * 100.0
                )
            }
            RandomAuditionSource::MarkedRange => {
                format!(
                    "Random marked range {file_name} from {:.1}%",
                    self.start * 100.0
                )
            }
        }
    }
}

pub(in crate::native_app) fn random_audition_span_for_unit(
    duration_seconds: f32,
    unit: f32,
) -> (f32, f32) {
    if duration_seconds <= RANDOM_AUDITION_SECONDS {
        return (0.0, 1.0);
    }

    let width = (RANDOM_AUDITION_SECONDS / duration_seconds).clamp(0.0, 1.0);
    let max_start = 1.0 - width;
    let start = unit.clamp(0.0, 1.0) * max_start;
    (start, start + width)
}
