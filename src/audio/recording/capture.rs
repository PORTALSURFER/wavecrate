//! CPAL recording bootstrap and capture-loop wiring.

use std::path::Path;
use std::sync::mpsc;

use cpal::Stream;

use super::monitor::{MonitorSenderSlot, forward_monitor_samples, new_monitor_sender_slot};
use super::writer::{RecorderCommand, RecorderWriter};
use crate::audio::input::{
    AudioInputConfig, AudioInputError, ResolvedInput, StreamChannelSelection, build_input_stream,
    resolve_input_stream_config,
};

/// Bundled runtime pieces returned when a recording session starts successfully.
pub(super) struct RecordingRuntime {
    pub(super) stream: Stream,
    pub(super) writer: RecorderWriter,
    pub(super) resolved: ResolvedInput,
    pub(super) monitor_sender: MonitorSenderSlot,
}

/// Resolve input settings, start the WAV writer, and build the live capture stream.
pub(super) fn start_recording_runtime(
    config: &AudioInputConfig,
    path: &Path,
) -> Result<RecordingRuntime, AudioInputError> {
    let resolved = resolve_input_stream_config(config)?;
    let selection =
        StreamChannelSelection::new(resolved.stream_config.channels, &resolved.selected_channels);
    let (sender, receiver) = mpsc::channel();
    let monitor_sender = new_monitor_sender_slot();
    let writer = RecorderWriter::spawn(
        path.to_path_buf(),
        resolved.resolved.sample_rate,
        resolved.resolved.recorded_channel_count,
        receiver,
        sender.clone(),
    )?;
    let sender_clone = sender.clone();
    let monitor_sender_clone = monitor_sender.clone();
    let stream = build_input_stream(
        &resolved.device,
        &resolved.stream_config,
        resolved.sample_format,
        selection,
        move |samples| {
            forward_monitor_samples(&monitor_sender_clone, &samples);
            let _ = sender_clone.send(RecorderCommand::Samples(samples));
        },
    )?;
    Ok(RecordingRuntime {
        stream,
        writer,
        resolved: resolved.resolved,
        monitor_sender,
    })
}
