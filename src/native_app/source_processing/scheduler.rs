//! Deterministic priority, fairness, and resource-budget policy for native source work.
#![cfg_attr(test, allow(dead_code))]

use std::collections::{BTreeMap, BTreeSet};
use wavecrate::sample_sources::readiness::{ReadinessStage, ReadinessTarget};

/// Execution lane used to cap work with similar resource pressure.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum ProcessingLane {
    Scan,
    Hashing,
    DecodeSummary,
    FeatureAnalysis,
    Embedding,
    Finalization,
    Cleanup,
}

impl ProcessingLane {
    #[cfg(test)]
    const ALL: [Self; 7] = [
        Self::Scan,
        Self::Hashing,
        Self::DecodeSummary,
        Self::FeatureAnalysis,
        Self::Embedding,
        Self::Finalization,
        Self::Cleanup,
    ];

    pub(crate) fn for_readiness_stage(stage: ReadinessStage) -> Self {
        match stage {
            ReadinessStage::IndexedIdentity => Self::Hashing,
            ReadinessStage::PlaybackSummary => Self::DecodeSummary,
            ReadinessStage::AnalysisFeatures => Self::FeatureAnalysis,
            ReadinessStage::EmbeddingAspects => Self::Embedding,
            ReadinessStage::SimilarityLayout => Self::Finalization,
        }
    }

    fn demand(self) -> ResourceUse {
        match self {
            Self::Scan | Self::Cleanup => ResourceUse::new(0, 1, 1),
            Self::Hashing | Self::DecodeSummary => ResourceUse::new(1, 1, 1),
            Self::FeatureAnalysis | Self::Embedding => ResourceUse::new(1, 1, 1),
            Self::Finalization => ResourceUse::new(1, 1, 1),
        }
    }
}

/// CPU, IO, and SQLite writer capacity consumed by one work item.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct ResourceUse {
    pub(crate) cpu: usize,
    pub(crate) io: usize,
    pub(crate) database: usize,
}

impl ResourceUse {
    const fn new(cpu: usize, io: usize, database: usize) -> Self {
        Self { cpu, io, database }
    }

    fn fits_within(self, limit: Self) -> bool {
        self.cpu <= limit.cpu && self.io <= limit.io && self.database <= limit.database
    }

    fn add(self, other: Self) -> Self {
        Self::new(
            self.cpu.saturating_add(other.cpu),
            self.io.saturating_add(other.io),
            self.database.saturating_add(other.database),
        )
    }

    fn subtract(self, other: Self) -> Self {
        Self::new(
            self.cpu.saturating_sub(other.cpu),
            self.io.saturating_sub(other.io),
            self.database.saturating_sub(other.database),
        )
    }
}

/// Explicit global, per-source, and per-lane concurrency ceilings.
#[derive(Clone, Debug)]
pub(crate) struct ProcessingBudgets {
    pub(crate) global: ResourceUse,
    pub(crate) per_source: ResourceUse,
    lane_limits: BTreeMap<ProcessingLane, usize>,
}

impl ProcessingBudgets {
    #[cfg(test)]
    pub(crate) fn for_tests(
        global: ResourceUse,
        per_source: ResourceUse,
        lane_limit: usize,
    ) -> Self {
        Self {
            global,
            per_source,
            lane_limits: ProcessingLane::ALL
                .into_iter()
                .map(|lane| (lane, lane_limit))
                .collect(),
        }
    }

    fn lane_limit(&self, lane: ProcessingLane) -> usize {
        self.lane_limits.get(&lane).copied().unwrap_or(0)
    }
}

impl Default for ProcessingBudgets {
    fn default() -> Self {
        let parallelism = std::thread::available_parallelism()
            .map(std::num::NonZeroUsize::get)
            .unwrap_or(2);
        let cpu = parallelism.saturating_sub(2).max(1);
        Self {
            global: ResourceUse::new(cpu, 2, 1),
            per_source: ResourceUse::new(1, 1, 1),
            lane_limits: [
                (ProcessingLane::Scan, 1),
                (ProcessingLane::Hashing, 1),
                (ProcessingLane::DecodeSummary, cpu.min(2)),
                (ProcessingLane::FeatureAnalysis, cpu),
                (ProcessingLane::Embedding, 1),
                (ProcessingLane::Finalization, 1),
                (ProcessingLane::Cleanup, 1),
            ]
            .into_iter()
            .collect(),
        }
    }
}

/// Capacity reservation retained until one owned worker reports completion.
#[derive(Clone, Debug)]
pub(crate) struct BudgetPermit {
    source_id: String,
    lane: ProcessingLane,
    demand: ResourceUse,
}

impl BudgetPermit {
    pub(crate) fn source_id(&self) -> &str {
        &self.source_id
    }
}

/// Mutable accounting for all in-flight supervisor work.
#[derive(Debug)]
pub(crate) struct BudgetTracker {
    limits: ProcessingBudgets,
    global: ResourceUse,
    by_source: BTreeMap<String, ResourceUse>,
    by_lane: BTreeMap<ProcessingLane, usize>,
}

impl BudgetTracker {
    pub(crate) fn new(limits: ProcessingBudgets) -> Self {
        Self {
            limits,
            global: ResourceUse::default(),
            by_source: BTreeMap::new(),
            by_lane: BTreeMap::new(),
        }
    }

    pub(crate) fn can_acquire(&self, source_id: &str, lane: ProcessingLane) -> bool {
        let demand = lane.demand();
        let global = self.global.add(demand);
        let source = self
            .by_source
            .get(source_id)
            .copied()
            .unwrap_or_default()
            .add(demand);
        let lane_count = self.by_lane.get(&lane).copied().unwrap_or(0);
        global.fits_within(self.limits.global)
            && source.fits_within(self.limits.per_source)
            && lane_count < self.limits.lane_limit(lane)
    }

    pub(crate) fn try_acquire(
        &mut self,
        source_id: &str,
        lane: ProcessingLane,
    ) -> Option<BudgetPermit> {
        if !self.can_acquire(source_id, lane) {
            return None;
        }
        let demand = lane.demand();
        self.global = self.global.add(demand);
        let source = self.by_source.entry(source_id.to_string()).or_default();
        *source = source.add(demand);
        *self.by_lane.entry(lane).or_default() += 1;
        Some(BudgetPermit {
            source_id: source_id.to_string(),
            lane,
            demand,
        })
    }

    pub(crate) fn release(&mut self, permit: BudgetPermit) {
        self.global = self.global.subtract(permit.demand);
        if let Some(source) = self.by_source.get_mut(&permit.source_id) {
            *source = source.subtract(permit.demand);
            if *source == ResourceUse::default() {
                self.by_source.remove(&permit.source_id);
            }
        }
        if let Some(count) = self.by_lane.get_mut(&permit.lane) {
            *count = count.saturating_sub(1);
            if *count == 0 {
                self.by_lane.remove(&permit.lane);
            }
        }
    }

    pub(crate) fn active_sources(&self) -> BTreeSet<String> {
        self.by_source.keys().cloned().collect()
    }

    #[cfg(test)]
    pub(crate) fn current_global(&self) -> ResourceUse {
        self.global
    }
}

/// Stable key used by interaction context to raise already-authoritative work.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct PriorityKey {
    pub(crate) source_id: String,
    pub(crate) scope_id: String,
}

impl PriorityKey {
    #[cfg(test)]
    fn new(source_id: &str, scope_id: &str) -> Self {
        Self {
            source_id: source_id.to_string(),
            scope_id: scope_id.to_string(),
        }
    }
}

/// Session-local priority hints. They never create durable work.
#[derive(Clone, Debug, Default)]
pub(crate) struct PriorityContext {
    pub(crate) immediate: BTreeSet<PriorityKey>,
    pub(crate) visible: BTreeSet<PriorityKey>,
    pub(crate) immediate_paths: BTreeSet<(String, String)>,
    pub(crate) visible_paths: BTreeSet<(String, String)>,
    pub(crate) selected_source: Option<String>,
    pub(crate) current_folder: Option<(String, String)>,
}

/// One durable or discovered source-processing unit eligible for scheduling.
#[derive(Clone, Debug)]
pub(crate) struct WorkCandidate {
    pub(crate) source_id: String,
    pub(crate) scope_id: String,
    pub(crate) relative_path: Option<String>,
    pub(crate) lane: ProcessingLane,
    pub(crate) stage_rank: u8,
    pub(crate) enqueued_at: i64,
}

impl WorkCandidate {
    pub(crate) fn readiness(target: &ReadinessTarget, enqueued_at: i64) -> Self {
        Self {
            source_id: target.source_id.clone(),
            scope_id: target.scope_id.clone(),
            relative_path: target.relative_path.clone(),
            lane: ProcessingLane::for_readiness_stage(target.stage),
            stage_rank: stage_rank(target.stage),
            enqueued_at,
        }
    }

    pub(crate) fn source(
        source_id: impl Into<String>,
        lane: ProcessingLane,
        stage_rank: u8,
        enqueued_at: i64,
    ) -> Self {
        let source_id = source_id.into();
        Self {
            scope_id: source_id.clone(),
            source_id,
            relative_path: None,
            lane,
            stage_rank,
            enqueued_at,
        }
    }

    pub(crate) fn file(
        source_id: impl Into<String>,
        relative_path: impl Into<String>,
        lane: ProcessingLane,
        stage_rank: u8,
        enqueued_at: i64,
    ) -> Self {
        let source_id = source_id.into();
        let relative_path = relative_path.into();
        Self {
            source_id,
            scope_id: relative_path.clone(),
            relative_path: Some(relative_path),
            lane,
            stage_rank,
            enqueued_at,
        }
    }
}

/// Weighted-fair scheduler that preserves interaction priority without starving other sources.
#[derive(Debug, Default)]
pub(crate) struct FairScheduler {
    virtual_finish: BTreeMap<String, u64>,
    paused: bool,
}

impl FairScheduler {
    pub(crate) fn set_paused(&mut self, paused: bool) {
        self.paused = paused;
    }

    pub(crate) fn choose(
        &mut self,
        candidates: &[WorkCandidate],
        priority: &PriorityContext,
        budgets: &BudgetTracker,
    ) -> Option<usize> {
        if self.paused {
            return None;
        }
        let mut best_by_source = BTreeMap::<&str, usize>::new();
        for (index, candidate) in candidates.iter().enumerate() {
            if !budgets.can_acquire(&candidate.source_id, candidate.lane) {
                continue;
            }
            best_by_source
                .entry(candidate.source_id.as_str())
                .and_modify(|current| {
                    if candidate_order(candidate, &candidates[*current], priority).is_lt() {
                        *current = index;
                    }
                })
                .or_insert(index);
        }
        let selected = best_by_source
            .into_iter()
            .map(|(source_id, index)| {
                let weight = priority_weight(&candidates[index], priority);
                let increment = 48 / weight;
                let finish = self
                    .virtual_finish
                    .get(source_id)
                    .copied()
                    .unwrap_or_default()
                    .saturating_add(increment);
                (finish, std::cmp::Reverse(weight), source_id, index)
            })
            .min()?;
        self.virtual_finish
            .insert(selected.2.to_string(), selected.0);
        self.normalize_virtual_time();
        Some(selected.3)
    }

    fn normalize_virtual_time(&mut self) {
        let Some(minimum) = self.virtual_finish.values().copied().min() else {
            return;
        };
        if minimum < 1_000_000 {
            return;
        }
        for finish in self.virtual_finish.values_mut() {
            *finish = finish.saturating_sub(minimum);
        }
    }
}

fn candidate_order(
    left: &WorkCandidate,
    right: &WorkCandidate,
    priority: &PriorityContext,
) -> std::cmp::Ordering {
    std::cmp::Reverse(priority_weight(left, priority))
        .cmp(&std::cmp::Reverse(priority_weight(right, priority)))
        .then_with(|| left.stage_rank.cmp(&right.stage_rank))
        .then_with(|| left.enqueued_at.cmp(&right.enqueued_at))
        .then_with(|| left.scope_id.cmp(&right.scope_id))
}

fn priority_weight(candidate: &WorkCandidate, context: &PriorityContext) -> u64 {
    let key = PriorityKey {
        source_id: candidate.source_id.clone(),
        scope_id: candidate.scope_id.clone(),
    };
    if context.immediate.contains(&key) {
        return 48;
    }
    if candidate.relative_path.as_ref().is_some_and(|path| {
        context
            .immediate_paths
            .contains(&(candidate.source_id.clone(), path.clone()))
    }) {
        return 48;
    }
    if context.visible.contains(&key) {
        return 12;
    }
    if candidate.relative_path.as_ref().is_some_and(|path| {
        context
            .visible_paths
            .contains(&(candidate.source_id.clone(), path.clone()))
    }) {
        return 12;
    }
    if context
        .current_folder
        .as_ref()
        .is_some_and(|(source, folder)| {
            source == &candidate.source_id
                && candidate
                    .relative_path
                    .as_deref()
                    .is_some_and(|path| path.starts_with(folder))
        })
    {
        return 6;
    }
    if context.selected_source.as_deref() == Some(candidate.source_id.as_str()) {
        return 3;
    }
    1
}

fn stage_rank(stage: ReadinessStage) -> u8 {
    match stage {
        ReadinessStage::PlaybackSummary => 0,
        ReadinessStage::IndexedIdentity => 1,
        ReadinessStage::AnalysisFeatures => 2,
        ReadinessStage::EmbeddingAspects => 3,
        ReadinessStage::SimilarityLayout => 4,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn target(source: &str, scope: &str, stage: ReadinessStage) -> ReadinessTarget {
        if stage == ReadinessStage::SimilarityLayout {
            ReadinessTarget::source(source, stage, "v1", 1, "members-v1")
        } else {
            ReadinessTarget::file(
                source,
                scope,
                format!("folder/{scope}.wav"),
                stage,
                "v1",
                1,
                format!("content-{scope}"),
            )
        }
    }

    fn candidate(source: &str, scope: &str, stage: ReadinessStage) -> WorkCandidate {
        WorkCandidate::readiness(&target(source, scope, stage), 10)
    }

    #[test]
    fn immediate_and_visible_work_lead_without_starving_backlog_sources() {
        let mut scheduler = FairScheduler::default();
        let budgets = BudgetTracker::new(ProcessingBudgets::for_tests(
            ResourceUse::new(10, 10, 10),
            ResourceUse::new(10, 10, 10),
            10,
        ));
        let candidates = [
            candidate("active", "now", ReadinessStage::PlaybackSummary),
            candidate("backlog", "old", ReadinessStage::AnalysisFeatures),
        ];
        let priority = PriorityContext {
            immediate: [PriorityKey::new("active", "now")].into_iter().collect(),
            ..PriorityContext::default()
        };
        let mut chosen_sources = Vec::new();
        for _ in 0..60 {
            let index = scheduler.choose(&candidates, &priority, &budgets).unwrap();
            chosen_sources.push(candidates[index].source_id.as_str());
        }
        assert_eq!(chosen_sources[0], "active");
        assert!(
            chosen_sources[..10]
                .iter()
                .all(|source| *source == "active")
        );
        assert!(chosen_sources.contains(&"backlog"));
    }

    #[test]
    fn current_folder_and_stage_order_prioritize_interactive_readiness() {
        let mut scheduler = FairScheduler::default();
        let budgets = BudgetTracker::new(ProcessingBudgets::for_tests(
            ResourceUse::new(10, 10, 10),
            ResourceUse::new(10, 10, 10),
            10,
        ));
        let candidates = [
            candidate("source", "analysis", ReadinessStage::AnalysisFeatures),
            candidate("source", "playback", ReadinessStage::PlaybackSummary),
        ];
        let priority = PriorityContext {
            current_folder: Some(("source".to_string(), "folder/".to_string())),
            ..PriorityContext::default()
        };
        let index = scheduler.choose(&candidates, &priority, &budgets).unwrap();
        assert_eq!(candidates[index].scope_id, "playback");
    }

    #[test]
    fn budgets_cap_global_per_source_and_lane_usage() {
        let limits =
            ProcessingBudgets::for_tests(ResourceUse::new(2, 2, 2), ResourceUse::new(1, 1, 1), 1);
        let mut tracker = BudgetTracker::new(limits);
        let first = tracker
            .try_acquire("one", ProcessingLane::DecodeSummary)
            .expect("first permit");
        assert!(
            tracker
                .try_acquire("one", ProcessingLane::FeatureAnalysis)
                .is_none()
        );
        assert!(
            tracker
                .try_acquire("two", ProcessingLane::DecodeSummary)
                .is_none()
        );
        let second = tracker
            .try_acquire("two", ProcessingLane::FeatureAnalysis)
            .expect("independent lane");
        assert_eq!(tracker.current_global(), ResourceUse::new(2, 2, 2));
        tracker.release(first);
        tracker.release(second);
        assert_eq!(tracker.current_global(), ResourceUse::default());
    }

    #[test]
    fn pause_retains_the_same_pending_backlog_for_resume() {
        let mut scheduler = FairScheduler::default();
        let budgets = BudgetTracker::new(ProcessingBudgets::for_tests(
            ResourceUse::new(2, 2, 2),
            ResourceUse::new(1, 1, 1),
            2,
        ));
        let candidates = [candidate(
            "source",
            "pending",
            ReadinessStage::AnalysisFeatures,
        )];
        scheduler.set_paused(true);
        assert!(
            scheduler
                .choose(&candidates, &PriorityContext::default(), &budgets)
                .is_none()
        );
        scheduler.set_paused(false);
        assert_eq!(
            scheduler.choose(&candidates, &PriorityContext::default(), &budgets),
            Some(0)
        );
    }

    #[test]
    fn stress_accounting_never_exceeds_resource_ceilings() {
        let limits =
            ProcessingBudgets::for_tests(ResourceUse::new(4, 3, 2), ResourceUse::new(2, 2, 1), 2);
        let mut tracker = BudgetTracker::new(limits.clone());
        let mut permits = Vec::new();
        for index in 0..1_000 {
            let source = format!("source-{}", index % 8);
            let lane = ProcessingLane::ALL[index % ProcessingLane::ALL.len()];
            if let Some(permit) = tracker.try_acquire(&source, lane) {
                assert!(tracker.current_global().fits_within(limits.global));
                permits.push(permit);
            }
        }
        for permit in permits {
            tracker.release(permit);
        }
        assert_eq!(tracker.current_global(), ResourceUse::default());
    }
}
