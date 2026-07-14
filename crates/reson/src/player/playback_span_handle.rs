use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
};

use super::{PlaybackMetronomeConfig, PlaybackSpanPlan};

const NO_PENDING_SEEK: u64 = u64::MAX;

#[derive(Clone, Debug)]
pub(crate) struct PlaybackSpanHandle {
    shared: Arc<PlaybackSpanShared>,
}

#[derive(Debug)]
struct PlaybackSpanShared {
    pending: PlaybackControlMailbox,
    applied_metronome: AppliedMetronomeSlot,
}

/// Single-writer, single-audio-reader control mailbox.
///
/// The writer fills the inactive slot before one release-store publishes its
/// generation. The audio reader makes one bounded attempt per frame and keeps
/// its last coherent snapshot if publication changes during that attempt. It
/// never waits for the writer, and the writer never touches the published slot.
#[derive(Debug)]
struct PlaybackControlMailbox {
    published: AtomicU64,
    slots: [PlaybackControlSlot; 2],
}

#[derive(Debug)]
struct PlaybackControlSlot {
    start_frame: AtomicU64,
    end_frame: AtomicU64,
    pending_seek_frame: AtomicU64,
    metronome_enabled: AtomicU64,
    metronome_beat_count: AtomicU64,
    metronome_cycle_frames: AtomicU64,
    metronome_anchor_frame: AtomicU64,
    metronome_anchor_phase_frames: AtomicU64,
}

#[derive(Debug)]
struct AppliedMetronomeSlot {
    revision: AtomicU64,
    enabled: AtomicU64,
    beat_count: AtomicU64,
    cycle_frames: AtomicU64,
    phase_frames: AtomicU64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct PlaybackSpanSnapshot {
    generation: u64,
    start_frame: u64,
    end_frame: u64,
    pending_seek_frame: Option<u64>,
    metronome: PlaybackMetronomeSnapshot,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct PlaybackMetronomeSnapshot {
    enabled: bool,
    beat_count: u16,
    cycle_frames: u64,
    anchor_frame: u64,
    anchor_phase_frames: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct AppliedMetronomeSnapshot {
    revision: u64,
    enabled: bool,
    beat_count: u16,
    cycle_frames: u64,
    phase_frames: u64,
}

impl PlaybackSpanHandle {
    #[cfg(test)]
    pub(crate) fn from_plan(plan: &PlaybackSpanPlan) -> Self {
        Self::from_plan_with_metronome(plan, None)
    }

    pub(crate) fn from_plan_with_metronome(
        plan: &PlaybackSpanPlan,
        metronome: Option<PlaybackMetronomeConfig>,
    ) -> Self {
        let initial = PlaybackSpanSnapshot::from_plan(1, plan, None, metronome);
        let initial_metronome =
            initial.metronome_at(plan.start_frame().saturating_add(plan.seek_offset_frames()));
        Self {
            shared: Arc::new(PlaybackSpanShared {
                pending: PlaybackControlMailbox::new(initial),
                applied_metronome: AppliedMetronomeSlot::new(initial_metronome),
            }),
        }
    }

    pub(crate) fn update_from_plan(
        &self,
        plan: &PlaybackSpanPlan,
        seek_frame: Option<u64>,
        metronome: Option<PlaybackMetronomeConfig>,
    ) {
        self.shared.pending.publish(plan, seek_frame, metronome);
    }

    pub(crate) fn initial_snapshot(&self) -> PlaybackSpanSnapshot {
        self.shared
            .pending
            .try_snapshot()
            .expect("initial playback control transaction must be complete")
    }

    pub(crate) fn latest_snapshot(&self, fallback: PlaybackSpanSnapshot) -> PlaybackSpanSnapshot {
        self.shared.pending.try_snapshot().unwrap_or(fallback)
    }

    pub(crate) fn publish_applied_metronome(
        &self,
        snapshot: PlaybackSpanSnapshot,
        audible_frame: u64,
    ) {
        self.shared
            .applied_metronome
            .publish(snapshot.metronome_at(audible_frame));
    }

    pub(crate) fn applied_metronome(&self) -> AppliedMetronomeSnapshot {
        self.shared.applied_metronome.snapshot()
    }

    pub(crate) fn applied_metronome_revision(&self) -> u64 {
        self.shared.applied_metronome.revision()
    }
}

impl PlaybackControlMailbox {
    fn new(snapshot: PlaybackSpanSnapshot) -> Self {
        Self {
            published: AtomicU64::new(snapshot.generation.saturating_mul(2)),
            slots: [
                PlaybackControlSlot::new(snapshot),
                PlaybackControlSlot::new(snapshot),
            ],
        }
    }

    fn publish(
        &self,
        plan: &PlaybackSpanPlan,
        seek_frame: Option<u64>,
        metronome: Option<PlaybackMetronomeConfig>,
    ) {
        let current = self.published.load(Ordering::Relaxed);
        let generation = (current >> 1).wrapping_add(1);
        let slot_index = ((current & 1) ^ 1) as usize;
        let snapshot = PlaybackSpanSnapshot::from_plan(generation, plan, seek_frame, metronome);

        self.slots[slot_index].store(snapshot);
        self.published.store(
            generation.wrapping_mul(2) | slot_index as u64,
            Ordering::Release,
        );
    }

    fn try_snapshot(&self) -> Option<PlaybackSpanSnapshot> {
        let before = self.published.load(Ordering::Acquire);
        let slot_index = (before & 1) as usize;
        let snapshot = self.slots[slot_index].snapshot(before >> 1);
        let after = self.published.load(Ordering::Acquire);
        (before == after).then_some(snapshot)
    }
}

impl PlaybackControlSlot {
    fn new(snapshot: PlaybackSpanSnapshot) -> Self {
        Self {
            start_frame: AtomicU64::new(snapshot.start_frame),
            end_frame: AtomicU64::new(snapshot.end_frame),
            pending_seek_frame: AtomicU64::new(encode_optional_frame(snapshot.pending_seek_frame)),
            metronome_enabled: AtomicU64::new(u64::from(snapshot.metronome.enabled)),
            metronome_beat_count: AtomicU64::new(u64::from(snapshot.metronome.beat_count)),
            metronome_cycle_frames: AtomicU64::new(snapshot.metronome.cycle_frames),
            metronome_anchor_frame: AtomicU64::new(snapshot.metronome.anchor_frame),
            metronome_anchor_phase_frames: AtomicU64::new(snapshot.metronome.anchor_phase_frames),
        }
    }

    fn store(&self, snapshot: PlaybackSpanSnapshot) {
        self.start_frame
            .store(snapshot.start_frame, Ordering::Relaxed);
        self.end_frame.store(snapshot.end_frame, Ordering::Relaxed);
        self.pending_seek_frame.store(
            encode_optional_frame(snapshot.pending_seek_frame),
            Ordering::Relaxed,
        );
        self.metronome_enabled
            .store(u64::from(snapshot.metronome.enabled), Ordering::Relaxed);
        self.metronome_beat_count
            .store(u64::from(snapshot.metronome.beat_count), Ordering::Relaxed);
        self.metronome_cycle_frames
            .store(snapshot.metronome.cycle_frames, Ordering::Relaxed);
        self.metronome_anchor_frame
            .store(snapshot.metronome.anchor_frame, Ordering::Relaxed);
        self.metronome_anchor_phase_frames
            .store(snapshot.metronome.anchor_phase_frames, Ordering::Relaxed);
    }

    fn snapshot(&self, generation: u64) -> PlaybackSpanSnapshot {
        PlaybackSpanSnapshot {
            generation,
            start_frame: self.start_frame.load(Ordering::Relaxed),
            end_frame: self.end_frame.load(Ordering::Relaxed),
            pending_seek_frame: decode_optional_frame(
                self.pending_seek_frame.load(Ordering::Relaxed),
            ),
            metronome: PlaybackMetronomeSnapshot {
                enabled: self.metronome_enabled.load(Ordering::Relaxed) != 0,
                beat_count: self.metronome_beat_count.load(Ordering::Relaxed) as u16,
                cycle_frames: self.metronome_cycle_frames.load(Ordering::Relaxed),
                anchor_frame: self.metronome_anchor_frame.load(Ordering::Relaxed),
                anchor_phase_frames: self.metronome_anchor_phase_frames.load(Ordering::Relaxed),
            },
        }
    }
}

impl AppliedMetronomeSlot {
    fn new(snapshot: PlaybackMetronomeSnapshot) -> Self {
        Self {
            revision: AtomicU64::new(1),
            enabled: AtomicU64::new(u64::from(snapshot.enabled)),
            beat_count: AtomicU64::new(u64::from(snapshot.beat_count)),
            cycle_frames: AtomicU64::new(snapshot.cycle_frames),
            phase_frames: AtomicU64::new(snapshot.anchor_phase_frames),
        }
    }

    fn publish(&self, snapshot: PlaybackMetronomeSnapshot) {
        self.enabled
            .store(u64::from(snapshot.enabled), Ordering::Relaxed);
        self.beat_count
            .store(u64::from(snapshot.beat_count), Ordering::Relaxed);
        self.cycle_frames
            .store(snapshot.cycle_frames, Ordering::Relaxed);
        self.phase_frames
            .store(snapshot.anchor_phase_frames, Ordering::Relaxed);
        self.revision.fetch_add(1, Ordering::Release);
    }

    fn snapshot(&self) -> AppliedMetronomeSnapshot {
        let revision = self.revision.load(Ordering::Acquire);
        AppliedMetronomeSnapshot {
            revision,
            enabled: self.enabled.load(Ordering::Relaxed) != 0,
            beat_count: self.beat_count.load(Ordering::Relaxed) as u16,
            cycle_frames: self.cycle_frames.load(Ordering::Relaxed),
            phase_frames: self.phase_frames.load(Ordering::Relaxed),
        }
    }

    fn revision(&self) -> u64 {
        self.revision.load(Ordering::Acquire)
    }
}

impl PlaybackSpanSnapshot {
    fn from_plan(
        generation: u64,
        plan: &PlaybackSpanPlan,
        seek_frame: Option<u64>,
        metronome: Option<PlaybackMetronomeConfig>,
    ) -> Self {
        let start_frame = plan.start_frame();
        let end_frame = plan.end_frame().max(start_frame.saturating_add(1));
        let pending_seek_frame =
            seek_frame.map(|frame| clamp_frame_to_span(frame, start_frame, end_frame));
        let anchor_frame = clamp_frame_to_span(
            start_frame.saturating_add(plan.seek_offset_frames()),
            start_frame,
            end_frame,
        );
        let (enabled, beat_count, cycle_frames, anchor_phase_frames) = match metronome {
            Some(config) => {
                let (cycle_frames, phase_frames) =
                    config.cycle(plan.frame_count(), plan.seek_offset_frames());
                (
                    true,
                    config.beat_count(),
                    cycle_frames.max(1),
                    phase_frames % cycle_frames.max(1),
                )
            }
            None => (false, 1, plan.frame_count().max(1), 0),
        };
        Self {
            generation,
            start_frame,
            end_frame,
            pending_seek_frame,
            metronome: PlaybackMetronomeSnapshot {
                enabled,
                beat_count,
                cycle_frames,
                anchor_frame,
                anchor_phase_frames,
            },
        }
    }

    pub(crate) fn generation(self) -> u64 {
        self.generation
    }

    pub(crate) fn start_frame(self) -> u64 {
        self.start_frame
    }

    pub(crate) fn end_frame(self) -> u64 {
        self.end_frame
    }

    pub(crate) fn pending_seek_frame(self) -> Option<u64> {
        self.pending_seek_frame
    }

    pub(crate) fn contains(self, frame: u64) -> bool {
        (self.start_frame..self.end_frame).contains(&frame)
    }

    fn metronome_at(self, audible_frame: u64) -> PlaybackMetronomeSnapshot {
        let mut metronome = self.metronome;
        metronome.anchor_phase_frames = metronome.phase_at(audible_frame);
        metronome.anchor_frame = audible_frame;
        metronome
    }
}

impl PlaybackMetronomeSnapshot {
    fn phase_at(self, frame: u64) -> u64 {
        let cycle_frames = self.cycle_frames.max(1);
        if frame >= self.anchor_frame {
            return add_modulo(
                self.anchor_phase_frames,
                frame.saturating_sub(self.anchor_frame),
                cycle_frames,
            );
        }
        subtract_modulo(
            self.anchor_phase_frames,
            self.anchor_frame.saturating_sub(frame),
            cycle_frames,
        )
    }
}

impl AppliedMetronomeSnapshot {
    pub(crate) fn revision(self) -> u64 {
        self.revision
    }

    pub(crate) fn enabled(self) -> bool {
        self.enabled
    }

    pub(crate) fn beat_count(self) -> u16 {
        self.beat_count
    }

    pub(crate) fn cycle_frames(self) -> u64 {
        self.cycle_frames
    }

    pub(crate) fn phase_frames(self) -> u64 {
        self.phase_frames
    }
}

fn encode_optional_frame(frame: Option<u64>) -> u64 {
    frame.unwrap_or(NO_PENDING_SEEK)
}

fn decode_optional_frame(frame: u64) -> Option<u64> {
    (frame != NO_PENDING_SEEK).then_some(frame)
}

fn clamp_frame_to_span(frame: u64, start_frame: u64, end_frame: u64) -> u64 {
    frame.clamp(start_frame, end_frame.saturating_sub(1))
}

fn add_modulo(value: u64, increment: u64, modulus: u64) -> u64 {
    let modulus = modulus.max(1);
    let value = value % modulus;
    let increment = increment % modulus;
    if increment == 0 {
        return value;
    }
    let distance_to_wrap = modulus - increment;
    if value >= distance_to_wrap {
        value - distance_to_wrap
    } else {
        value + increment
    }
}

fn subtract_modulo(value: u64, decrement: u64, modulus: u64) -> u64 {
    let modulus = modulus.max(1);
    let value = value % modulus;
    let decrement = decrement % modulus;
    if value >= decrement {
        value - decrement
    } else {
        modulus - (decrement - value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::player::{
        PlaybackChannelLayout, PlaybackSeekBehavior, PlaybackSourceIdentity, PlaybackSourceKind,
        PlaybackSpanRequest,
    };

    #[test]
    fn pending_seek_is_clamped_inside_one_coherent_update() {
        let handle = PlaybackSpanHandle::from_plan(&span_plan(0, 10, 0));
        let initial = handle.initial_snapshot();

        handle.update_from_plan(&span_plan(20, 40, 0), Some(90), None);

        let updated = handle.latest_snapshot(initial);
        assert_eq!(updated.start_frame(), 20);
        assert_eq!(updated.end_frame(), 40);
        assert_eq!(updated.pending_seek_frame(), Some(39));
    }

    #[test]
    fn rapid_updates_converge_on_the_latest_complete_transaction() {
        let handle = PlaybackSpanHandle::from_plan(&span_plan(0, 10, 0));
        let mut snapshot = handle.initial_snapshot();

        for index in 1..=10_000_u64 {
            let start = index * 10;
            let metronome =
                PlaybackMetronomeConfig::new((index % 16 + 1) as u16).with_cycle(index + 1, index);
            handle.update_from_plan(
                &span_plan(start, start + 10, index % 10),
                Some(start + 5),
                Some(metronome),
            );
            snapshot = handle.latest_snapshot(snapshot);
        }

        assert_eq!(snapshot.start_frame(), 100_000);
        assert_eq!(snapshot.end_frame(), 100_010);
        assert_eq!(snapshot.pending_seek_frame(), Some(100_005));
        assert_eq!(snapshot.metronome.beat_count, 1);
        assert_eq!(snapshot.metronome.cycle_frames, 10_001);
        assert_eq!(snapshot.metronome.anchor_phase_frames, 10_000);
    }

    #[test]
    fn an_in_progress_inactive_slot_write_does_not_disturb_audio_reads() {
        let handle = PlaybackSpanHandle::from_plan(&span_plan(0, 10, 0));
        let fallback = handle.initial_snapshot();
        let published = handle.shared.pending.published.load(Ordering::Relaxed);
        let inactive_slot = ((published & 1) ^ 1) as usize;
        let staged = PlaybackSpanSnapshot::from_plan(
            (published >> 1) + 1,
            &span_plan(20, 40, 0),
            Some(30),
            Some(PlaybackMetronomeConfig::new(7).with_cycle(20, 10)),
        );
        handle.shared.pending.slots[inactive_slot].store(staged);

        assert_eq!(handle.latest_snapshot(fallback), fallback);
    }

    #[test]
    fn metronome_phase_math_handles_the_full_u64_range() {
        let metronome = PlaybackMetronomeSnapshot {
            enabled: true,
            beat_count: 4,
            cycle_frames: u64::MAX,
            anchor_frame: 1,
            anchor_phase_frames: u64::MAX - 1,
        };

        assert_eq!(metronome.phase_at(2), 0);
        assert_eq!(metronome.phase_at(0), u64::MAX - 2);
    }

    #[test]
    fn live_control_audio_path_has_no_blocking_synchronization_tokens() {
        let sources = [
            include_str!("playback_span_handle.rs"),
            include_str!("playback/span_samples.rs"),
            include_str!("metronome.rs"),
        ];
        let blocking_tokens = [
            concat!("Rw", "Lock"),
            concat!("Mu", "tex"),
            concat!("Cond", "var"),
            concat!(".lo", "ck("),
            concat!(".re", "ad("),
            concat!(".wr", "ite("),
        ];

        for token in blocking_tokens {
            assert!(
                sources.iter().all(|source| !source.contains(token)),
                "live playback control path must not contain blocking token {token}"
            );
        }
    }

    fn span_plan(start_frame: u64, end_frame: u64, offset_frame: u64) -> PlaybackSpanPlan {
        let duration = end_frame.max(1) as f32 / 1_000.0;
        PlaybackSpanPlan::new(
            PlaybackSourceIdentity::new(PlaybackSourceKind::Bytes, None),
            PlaybackChannelLayout::new(1, 1_000).expect("layout"),
            PlaybackSpanRequest::new(
                start_frame as f32 / 1_000.0,
                end_frame as f32 / 1_000.0,
                duration,
                true,
                PlaybackSeekBehavior::FrameOffset(offset_frame),
            ),
        )
        .expect("span plan")
    }
}
