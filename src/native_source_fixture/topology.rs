use super::FixtureName;

const LARGE_SOURCE_FILE_COUNT: usize = 10_000;

#[derive(Clone)]
pub(super) struct GeneratedAudio {
    pub(super) relative_path: String,
    pub(super) channels: u16,
    pub(super) sample_rate: u32,
    pub(super) frames: u32,
    pub(super) seed: u32,
}

pub(super) struct SourceDefinition {
    pub(super) id: &'static str,
    pub(super) directory_name: &'static str,
    pub(super) directories: Vec<String>,
    pub(super) audio: Vec<GeneratedAudio>,
    pub(super) unsupported: Vec<(&'static str, &'static [u8])>,
}

pub(super) fn definitions(fixture: FixtureName) -> Vec<SourceDefinition> {
    match fixture {
        FixtureName::Empty => Vec::new(),
        FixtureName::SmallMultiSource => small_multi_source_definitions(),
        FixtureName::LargeSource => vec![large_source_definition()],
    }
}

fn small_multi_source_definitions() -> Vec<SourceDefinition> {
    vec![
        SourceDefinition {
            id: "fixture-small-alpha-v1",
            directory_name: "source-alpha",
            directories: strings(&["drums", "textures/nested", "empty"]),
            audio: vec![
                audio("drums/kick-mono-44100.wav", 1, 44_100, 11_025, 11),
                audio("drums/snare-stereo-48000.wav", 2, 48_000, 18_000, 12),
                audio(
                    "textures/nested/pad-stereo-44100.wav",
                    2,
                    44_100,
                    22_050,
                    13,
                ),
                audio("textures/noise-mono-48000.wav", 1, 48_000, 9_600, 14),
            ],
            unsupported: Vec::new(),
        },
        SourceDefinition {
            id: "fixture-small-beta-v1",
            directory_name: "source-beta",
            directories: strings(&["loops", "oneshots", "mutable"]),
            audio: vec![
                audio("loops/loop-mono-48000.wav", 1, 48_000, 48_000, 21),
                audio("oneshots/hit-stereo-44100.wav", 2, 44_100, 8_820, 22),
                audio("mutable/change-me.wav", 1, 44_100, 5_512, 23),
                audio("mutable/move-me.wav", 1, 44_100, 5_512, 24),
                audio("mutable/delete-me.wav", 1, 44_100, 5_512, 25),
            ],
            unsupported: vec![(
                "fixture-note.txt",
                b"Wavecrate deterministic non-audio visibility fixture.\n",
            )],
        },
    ]
}

fn large_source_definition() -> SourceDefinition {
    let mut directories = Vec::with_capacity(100);
    let mut generated = Vec::with_capacity(LARGE_SOURCE_FILE_COUNT);
    for group in 0..100 {
        directories.push(format!("batch-{group:03}"));
    }
    for index in 0..LARGE_SOURCE_FILE_COUNT {
        let path = format!("batch-{:03}/sample-{index:05}.wav", index / 100);
        generated.push(audio(
            &path,
            if index % 3 == 0 { 2 } else { 1 },
            if index % 2 == 0 { 44_100 } else { 48_000 },
            256,
            index as u32 + 100,
        ));
    }
    SourceDefinition {
        id: "fixture-large-v1",
        directory_name: "source-large",
        directories,
        audio: generated,
        unsupported: Vec::new(),
    }
}

fn audio(
    relative_path: &str,
    channels: u16,
    sample_rate: u32,
    frames: u32,
    seed: u32,
) -> GeneratedAudio {
    GeneratedAudio {
        relative_path: relative_path.to_owned(),
        channels,
        sample_rate,
        frames,
        seed,
    }
}

fn strings(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| (*value).to_owned()).collect()
}
