use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, AtomicUsize};
use std::sync::mpsc::{self, Receiver, SyncSender};

use cpal::traits::DeviceTrait;
use cpal::{FromSample, SizedSample};
use tracing::warn;

use crate::output::callback::{CallbackState, StreamCommand, process_audio_callback};

/// Raw stream construction pieces returned before wrapping in `CpalAudioStream`.
pub(super) struct BuiltStreamState {
    pub(super) stream: cpal::Stream,
    pub(super) command_sender: SyncSender<StreamCommand>,
    pub(super) error_receiver: Receiver<String>,
    pub(super) clear_pending: Arc<AtomicBool>,
    pub(super) command_generation: Arc<AtomicU64>,
}

pub(super) fn build_stream_with_state(
    device: &cpal::Device,
    stream_config: &cpal::StreamConfig,
    sample_format: cpal::SampleFormat,
    volume_bits: Arc<AtomicU32>,
    active_sources: Arc<AtomicUsize>,
    clear_pending: Arc<AtomicBool>,
    command_generation: Arc<AtomicU64>,
) -> Result<BuiltStreamState, cpal::BuildStreamError> {
    const COMMAND_QUEUE_CAPACITY: usize = 512;
    let (command_sender, command_receiver) = mpsc::sync_channel(COMMAND_QUEUE_CAPACITY);
    let (error_sender, error_receiver) = mpsc::channel();
    let callback_state = CallbackState::new(
        command_receiver,
        error_sender,
        volume_bits,
        active_sources,
        clear_pending.clone(),
        command_generation.clone(),
    );
    let stream = match sample_format {
        cpal::SampleFormat::F32 => {
            build_typed_output_stream::<f32>(device, stream_config, callback_state)?
        }
        cpal::SampleFormat::F64 => {
            build_typed_output_stream::<f64>(device, stream_config, callback_state)?
        }
        cpal::SampleFormat::I8 => {
            build_typed_output_stream::<i8>(device, stream_config, callback_state)?
        }
        cpal::SampleFormat::I16 => {
            build_typed_output_stream::<i16>(device, stream_config, callback_state)?
        }
        cpal::SampleFormat::I24 => {
            build_typed_output_stream::<cpal::I24>(device, stream_config, callback_state)?
        }
        cpal::SampleFormat::I32 => {
            build_typed_output_stream::<i32>(device, stream_config, callback_state)?
        }
        cpal::SampleFormat::I64 => {
            build_typed_output_stream::<i64>(device, stream_config, callback_state)?
        }
        cpal::SampleFormat::U8 => {
            build_typed_output_stream::<u8>(device, stream_config, callback_state)?
        }
        cpal::SampleFormat::U16 => {
            build_typed_output_stream::<u16>(device, stream_config, callback_state)?
        }
        cpal::SampleFormat::U24 => {
            build_typed_output_stream::<cpal::U24>(device, stream_config, callback_state)?
        }
        cpal::SampleFormat::U32 => {
            build_typed_output_stream::<u32>(device, stream_config, callback_state)?
        }
        cpal::SampleFormat::U64 => {
            build_typed_output_stream::<u64>(device, stream_config, callback_state)?
        }
        format => {
            warn!("Unsupported output sample format {format:?}; trying f32 stream");
            build_typed_output_stream::<f32>(device, stream_config, callback_state)?
        }
    };
    Ok(BuiltStreamState {
        stream,
        command_sender,
        error_receiver,
        clear_pending,
        command_generation,
    })
}

fn build_typed_output_stream<T>(
    device: &cpal::Device,
    stream_config: &cpal::StreamConfig,
    mut callback_state: CallbackState,
) -> Result<cpal::Stream, cpal::BuildStreamError>
where
    T: SizedSample + FromSample<f32>,
{
    let mut scratch = Vec::new();
    let stream_error_sender = callback_state.error_sender();
    device.build_output_stream(
        stream_config,
        move |data: &mut [T], _: &cpal::OutputCallbackInfo| {
            scratch.resize(data.len(), 0.0);
            process_audio_callback(&mut callback_state, &mut scratch);
            for (sample_out, sample_in) in data.iter_mut().zip(scratch.iter().copied()) {
                *sample_out = T::from_sample(sample_in.clamp(-1.0, 1.0));
            }
        },
        move |err| {
            let message = format!("Audio output stream error: {err}");
            tracing::error!("{message}");
            let _ = stream_error_sender.send(message);
        },
        None,
    )
}
