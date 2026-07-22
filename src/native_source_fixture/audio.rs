use std::path::Path;

pub(super) struct AudioSpec {
    pub(super) channels: u16,
    pub(super) sample_rate: u32,
    pub(super) frames: u32,
    pub(super) seed: u32,
}

pub(super) fn write_deterministic_wav(path: &Path, spec: &AudioSpec) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|error| format!("create WAV parent {}: {error}", parent.display()))?;
    }
    let wav_spec = hound::WavSpec {
        channels: spec.channels,
        sample_rate: spec.sample_rate,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut writer = hound::WavWriter::create(path, wav_spec)
        .map_err(|error| format!("create deterministic WAV {}: {error}", path.display()))?;
    for frame in 0..spec.frames {
        for channel in 0..u32::from(spec.channels) {
            let phase = frame
                .wrapping_mul(1_103 + spec.seed.wrapping_mul(2))
                .wrapping_add(channel.wrapping_mul(7_919))
                .wrapping_add(spec.seed.wrapping_mul(977));
            let centered = (phase % 49_153) as i32 - 24_576;
            writer
                .write_sample((centered / 2) as i16)
                .map_err(|error| format!("write deterministic WAV {}: {error}", path.display()))?;
        }
    }
    writer
        .finalize()
        .map_err(|error| format!("finalize deterministic WAV {}: {error}", path.display()))
}
