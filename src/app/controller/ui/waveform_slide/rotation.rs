pub(super) fn rotate_interleaved_samples(
    samples: &[f32],
    channels: usize,
    offset_frames: isize,
) -> Vec<f32> {
    if samples.is_empty() || channels == 0 {
        return Vec::new();
    }
    let total_frames = samples.len() / channels;
    if total_frames == 0 {
        return Vec::new();
    }
    let offset = offset_frames.rem_euclid(total_frames as isize) as usize;
    if offset == 0 {
        return samples.to_vec();
    }
    let mut rotated = vec![0.0; samples.len()];
    for frame in 0..total_frames {
        let dest_frame = (frame + offset) % total_frames;
        let src = frame * channels;
        let dest = dest_frame * channels;
        rotated[dest..dest + channels].copy_from_slice(&samples[src..src + channels]);
    }
    rotated
}
