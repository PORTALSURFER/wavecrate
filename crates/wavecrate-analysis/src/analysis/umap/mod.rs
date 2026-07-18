//! Current starmap projection used by exact similarity publication.

mod projection;

use projection::compute_tsne;

type LayoutPoint = [f32; 2];

pub(crate) fn compute_layout_for_embeddings(
    embeddings: &[(String, Vec<f32>)],
    seed: u64,
    min_coverage: f32,
) -> Result<Vec<LayoutPoint>, String> {
    let Some((_, first)) = embeddings.first() else {
        return Ok(Vec::new());
    };
    if first.is_empty() {
        return Err("Similarity embedding dimension must be greater than zero".to_string());
    }
    let dim = first.len();
    if embeddings.len() <= 2 {
        return Ok(match embeddings.len() {
            1 => vec![[0.0, 0.0]],
            2 => vec![[-1.0, 0.0], [1.0, 0.0]],
            _ => Vec::new(),
        });
    }
    let mut vectors = Vec::with_capacity(embeddings.len().saturating_mul(dim));
    for (sample_id, embedding) in embeddings {
        if embedding.len() != dim {
            return Err(format!(
                "Embedding dim mismatch: expected {dim}, got {} for {sample_id}",
                embedding.len()
            ));
        }
        vectors.extend(embedding.iter().copied().map(f64::from));
    }
    let layout = compute_tsne(vectors, dim, seed)?;
    validate_layout(&layout, min_coverage)?;
    Ok(layout)
}

fn validate_layout(layout: &[LayoutPoint], min_coverage: f32) -> Result<(), String> {
    let valid = layout
        .iter()
        .filter(|point| point[0].is_finite() && point[1].is_finite())
        .count();
    let coverage = if layout.is_empty() {
        1.0
    } else {
        valid as f32 / layout.len() as f32
    };
    if coverage < min_coverage {
        return Err(format!(
            "Starmap layout coverage {coverage:.3} is below required {min_coverage:.3}"
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_layout_handles_small_manifests_deterministically() {
        assert!(
            compute_layout_for_embeddings(&[], 0, 0.95)
                .unwrap()
                .is_empty()
        );
        assert_eq!(
            compute_layout_for_embeddings(&[("a".to_string(), vec![1.0])], 0, 0.95).unwrap(),
            vec![[0.0, 0.0]]
        );
        assert_eq!(
            compute_layout_for_embeddings(
                &[("a".to_string(), vec![1.0]), ("b".to_string(), vec![0.0]),],
                0,
                0.95,
            )
            .unwrap(),
            vec![[-1.0, 0.0], [1.0, 0.0]]
        );
    }

    #[test]
    fn exact_layout_rejects_mixed_embedding_dimensions() {
        let error = compute_layout_for_embeddings(
            &[
                ("a".to_string(), vec![1.0, 0.0]),
                ("b".to_string(), vec![1.0]),
                ("c".to_string(), vec![0.0, 1.0]),
            ],
            0,
            0.95,
        )
        .unwrap_err();
        assert!(error.contains("dim mismatch"));
    }
}
