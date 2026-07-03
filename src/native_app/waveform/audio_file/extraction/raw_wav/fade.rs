use std::io::{Read, Write};

use wavecrate::audio::short_edge_fade_gain;

use super::{RawWavLayout, SampleEncoding, copy_exact_data};

pub(super) fn write_faded_data<R: Read, W: Write>(
    reader: &mut R,
    writer: &mut W,
    layout: &RawWavLayout,
    frame_count: u64,
    fade_frames: usize,
    gain: f32,
) -> Result<(), String> {
    let fade_frames = fade_frames as u64;
    let block_align = usize::from(layout.block_align);
    let mut frame_bytes = vec![0_u8; block_align];
    let mut frame = 0_u64;
    while frame < frame_count {
        if (gain - 1.0).abs() <= f32::EPSILON
            && frame >= fade_frames
            && frame_count.saturating_sub(frame) > fade_frames
        {
            let middle_frames = frame_count
                .saturating_sub(fade_frames)
                .saturating_sub(frame);
            copy_exact_data(
                reader,
                writer,
                middle_frames.saturating_mul(u64::from(layout.block_align)),
            )?;
            frame = frame.saturating_add(middle_frames);
            continue;
        }

        reader
            .read_exact(&mut frame_bytes)
            .map_err(|err| format!("failed to read WAV selection frame: {err}"))?;
        let gain =
            gain * short_edge_fade_gain(frame as usize, frame_count as usize, fade_frames as usize);
        scale_frame_samples(&mut frame_bytes, layout, gain)?;
        writer
            .write_all(&frame_bytes)
            .map_err(|err| format!("failed to write WAV selection frame: {err}"))?;
        frame = frame.saturating_add(1);
    }
    Ok(())
}

fn scale_frame_samples(frame: &mut [u8], layout: &RawWavLayout, gain: f32) -> Result<(), String> {
    if (gain - 1.0).abs() <= f32::EPSILON {
        return Ok(());
    }
    let bytes_per_sample = usize::from(layout.bits_per_sample / 8);
    if bytes_per_sample == 0 {
        return Err(String::from("WAV sample width is invalid"));
    }
    for channel in 0..usize::from(layout.channels) {
        let offset = channel.saturating_mul(bytes_per_sample);
        let sample = frame
            .get_mut(offset..offset + bytes_per_sample)
            .ok_or_else(|| String::from("WAV frame is shorter than its format"))?;
        match (layout.encoding, layout.bits_per_sample) {
            (SampleEncoding::Int, 8) => scale_u8_pcm(sample, gain),
            (SampleEncoding::Int, 16) => scale_i16_pcm(sample, gain),
            (SampleEncoding::Int, 24) => scale_i24_pcm(sample, gain),
            (SampleEncoding::Int, 32) => scale_i32_pcm(sample, gain),
            (SampleEncoding::Float, 32) => scale_f32_pcm(sample, gain),
            _ => return Err(String::from("WAV sample format is unsupported for fading")),
        }
    }
    Ok(())
}

fn scale_u8_pcm(bytes: &mut [u8], gain: f32) {
    let centered = f32::from(bytes[0]) - 128.0;
    bytes[0] = (centered * gain + 128.0).round().clamp(0.0, 255.0) as u8;
}

fn scale_i16_pcm(bytes: &mut [u8], gain: f32) {
    let scaled = (f32::from(i16::from_le_bytes([bytes[0], bytes[1]])) * gain)
        .round()
        .clamp(i16::MIN as f32, i16::MAX as f32) as i16;
    bytes.copy_from_slice(&scaled.to_le_bytes());
}

fn scale_i24_pcm(bytes: &mut [u8], gain: f32) {
    let raw = i32::from_le_bytes([
        bytes[0],
        bytes[1],
        bytes[2],
        if bytes[2] & 0x80 == 0 { 0x00 } else { 0xff },
    ]);
    let scaled = (raw as f32 * gain).round().clamp(-8_388_608.0, 8_388_607.0) as i32;
    let out = scaled.to_le_bytes();
    bytes.copy_from_slice(&out[..3]);
}

fn scale_i32_pcm(bytes: &mut [u8], gain: f32) {
    let scaled = (i32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) as f32 * gain)
        .round()
        .clamp(i32::MIN as f32, i32::MAX as f32) as i32;
    bytes.copy_from_slice(&scaled.to_le_bytes());
}

fn scale_f32_pcm(bytes: &mut [u8], gain: f32) {
    let scaled = f32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) * gain;
    bytes.copy_from_slice(&scaled.clamp(-1.0, 1.0).to_le_bytes());
}
