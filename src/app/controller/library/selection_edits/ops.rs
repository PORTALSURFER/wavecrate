use super::buffer::SelectionEditBuffer;

mod fades;
mod selection_fades;

#[cfg(test)]
pub(crate) use fades::fade_factor;
pub(crate) use fades::{apply_directional_fade, apply_edge_fades};
pub(crate) use selection_fades::{SelectionFadeRequest, apply_selection_fades};

pub(crate) fn crop_buffer(buffer: &mut SelectionEditBuffer) -> Result<(), String> {
    let cropped = slice_frames(
        &buffer.samples,
        buffer.channels,
        buffer.start_frame,
        buffer.end_frame,
    );
    if cropped.is_empty() {
        return Err("Selection has no audio to crop".into());
    }
    buffer.samples = cropped;
    Ok(())
}

pub(crate) fn trim_buffer(buffer: &mut SelectionEditBuffer) -> Result<(), String> {
    let total_frames = buffer.samples.len() / buffer.channels;
    if buffer.start_frame == 0 && buffer.end_frame >= total_frames {
        return Err("Cannot trim the entire file; crop instead".into());
    }
    let prefix_end = buffer.start_frame * buffer.channels;
    let suffix_start = buffer.end_frame * buffer.channels;
    let mut trimmed = Vec::with_capacity(
        buffer
            .samples
            .len()
            .saturating_sub(suffix_start - prefix_end),
    );
    trimmed.extend_from_slice(&buffer.samples[..prefix_end]);
    trimmed.extend_from_slice(&buffer.samples[suffix_start..]);
    if trimmed.is_empty() {
        return Err("Trim removed all audio; crop instead".into());
    }
    buffer.samples = trimmed;
    Ok(())
}

pub(crate) fn mute_buffer(buffer: &mut SelectionEditBuffer) -> Result<(), String> {
    apply_muted_selection(
        &mut buffer.samples,
        buffer.channels,
        buffer.start_frame,
        buffer.end_frame,
    );
    Ok(())
}

pub(crate) fn reverse_buffer(buffer: &mut SelectionEditBuffer) -> Result<(), String> {
    let channels = buffer.channels.max(1);
    let total_frames = buffer.samples.len() / channels;
    let start = buffer.start_frame.min(total_frames);
    let end = buffer.end_frame.min(total_frames);
    if end <= start + 1 {
        return Ok(());
    }
    let mut left = start;
    let mut right = end - 1;
    while left < right {
        let left_offset = left * channels;
        let right_offset = right * channels;
        for ch in 0..channels {
            buffer.samples.swap(left_offset + ch, right_offset + ch);
        }
        left += 1;
        right = right.saturating_sub(1);
    }
    Ok(())
}

pub(crate) fn slice_frames(
    samples: &[f32],
    channels: usize,
    start_frame: usize,
    end_frame: usize,
) -> Vec<f32> {
    let mut cropped = Vec::with_capacity((end_frame - start_frame) * channels);
    for frame in start_frame..end_frame {
        let offset = frame * channels;
        cropped.extend_from_slice(&samples[offset..offset + channels]);
    }
    cropped
}

pub(super) fn clamped_selection_span(
    total_frames: usize,
    start_frame: usize,
    end_frame: usize,
) -> (usize, usize) {
    let clamped_start = start_frame.min(total_frames);
    let clamped_end = end_frame.min(total_frames);
    (clamped_start, clamped_end)
}

pub(crate) fn apply_muted_selection(
    samples: &mut [f32],
    channels: usize,
    start_frame: usize,
    end_frame: usize,
) {
    if end_frame <= start_frame {
        return;
    }
    let channels = channels.max(1);
    let total_frames = samples.len() / channels;
    let clamped_start = start_frame.min(total_frames);
    let clamped_end = end_frame.min(total_frames);
    for frame in clamped_start..clamped_end {
        let offset = frame * channels;
        let frame_end = (offset + channels).min(samples.len());
        for sample in &mut samples[offset..frame_end] {
            *sample = 0.0;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::apply_edge_fades;

    #[test]
    fn edge_fades_ramp_selection_edges() {
        let mut samples = vec![1.0_f32; 4];
        apply_edge_fades(&mut samples, 1, 0, 4, 2);
        assert!((samples[0] - 0.0).abs() < 1e-6);
        assert!((samples[1] - 1.0).abs() < 1e-6);
        assert!((samples[2] - 1.0).abs() < 1e-6);
        assert!((samples[3] - 0.0).abs() < 1e-6);
    }
}
