//! Deterministic priority, fairness, and resource-budget policy for native source work.
#![cfg_attr(test, allow(dead_code))]

use std::collections::{BTreeMap, BTreeSet};
use wavecrate::sample_sources::readiness::{ReadinessStage, ReadinessTarget};

/// Execution lane used to cap work with similar resource pressure.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum ProcessingLane {
    Scan,
    Hashing,
    FeatureAnalysis,
    Embedding,
    Finalization,
    Cleanup,
}

impl ProcessingLane {
    #[cfg(test)]
    const ALL: [Self; 6] = [
        Self::Scan,
        Self::Hashing,
        Self::FeatureAnalysis,
        Self::Embedding,
        Self::Finalization,
        Self::Cleanup,
    ];

    pub(crate) fn for_readiness_stage(stage: ReadinessStage) -> Self {
        match stage {
            ReadinessStage::IndexedIdentity => Self::Hashing,
            ReadinessStage::AnalysisFeatures => Self::FeatureAnalysis,
            ReadinessStage::EmbeddingAspects => Self::Embedding,
            ReadinessStage::SimilarityLayout => Self::Finalization,
        }
    }

    fn demand(self) -> ResourceUse {
        match self {
            Self::Scan | Self::Cleanup => ResourceUse::new(0, 1, 1),
            Self::Hashing => ResourceUse::new(1, 1, 1),
            Self::FeatureAnalysis | Self::Embedding => ResourceUse::new(1, 1, 0),
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
                (ProcessingLane::FeatureAnalysis, cpu.min(2)),
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

    #[cfg(test)]
    pub(crate) fn active_sources(&self) -> BTreeSet<String> {
        self.by_source.keys().cloned().collect()
    }

    #[cfg(test)]
    pub(crate) fn current_global(&self) -> ResourceUse {
        self.global
    }

    pub(crate) fn execution_worker_limit(&self) -> usize {
        self.limits
            .lane_limit(ProcessingLane::FeatureAnalysis)
            .max(1)
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
}

/// Source-queued scheduler that chooses a source fairly, then drains its runnable work before
/// moving to another source.
#[derive(Debug, Default)]
pub(crate) struct FairScheduler {
    virtual_finish: BTreeMap<String, u64>,
    active_source: Option<String>,
}

impl FairScheduler {
    pub(crate) fn choose(
        &mut self,
        candidates: &[WorkCandidate],
        priority: &PriorityContext,
        budgets: &BudgetTracker,
    ) -> Option<usize> {
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
        if let Some(active_source) = self.active_source.as_deref()
            && let Some(index) = best_by_source.remove(active_source)
        {
            return Some(index);
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
        // Visible source ownership remains stable while bounded work from another source uses
        // otherwise-idle execution capacity. Ownership itself is released only after a fresh
        // convergence/retry decision in the coordinator.
        if self.active_source.is_none() {
            self.active_source = Some(selected.2.to_string());
        }
        self.virtual_finish
            .insert(selected.2.to_string(), selected.0);
        self.normalize_virtual_time();
        Some(selected.3)
    }

    pub(crate) fn active_source(&self) -> Option<&str> {
        self.active_source.as_deref()
    }

    pub(crate) fn release_active_source(&mut self) -> Option<String> {
        self.active_source.take()
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
        ReadinessStage::IndexedIdentity => 0,
        ReadinessStage::AnalysisFeatures => 1,
        ReadinessStage::EmbeddingAspects => 2,
        ReadinessStage::SimilarityLayout => 3,
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
    fn selected_source_is_drained_before_the_queue_advances() {
        let mut scheduler = FairScheduler::default();
        let budgets = BudgetTracker::new(ProcessingBudgets::for_tests(
            ResourceUse::new(10, 10, 10),
            ResourceUse::new(10, 10, 10),
            10,
        ));
        let mut candidates = vec![
            candidate("active", "now", ReadinessStage::IndexedIdentity),
            candidate("active", "next", ReadinessStage::AnalysisFeatures),
            candidate("backlog", "old", ReadinessStage::AnalysisFeatures),
        ];
        let priority = PriorityContext {
            immediate: [PriorityKey::new("active", "now")].into_iter().collect(),
            ..PriorityContext::default()
        };
        let mut chosen = Vec::new();
        while !candidates.is_empty() {
            let Some(index) = scheduler.choose(&candidates, &priority, &budgets) else {
                let active_source = scheduler
                    .active_source()
                    .expect("a blocked choice retains source ownership");
                assert!(
                    candidates
                        .iter()
                        .all(|candidate| candidate.source_id != active_source),
                    "the active source must be drained before ownership is released"
                );
                scheduler.release_active_source();
                continue;
            };
            let candidate = candidates.remove(index);
            chosen.push((candidate.source_id, candidate.scope_id));
        }

        assert_eq!(
            chosen,
            vec![
                (String::from("active"), String::from("now")),
                (String::from("active"), String::from("next")),
                (String::from("backlog"), String::from("old")),
            ]
        );
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
            candidate("source", "identity", ReadinessStage::IndexedIdentity),
        ];
        let priority = PriorityContext {
            current_folder: Some(("source".to_string(), "folder/".to_string())),
            ..PriorityContext::default()
        };
        let index = scheduler.choose(&candidates, &priority, &budgets).unwrap();
        assert_eq!(candidates[index].scope_id, "identity");
    }

    #[test]
    fn exact_identity_is_drained_before_content_derived_stages() {
        let mut scheduler = FairScheduler::default();
        let budgets = BudgetTracker::new(ProcessingBudgets::for_tests(
            ResourceUse::new(10, 10, 10),
            ResourceUse::new(10, 10, 10),
            10,
        ));
        let candidates = [
            candidate("source", "sample", ReadinessStage::AnalysisFeatures),
            candidate("source", "sample", ReadinessStage::IndexedIdentity),
        ];

        let index = scheduler
            .choose(&candidates, &PriorityContext::default(), &budgets)
            .unwrap();

        assert_eq!(candidates[index].stage_rank, 0);
    }

    #[test]
    fn budgets_cap_global_per_source_and_lane_usage() {
        let limits =
            ProcessingBudgets::for_tests(ResourceUse::new(2, 2, 2), ResourceUse::new(1, 1, 1), 1);
        let mut tracker = BudgetTracker::new(limits);
        let first = tracker
            .try_acquire("one", ProcessingLane::Hashing)
            .expect("first permit");
        assert!(
            tracker
                .try_acquire("one", ProcessingLane::FeatureAnalysis)
                .is_none()
        );
        assert!(
            tracker
                .try_acquire("two", ProcessingLane::Hashing)
                .is_none()
        );
        let second = tracker
            .try_acquire("two", ProcessingLane::FeatureAnalysis)
            .expect("independent lane");
        assert_eq!(tracker.current_global(), ResourceUse::new(2, 2, 1));
        tracker.release(first);
        tracker.release(second);
        assert_eq!(tracker.current_global(), ResourceUse::default());
    }

    #[test]
    fn blocked_active_source_admits_secondary_work_without_switching_visible_owner() {
        let mut scheduler = FairScheduler::default();
        let limits =
            ProcessingBudgets::for_tests(ResourceUse::new(2, 2, 2), ResourceUse::new(1, 1, 1), 2);
        let mut budgets = BudgetTracker::new(limits);
        let candidates = [
            candidate("active", "first", ReadinessStage::AnalysisFeatures),
            candidate("backlog", "other", ReadinessStage::AnalysisFeatures),
        ];
        let chosen = scheduler
            .choose(&candidates, &PriorityContext::default(), &budgets)
            .expect("select active source");
        assert_eq!(candidates[chosen].source_id, "active");
        let permit = budgets
            .try_acquire("active", ProcessingLane::FeatureAnalysis)
            .expect("occupy active-source budget");

        assert_eq!(
            scheduler.choose(&candidates, &PriorityContext::default(), &budgets),
            Some(1),
            "idle global capacity should admit an independent source"
        );
        assert_eq!(scheduler.active_source(), Some("active"));
        budgets.release(permit);
    }

    #[test]
    fn secondary_admission_preserves_visible_owner_until_explicit_release() {
        let mut scheduler = FairScheduler::default();
        let budgets = BudgetTracker::new(ProcessingBudgets::for_tests(
            ResourceUse::new(2, 2, 2),
            ResourceUse::new(1, 1, 1),
            2,
        ));
        let active = [candidate(
            "active",
            "first",
            ReadinessStage::AnalysisFeatures,
        )];
        assert_eq!(
            scheduler.choose(&active, &PriorityContext::default(), &budgets),
            Some(0)
        );
        let backlog = [candidate(
            "backlog",
            "other",
            ReadinessStage::AnalysisFeatures,
        )];
        assert_eq!(
            scheduler.choose(&backlog, &PriorityContext::default(), &budgets),
            Some(0),
            "secondary work should use capacity without taking visible ownership"
        );
        assert_eq!(scheduler.active_source(), Some("active"));
        assert_eq!(scheduler.release_active_source().as_deref(), Some("active"));
        assert_eq!(
            scheduler.choose(&backlog, &PriorityContext::default(), &budgets),
            Some(0)
        );
        assert_eq!(scheduler.active_source(), Some("backlog"));
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
