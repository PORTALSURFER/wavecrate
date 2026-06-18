//! Aspect descriptor helpers for Sononym-style similarity feedback.

use super::similarity::{SIMILARITY_DIM, normalize_l2_in_place};
use super::vector::FEATURE_VECTOR_LEN_V1;

/// Model identifier for aspect descriptors derived from the V1 feature vector.
pub const ASPECT_DESCRIPTOR_MODEL_ID: &str = "features_v1__ostpa_aspects_v1";
/// Data type label for stored aspect descriptors.
pub const ASPECT_DESCRIPTOR_DTYPE_F32: &str = "f32";
/// Number of visual similarity aspects in stable display order.
pub const ASPECT_COUNT: usize = 5;

const OVERALL_OFFSET: usize = 0;
const OVERALL_DIM: usize = SIMILARITY_DIM;
const SPECTRUM_OFFSET: usize = OVERALL_OFFSET + OVERALL_DIM;
const SPECTRUM_DIM: usize = 48;
const TIMBRE_OFFSET: usize = SPECTRUM_OFFSET + SPECTRUM_DIM;
const TIMBRE_DIM: usize = 126;
const PITCH_OFFSET: usize = TIMBRE_OFFSET + TIMBRE_DIM;
const PITCH_DIM: usize = 21;
const AMPLITUDE_OFFSET: usize = PITCH_OFFSET + PITCH_DIM;
const AMPLITUDE_DIM: usize = 8;

/// Number of `f32` values stored in a packed aspect descriptor set.
pub const ASPECT_DESCRIPTOR_DIM: usize = AMPLITUDE_OFFSET + AMPLITUDE_DIM;

/// Stable sound-character aspects shown by Wavecrate similarity feedback.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SimilarityAspect {
    /// Existing full-vector similarity.
    Overall,
    /// Spectral shape: centroid, rolloff, bandwidth, and band-energy ratios.
    Spectrum,
    /// Timbre proxy: MFCC statistics plus spectral flatness.
    Timbre,
    /// Current pitch proxy. V1 features do not contain true pitch/chroma data.
    Pitch,
    /// Amplitude envelope and level features.
    Amplitude,
}

impl SimilarityAspect {
    /// Stable ordered aspect list used for storage and row display.
    pub const ORDER: [Self; ASPECT_COUNT] = [
        Self::Overall,
        Self::Spectrum,
        Self::Timbre,
        Self::Pitch,
        Self::Amplitude,
    ];

    /// Bit used in validity masks for this aspect.
    pub const fn mask_bit(self) -> u32 {
        1 << self.index()
    }

    /// Stable display/index position.
    pub const fn index(self) -> usize {
        match self {
            Self::Overall => 0,
            Self::Spectrum => 1,
            Self::Timbre => 2,
            Self::Pitch => 3,
            Self::Amplitude => 4,
        }
    }

    fn range(self) -> std::ops::Range<usize> {
        match self {
            Self::Overall => OVERALL_OFFSET..OVERALL_OFFSET + OVERALL_DIM,
            Self::Spectrum => SPECTRUM_OFFSET..SPECTRUM_OFFSET + SPECTRUM_DIM,
            Self::Timbre => TIMBRE_OFFSET..TIMBRE_OFFSET + TIMBRE_DIM,
            Self::Pitch => PITCH_OFFSET..PITCH_OFFSET + PITCH_DIM,
            Self::Amplitude => AMPLITUDE_OFFSET..AMPLITUDE_OFFSET + AMPLITUDE_DIM,
        }
    }
}

/// Packed normalized descriptors plus a per-aspect validity mask.
#[derive(Clone, Debug, PartialEq)]
pub struct AspectDescriptorSet {
    packed: Vec<f32>,
    valid_mask: u32,
}

impl AspectDescriptorSet {
    /// Derive a normalized descriptor set from a V1 feature vector.
    pub fn from_feature_vector_v1(features: &[f32]) -> Result<Self, String> {
        if features.len() != FEATURE_VECTOR_LEN_V1 {
            return Err(format!(
                "Aspect features length mismatch: expected {FEATURE_VECTOR_LEN_V1}, got {}",
                features.len()
            ));
        }

        let mut packed = vec![0.0; ASPECT_DESCRIPTOR_DIM];
        let mut valid_mask = 0;
        push_aspect(
            features,
            SimilarityAspect::Overall,
            &OVERALL_INDICES,
            &mut packed,
            &mut valid_mask,
        );
        push_aspect(
            features,
            SimilarityAspect::Spectrum,
            &SPECTRUM_INDICES,
            &mut packed,
            &mut valid_mask,
        );
        push_aspect(
            features,
            SimilarityAspect::Timbre,
            &TIMBRE_INDICES,
            &mut packed,
            &mut valid_mask,
        );
        push_aspect(
            features,
            SimilarityAspect::Pitch,
            &PITCH_PROXY_INDICES,
            &mut packed,
            &mut valid_mask,
        );
        push_aspect(
            features,
            SimilarityAspect::Amplitude,
            &AMPLITUDE_INDICES,
            &mut packed,
            &mut valid_mask,
        );

        Ok(Self { packed, valid_mask })
    }

    /// Rebuild a descriptor set from a packed little-endian blob and validity mask.
    pub fn from_parts(packed: Vec<f32>, valid_mask: u32) -> Result<Self, String> {
        if packed.len() != ASPECT_DESCRIPTOR_DIM {
            return Err(format!(
                "Aspect descriptor length mismatch: expected {ASPECT_DESCRIPTOR_DIM}, got {}",
                packed.len()
            ));
        }
        Ok(Self {
            packed,
            valid_mask: valid_mask & all_aspect_mask(),
        })
    }

    /// Packed descriptor values in stable aspect order.
    pub fn packed(&self) -> &[f32] {
        &self.packed
    }

    /// Bitmask of aspects that normalized successfully.
    pub fn valid_mask(&self) -> u32 {
        self.valid_mask
    }

    /// Descriptor slice for one valid aspect.
    pub fn descriptor(&self, aspect: SimilarityAspect) -> Option<&[f32]> {
        if !self.is_valid(aspect) {
            return None;
        }
        Some(&self.packed[aspect.range()])
    }

    /// Whether an aspect has a usable normalized descriptor.
    pub fn is_valid(&self, aspect: SimilarityAspect) -> bool {
        self.valid_mask & aspect.mask_bit() != 0
    }

    /// Cosine score for one aspect against another descriptor set.
    pub fn cosine_with(&self, other: &Self, aspect: SimilarityAspect) -> Option<f32> {
        let left = self.descriptor(aspect)?;
        let right = other.descriptor(aspect)?;
        Some(dot(left, right).clamp(-1.0, 1.0))
    }
}

/// Create an aspect descriptor set from a V1 feature vector.
pub fn aspect_descriptors_from_features_v1(
    features: &[f32],
) -> Result<AspectDescriptorSet, String> {
    AspectDescriptorSet::from_feature_vector_v1(features)
}

/// Stable mask containing every known aspect bit.
pub const fn all_aspect_mask() -> u32 {
    (1 << ASPECT_COUNT) - 1
}

fn push_aspect(
    features: &[f32],
    aspect: SimilarityAspect,
    indices: &[usize],
    packed: &mut [f32],
    valid_mask: &mut u32,
) {
    let range = aspect.range();
    debug_assert_eq!(range.len(), indices.len());
    for (dst, &index) in packed[range.clone()].iter_mut().zip(indices) {
        *dst = features[index];
    }
    if normalize_l2_in_place(&mut packed[range]) {
        *valid_mask |= aspect.mask_bit();
    }
}

fn dot(left: &[f32], right: &[f32]) -> f32 {
    left.iter().zip(right).map(|(a, b)| a * b).sum()
}

const OVERALL_INDICES: [usize; OVERALL_DIM] = make_full_feature_indices();
const SPECTRUM_INDICES: [usize; SPECTRUM_DIM] = [
    9, 10, 11, 12, 15, 16, 17, 18, 19, 20, 23, 24, 25, 26, 27, 28, 31, 32, 33, 34, 35, 36, 37, 38,
    39, 40, 41, 42, 43, 44, 45, 46, 47, 48, 49, 50, 51, 52, 53, 54, 55, 56, 57, 58, 59, 60, 61, 62,
];
const TIMBRE_INDICES: [usize; TIMBRE_DIM] = [
    13, 14, 21, 22, 29, 30, 63, 64, 65, 66, 67, 68, 69, 70, 71, 72, 73, 74, 75, 76, 77, 78, 79, 80,
    81, 82, 83, 84, 85, 86, 87, 88, 89, 90, 91, 92, 93, 94, 95, 96, 97, 98, 99, 100, 101, 102, 103,
    104, 105, 106, 107, 108, 109, 110, 111, 112, 113, 114, 115, 116, 117, 118, 119, 120, 121, 122,
    123, 124, 125, 126, 127, 128, 129, 130, 131, 132, 133, 134, 135, 136, 137, 138, 139, 140, 141,
    142, 143, 144, 145, 146, 147, 148, 149, 150, 151, 152, 153, 154, 155, 156, 157, 158, 159, 160,
    161, 162, 163, 164, 165, 166, 167, 168, 169, 170, 171, 172, 173, 174, 175, 176, 177, 178, 179,
    180, 181, 182,
];
const PITCH_PROXY_INDICES: [usize; PITCH_DIM] = [
    4, 9, 10, 11, 12, 17, 19, 25, 27, 33, 35, 37, 39, 41, 43, 45, 47, 49, 51, 53, 61,
];
const AMPLITUDE_INDICES: [usize; AMPLITUDE_DIM] = [0, 1, 2, 3, 5, 6, 7, 8];

const fn make_full_feature_indices() -> [usize; OVERALL_DIM] {
    let mut indices = [0; OVERALL_DIM];
    let mut index = 0;
    while index < OVERALL_DIM {
        indices[index] = index;
        index += 1;
    }
    indices
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn descriptor_set_has_stable_layout() {
        let mut features = vec![0.0; FEATURE_VECTOR_LEN_V1];
        for (index, feature) in features.iter_mut().enumerate() {
            *feature = index as f32 + 1.0;
        }

        let descriptors = AspectDescriptorSet::from_feature_vector_v1(&features).unwrap();

        assert_eq!(descriptors.packed().len(), ASPECT_DESCRIPTOR_DIM);
        assert_eq!(descriptors.valid_mask(), all_aspect_mask());
        for aspect in SimilarityAspect::ORDER {
            let descriptor = descriptors.descriptor(aspect).unwrap();
            let norm = descriptor
                .iter()
                .map(|value| value * value)
                .sum::<f32>()
                .sqrt();
            assert!((norm - 1.0).abs() < 1e-5);
        }
    }

    #[test]
    fn zero_slices_remain_unavailable() {
        let features = vec![0.0; FEATURE_VECTOR_LEN_V1];

        let descriptors = AspectDescriptorSet::from_feature_vector_v1(&features).unwrap();

        assert_eq!(descriptors.valid_mask(), 0);
        assert!(descriptors.descriptor(SimilarityAspect::Spectrum).is_none());
    }

    #[test]
    fn cosine_returns_none_for_missing_aspects() {
        let features = vec![0.0; FEATURE_VECTOR_LEN_V1];
        let descriptors = AspectDescriptorSet::from_feature_vector_v1(&features).unwrap();

        assert!(
            descriptors
                .cosine_with(&descriptors, SimilarityAspect::Amplitude)
                .is_none()
        );
    }

    #[test]
    fn cosine_scores_identical_descriptors_at_one() {
        let mut features = vec![0.0; FEATURE_VECTOR_LEN_V1];
        for (index, feature) in features.iter_mut().enumerate() {
            *feature = index as f32 + 1.0;
        }
        let descriptors = AspectDescriptorSet::from_feature_vector_v1(&features).unwrap();

        let score = descriptors
            .cosine_with(&descriptors, SimilarityAspect::Timbre)
            .unwrap();

        assert!((score - 1.0).abs() < 1e-5);
    }

    #[test]
    fn rejects_wrong_feature_length() {
        let err = AspectDescriptorSet::from_feature_vector_v1(&[1.0, 2.0]).unwrap_err();
        assert!(err.contains("length mismatch"));
    }
}
