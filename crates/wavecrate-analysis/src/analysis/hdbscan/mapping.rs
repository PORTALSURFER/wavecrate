use std::collections::HashMap;

pub(super) fn assign_all_points_to_clusters(data: &[Vec<f32>], labels: &mut [i32]) {
    if data.is_empty() || labels.is_empty() || labels.iter().all(|label| *label >= 0) {
        return;
    }
    let mut centroid_sums: HashMap<i32, (Vec<f32>, usize)> = HashMap::new();
    for (index, label) in labels.iter().enumerate() {
        if *label < 0 {
            continue;
        }
        let Some(point) = data.get(index) else {
            continue;
        };
        let entry = centroid_sums
            .entry(*label)
            .or_insert_with(|| (vec![0.0; point.len()], 0));
        for (dimension, value) in point.iter().enumerate() {
            if let Some(sum) = entry.0.get_mut(dimension) {
                *sum += *value;
            }
        }
        entry.1 += 1;
    }
    if centroid_sums.is_empty() {
        labels.fill(0);
        return;
    }
    let centroids = centroid_sums
        .into_iter()
        .filter_map(|(label, (mut sums, count))| {
            if count == 0 {
                return None;
            }
            for value in &mut sums {
                *value /= count as f32;
            }
            Some((label, sums))
        })
        .collect::<Vec<_>>();
    for (index, label) in labels.iter_mut().enumerate() {
        if *label >= 0 {
            continue;
        }
        let Some(point) = data.get(index) else {
            continue;
        };
        *label = centroids
            .iter()
            .min_by(|(_, left), (_, right)| {
                squared_distance(point, left)
                    .partial_cmp(&squared_distance(point, right))
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .map_or(0, |(cluster, _)| *cluster);
    }
}

pub(super) fn remap_labels_deterministic(
    sample_ids: &[String],
    labels: &mut [i32],
) -> Result<(), String> {
    if sample_ids.len() != labels.len() {
        return Err("Cluster label length mismatch".to_string());
    }
    let mut next_label = 0;
    let mut mapping = HashMap::new();
    for label in labels {
        let canonical = mapping.entry(*label).or_insert_with(|| {
            let assigned = next_label;
            next_label += 1;
            assigned
        });
        *label = *canonical;
    }
    Ok(())
}

fn squared_distance(left: &[f32], right: &[f32]) -> f32 {
    left.iter()
        .zip(right)
        .map(|(left, right)| (*left - *right) * (*left - *right))
        .sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn assigns_noise_points_to_nearest_centroid() {
        let data = vec![vec![0.0, 0.0], vec![1.0, 1.0], vec![10.0, 10.0]];
        let mut labels = vec![0, -1, 1];
        assign_all_points_to_clusters(&data, &mut labels);
        assert_eq!(labels, vec![0, 0, 1]);
    }

    #[test]
    fn remaps_labels_by_manifest_order() {
        let sample_ids = vec!["a".to_string(), "b".to_string(), "c".to_string()];
        let mut labels = vec![5, 5, 2];
        remap_labels_deterministic(&sample_ids, &mut labels).unwrap();
        assert_eq!(labels, vec![0, 0, 1]);
    }
}
