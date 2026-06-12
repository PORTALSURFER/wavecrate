/// Return `true` when one segment key needs rematerialization.
pub(super) fn segment_key_changed<T: PartialEq>(
    has_retained_model: bool,
    cached_key: &Option<T>,
    next_key: &T,
) -> bool {
    !has_retained_model || cached_key.as_ref() != Some(next_key)
}
