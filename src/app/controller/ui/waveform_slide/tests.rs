use super::rotation::rotate_interleaved_samples;

#[test]
fn rotate_interleaved_samples_wraps_frames() {
    let samples = vec![1.0, -1.0, 2.0, -2.0, 3.0, -3.0];
    let rotated = rotate_interleaved_samples(&samples, 2, 1);
    assert_eq!(rotated, vec![3.0, -3.0, 1.0, -1.0, 2.0, -2.0]);
    let rotated_back = rotate_interleaved_samples(&samples, 2, -1);
    assert_eq!(rotated_back, vec![2.0, -2.0, 3.0, -3.0, 1.0, -1.0]);
}
