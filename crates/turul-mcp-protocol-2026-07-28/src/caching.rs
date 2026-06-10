//! Cache-control hints for results (SEP-2549).
//!
//! DRAFT-2026-v1 introduces a `CacheableResult` mixin that all list and read
//! results extend — `ttlMs` carries a client-side TTL hint (in milliseconds,
//! HTTP-`Cache-Control: max-age` semantics) and `cacheScope` indicates whether
//! the response is safe to share across users.
//!
//! Per schema: "A result that supports a time-to-live (TTL) hint for
//! client-side caching." Both fields are **required** when present on a
//! `CacheableResult`.
//!
//! ## Embedding pattern
//!
//! The schema's `extends CacheableResult` becomes `#[serde(flatten)] cache:
//! CacheableResult` in Rust. See `ListToolsResult`, `ReadResourceResult`, etc.

use serde::{Deserialize, Serialize};

/// Cache-sharing scope, analogous to HTTP `Cache-Control: public` / `private`.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum CacheScope {
    /// Any client or intermediary MAY cache and serve to any user.
    Public,
    /// Only the requesting user's client MAY cache. Shared caches MUST NOT
    /// serve a cached copy to a different user.
    Private,
}

impl CacheScope {
    /// Wire string for this scope.
    pub const fn as_str(&self) -> &'static str {
        match self {
            CacheScope::Public => "public",
            CacheScope::Private => "private",
        }
    }
}

impl std::fmt::Display for CacheScope {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Cacheable-result mixin per schema. Embed via `#[serde(flatten)]`:
///
/// ```ignore
/// pub struct ListToolsResult {
///     #[serde(default)]
///     pub result_type: crate::result_type::ResultType,
///
///     #[serde(flatten)]
///     pub cache: crate::caching::CacheableResult,
///
///     // ... domain-specific fields ...
/// }
/// ```
///
/// Both fields are REQUIRED on the wire. Constructors should always set them.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CacheableResult {
    /// Cache lifetime hint in milliseconds.
    ///
    /// - `0` means immediately stale; client MAY re-fetch every use.
    /// - Positive: client SHOULD consider the result fresh for this many ms.
    ///
    /// Schema type is `number` (`@minimum 0`) — fractional values are
    /// spec-legal and accepted on deserialize; whole values serialize as
    /// integers so the common case keeps its compact wire form.
    #[serde(with = "ttl_ms_serde")]
    pub ttl_ms: f64,

    /// Sharing scope.
    pub cache_scope: CacheScope,
}

impl CacheableResult {
    /// Construct with both required fields.
    pub fn new(ttl_ms: f64, cache_scope: CacheScope) -> Self {
        Self {
            ttl_ms,
            cache_scope,
        }
    }

    /// Convenience: immediately-stale public response (`ttlMs=0`, public scope).
    pub fn stale_public() -> Self {
        Self::new(0.0, CacheScope::Public)
    }

    /// Convenience: 60-second private cache.
    pub fn private_60s() -> Self {
        Self::new(60_000.0, CacheScope::Private)
    }
}

/// Serde for `ttlMs`: the schema declares `number` with `@minimum 0`.
/// Deserialize accepts any non-negative finite number; serialize emits an
/// integer for whole values (compact, byte-stable for the common case) and a
/// float otherwise.
pub mod ttl_ms_serde {
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S: Serializer>(value: &f64, serializer: S) -> Result<S::Ok, S::Error> {
        if value.fract() == 0.0 && *value >= 0.0 && *value <= (1u64 << 53) as f64 {
            serializer.serialize_u64(*value as u64)
        } else {
            serializer.serialize_f64(*value)
        }
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<f64, D::Error> {
        let value = f64::deserialize(deserializer)?;
        if !value.is_finite() || value < 0.0 {
            return Err(serde::de::Error::custom(
                "ttlMs must be a non-negative finite number",
            ));
        }
        Ok(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn cache_scope_serializes_lowercase() {
        // The literal strings are "public" / "private".
        let pub_v = serde_json::to_value(CacheScope::Public).unwrap();
        let priv_v = serde_json::to_value(CacheScope::Private).unwrap();
        assert_eq!(pub_v, "public");
        assert_eq!(priv_v, "private");
    }

    #[test]
    fn cache_scope_parses_lowercase_only() {
        let p: CacheScope = serde_json::from_str("\"public\"").unwrap();
        assert_eq!(p, CacheScope::Public);
        let q: CacheScope = serde_json::from_str("\"private\"").unwrap();
        assert_eq!(q, CacheScope::Private);

        // Other casings rejected.
        let bad: Result<CacheScope, _> = serde_json::from_str("\"Public\"");
        assert!(bad.is_err(), "PascalCase not accepted per schema literal");
    }

    #[test]
    fn cacheable_result_serializes_required_fields() {
        // Both fields REQUIRED on the wire.
        let c = CacheableResult::new(5000.0, CacheScope::Public);
        let v = serde_json::to_value(&c).unwrap();
        assert_eq!(v["ttlMs"], 5000);
        assert_eq!(v["cacheScope"], "public");
    }

    #[test]
    fn cacheable_result_rejects_missing_ttl_ms() {
        // Required field — must fail to parse without it.
        let bad = json!({"cacheScope": "public"});
        let r: Result<CacheableResult, _> = serde_json::from_value(bad);
        assert!(
            r.is_err(),
            "missing ttlMs must reject (schema marks REQUIRED)"
        );
    }

    #[test]
    fn cacheable_result_rejects_missing_cache_scope() {
        let bad = json!({"ttlMs": 0});
        let r: Result<CacheableResult, _> = serde_json::from_value(bad);
        assert!(
            r.is_err(),
            "missing cacheScope must reject (schema marks REQUIRED)"
        );
    }

    #[test]
    fn cacheable_result_accepts_zero_ttl() {
        // Schema: "If 0, the response SHOULD be considered immediately stale".
        let c = CacheableResult::new(0.0, CacheScope::Public);
        let v = serde_json::to_value(&c).unwrap();
        assert_eq!(v["ttlMs"], 0);
    }

    #[test]
    fn cacheable_result_round_trips() {
        let c = CacheableResult::new(86_400_000.0, CacheScope::Private);
        let s = serde_json::to_string(&c).unwrap();
        let parsed: CacheableResult = serde_json::from_str(&s).unwrap();
        assert_eq!(parsed.ttl_ms, 86_400_000.0);
        assert_eq!(parsed.cache_scope, CacheScope::Private);
    }

    #[test]
    fn cacheable_result_flatten_pattern_round_trips() {
        // Verifies the intended embedding pattern: `#[serde(flatten)] cache: CacheableResult`
        // — both fields appear at the parent's top level.
        #[derive(Debug, Serialize, Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct ListLike {
            tools: Vec<String>,
            #[serde(flatten)]
            cache: CacheableResult,
        }

        let l = ListLike {
            tools: vec!["echo".to_string()],
            cache: CacheableResult::new(60_000.0, CacheScope::Public),
        };
        let v = serde_json::to_value(&l).unwrap();
        assert_eq!(v["tools"][0], "echo");
        assert_eq!(v["ttlMs"], 60_000, "flattened ttlMs at parent top level");
        assert_eq!(
            v["cacheScope"], "public",
            "flattened cacheScope at parent top level"
        );

        let s = serde_json::to_string(&v).unwrap();
        let parsed: ListLike = serde_json::from_str(&s).unwrap();
        assert_eq!(parsed.cache.ttl_ms, 60_000.0);
        assert_eq!(parsed.cache.cache_scope, CacheScope::Public);
    }

    #[test]
    fn helpers_produce_expected_values() {
        let stale = CacheableResult::stale_public();
        assert_eq!(stale.ttl_ms, 0.0);
        assert_eq!(stale.cache_scope, CacheScope::Public);

        let p60 = CacheableResult::private_60s();
        assert_eq!(p60.ttl_ms, 60_000.0);
        assert_eq!(p60.cache_scope, CacheScope::Private);
    }
}
