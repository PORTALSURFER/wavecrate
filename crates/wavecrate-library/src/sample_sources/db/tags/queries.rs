use std::path::Path;

use super::super::util::{map_sql_error, normalize_relative_path};
use super::super::{SourceDatabase, SourceDbError, SourceTag, SourceTagUsage};
use super::identity::normalize_tag_identity;

const DEFAULT_TAG_LIMIT: usize = 32;

impl SourceDatabase {
    /// Return the most-used persisted normal tags, ordered by usage then label.
    pub fn most_used_tags(&self, limit: usize) -> Result<Vec<SourceTagUsage>, SourceDbError> {
        let limit = query_limit(limit);
        let mut stmt = self
            .connection
            .prepare(
                "SELECT st.id, st.display_label, st.normalized_text, COUNT(wft.path) AS usage_count
                 FROM source_tags st
                 JOIN wav_file_tags wft ON wft.tag_id = st.id
                 GROUP BY st.id, st.display_label, st.normalized_text
                 ORDER BY usage_count DESC,
                          st.display_label COLLATE NOCASE ASC,
                          st.normalized_text ASC
                 LIMIT ?1",
            )
            .map_err(map_sql_error)?;
        collect_tag_usage(
            stmt.query_map([limit], tag_usage_from_row)
                .map_err(map_sql_error)?,
        )
    }

    /// Search persisted normal tags by normalized text. Empty input falls back to most-used tags.
    pub fn search_tags(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<Vec<SourceTagUsage>, SourceDbError> {
        let identity = match normalize_tag_identity(query) {
            Ok(identity) => identity,
            Err(SourceDbError::EmptyTagLabel) => return self.most_used_tags(limit),
            Err(err) => return Err(err),
        };
        let limit = query_limit(limit);
        let mut stmt = self
            .connection
            .prepare(
                "SELECT st.id, st.display_label, st.normalized_text, COUNT(wft.path) AS usage_count
                 FROM source_tags st
                 LEFT JOIN wav_file_tags wft ON wft.tag_id = st.id
                 GROUP BY st.id, st.display_label, st.normalized_text
                 ORDER BY usage_count DESC,
                          st.display_label COLLATE NOCASE ASC,
                          st.normalized_text ASC",
            )
            .map_err(map_sql_error)?;
        let mut matches = collect_tag_usage(
            stmt.query_map([], tag_usage_from_row)
                .map_err(map_sql_error)?,
        )?
        .into_iter()
        .filter_map(|usage| {
            tag_match_rank(&usage.tag.normalized_text, &identity.normalized_text)
                .map(|rank| (rank, usage))
        })
        .collect::<Vec<_>>();

        matches.sort_by(|(left_rank, left), (right_rank, right)| {
            left_rank
                .cmp(right_rank)
                .then_with(|| right.assignment_count.cmp(&left.assignment_count))
                .then_with(|| {
                    left.tag
                        .display_label
                        .to_ascii_lowercase()
                        .cmp(&right.tag.display_label.to_ascii_lowercase())
                })
                .then_with(|| left.tag.normalized_text.cmp(&right.tag.normalized_text))
        });
        Ok(matches
            .into_iter()
            .take(limit as usize)
            .map(|(_, usage)| usage)
            .collect())
    }

    /// List normal tags assigned to one wav path.
    pub fn tags_for_path(&self, relative_path: &Path) -> Result<Vec<SourceTag>, SourceDbError> {
        let path = normalize_relative_path(relative_path)?;
        let mut stmt = self
            .connection
            .prepare(
                "SELECT st.id, st.display_label, st.normalized_text
                 FROM source_tags st
                 JOIN wav_file_tags wft ON wft.tag_id = st.id
                 WHERE wft.path = ?1
                 ORDER BY st.display_label COLLATE NOCASE ASC, st.normalized_text ASC",
            )
            .map_err(map_sql_error)?;
        let rows = stmt
            .query_map([path], |row| {
                Ok(SourceTag {
                    id: row.get(0)?,
                    display_label: row.get(1)?,
                    normalized_text: row.get(2)?,
                })
            })
            .map_err(map_sql_error)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(map_sql_error)
    }

    /// List user-facing normal tag labels assigned to one wav path.
    pub fn tag_labels_for_path(&self, relative_path: &Path) -> Result<Vec<String>, SourceDbError> {
        Ok(self
            .tags_for_path(relative_path)?
            .into_iter()
            .map(|tag| tag.display_label)
            .collect())
    }
}

fn query_limit(limit: usize) -> i64 {
    limit.clamp(1, DEFAULT_TAG_LIMIT) as i64
}

fn tag_usage_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<SourceTagUsage> {
    let assignment_count: i64 = row.get(3)?;
    Ok(SourceTagUsage {
        tag: SourceTag {
            id: row.get(0)?,
            display_label: row.get(1)?,
            normalized_text: row.get(2)?,
        },
        assignment_count: assignment_count.max(0) as u64,
    })
}

fn collect_tag_usage(
    rows: rusqlite::MappedRows<
        '_,
        impl FnMut(&rusqlite::Row<'_>) -> rusqlite::Result<SourceTagUsage>,
    >,
) -> Result<Vec<SourceTagUsage>, SourceDbError> {
    rows.collect::<Result<Vec<_>, _>>().map_err(map_sql_error)
}

fn tag_match_rank(candidate: &str, query: &str) -> Option<u8> {
    if candidate == query {
        return Some(0);
    }
    if candidate.contains(query) {
        return Some(1);
    }
    fuzzy_subsequence_match(candidate, query).then_some(2)
}

fn fuzzy_subsequence_match(candidate: &str, query: &str) -> bool {
    let mut chars = candidate.chars();
    query
        .chars()
        .all(|query_char| chars.any(|candidate_char| candidate_char == query_char))
}
