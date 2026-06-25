use radiant::gui::types::Rgba8;
use wavecrate::sample_sources::Rating;

const RATING_KEEP_COLOR: Rgba8 = Rgba8 {
    r: 122,
    g: 226,
    b: 96,
    a: 235,
};
const RATING_TRASH_COLOR: Rgba8 = Rgba8 {
    r: 238,
    g: 77,
    b: 67,
    a: 235,
};

#[derive(Clone, Debug)]
pub(super) struct RatingIndicator {
    rating: Rating,
    locked: bool,
}

impl RatingIndicator {
    pub(super) fn new(rating: Rating, locked: bool) -> Self {
        Self { rating, locked }
    }

    pub(super) fn count(&self) -> usize {
        self.rating.val().unsigned_abs().min(3) as usize
    }

    pub(super) fn color(&self) -> Option<Rgba8> {
        if self.rating.is_keep() {
            Some(RATING_KEEP_COLOR)
        } else if self.rating.is_trash() {
            Some(RATING_TRASH_COLOR)
        } else {
            None
        }
    }

    pub(super) fn shows_locked_keep_marker(&self) -> bool {
        self.locked && self.rating == Rating::KEEP_3
    }
}
