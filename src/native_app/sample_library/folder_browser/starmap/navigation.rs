use super::StarmapItem;

pub(super) fn starmap_navigation_target(
    items: &[StarmapItem],
    selected_file_id: &str,
    delta: i32,
) -> Option<String> {
    let current = find_starmap_item_by_file_id(items, selected_file_id)?;
    let direction = delta.signum() as f32;
    items
        .iter()
        .filter(|item| item.file_id != current.file_id)
        .filter(|item| (item.y - current.y) * direction > f32::EPSILON)
        .min_by(|left, right| {
            starmap_navigation_rank(current, left)
                .total_cmp(&starmap_navigation_rank(current, right))
                .then_with(|| left.file_id.cmp(&right.file_id))
        })
        .map(|item| item.file_id.clone())
}

pub(super) fn find_starmap_item_by_file_id<'a>(
    items: &'a [StarmapItem],
    file_id: &str,
) -> Option<&'a StarmapItem> {
    items.iter().find(|item| item.file_id.as_str() == file_id)
}

fn starmap_navigation_rank(current: &StarmapItem, candidate: &StarmapItem) -> f32 {
    let dx = candidate.x - current.x;
    let dy = candidate.y - current.y;
    dx * dx + dy * dy
}
