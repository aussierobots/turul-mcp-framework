//! Process-unique names for storage resources: the SQLite shared-cache URI,
//! and the scratch Postgres databases and DynamoDB tables the conformance
//! tests create.
//!
//! A timestamp alone is not unique: two calls in the same clock tick, or in
//! two test processes started together, return the same value. The name is
//! `{prefix}{unix_nanos}_{pid}_{sequence}`. The timestamp comes first so a
//! reaper can read a resource's age back with [`created_at_nanos`].

/// SQLite uses it in production; the Postgres and DynamoDB tests use it for scratch resources.
#[cfg(any(feature = "sqlite", test))]
pub(crate) fn unique_name(prefix: &str) -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static SEQUENCE: AtomicU64 = AtomicU64::new(0);
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let sequence = SEQUENCE.fetch_add(1, Ordering::Relaxed);
    format!("{prefix}{nanos}_{}_{sequence}", std::process::id())
}

/// The creation time, in Unix nanoseconds, of a name [`unique_name`] built
/// with `prefix`, or of an older `{prefix}{unix_nanos}` name. `None` when
/// `name` is neither.
#[cfg(test)]
pub(crate) fn created_at_nanos(name: &str, prefix: &str) -> Option<u128> {
    let rest = name.strip_prefix(prefix)?;
    let nanos = rest.split_once('_').map_or(rest, |(nanos, _)| nanos);
    nanos.parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Names drawn concurrently in one process never repeat, even within one clock tick.
    #[test]
    fn concurrent_names_are_distinct() {
        let threads: Vec<_> = (0..8)
            .map(|_| {
                std::thread::spawn(|| (0..2000).map(|_| unique_name("t_")).collect::<Vec<_>>())
            })
            .collect();
        let mut all: Vec<String> = threads
            .into_iter()
            .flat_map(|t| t.join().unwrap())
            .collect();
        let total = all.len();
        all.sort();
        all.dedup();
        assert_eq!(all.len(), total, "duplicate names");
    }

    /// The name carries the process id, so two processes started together differ.
    #[test]
    fn name_carries_the_process_id() {
        let name = unique_name("t_");
        assert!(
            name.contains(&format!("_{}_", std::process::id())),
            "{name}"
        );
    }

    /// The reaper reads back the creation time; other names are not mistaken for ours.
    #[test]
    fn created_at_round_trips() {
        let before = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let name = unique_name("turul_ext_tasks_");
        let at = created_at_nanos(&name, "turul_ext_tasks_").expect("parses");
        assert!(at >= before, "{at} < {before}");
        assert_eq!(
            created_at_nanos("turul_ext_tasks_123_4_5", "turul_ext_tasks_"),
            Some(123)
        );
        // Databases named before the process id and sequence were added.
        assert_eq!(
            created_at_nanos("turul_ext_tasks_123", "turul_ext_tasks_"),
            Some(123)
        );
        assert_eq!(created_at_nanos("other_123_4_5", "turul_ext_tasks_"), None);
        assert_eq!(
            created_at_nanos("turul_ext_tasks_abc_4_5", "turul_ext_tasks_"),
            None
        );
    }
}
