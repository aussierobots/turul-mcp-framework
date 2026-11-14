# ADR-033: Skills over MCP — `turul-mcp-ext-skills` on `turul-agent-skills`

**Status:** Proposed
**Date:** 2026-11-14
**Crates:** `turul-mcp-ext-skills` (new, 0.1.0: wire types and the MCP-specific producers), `turul-mcp-server` (feature `ext-skills`: handlers, discover advertisement, resource registration)
**External owner:** `turul-agent-skills` 0.1 (sibling repo `aussierobots/turul-agent-skills`)
**Related:** ADR-028 (extensions strategy), ADR-007 (auto-detected resource security), ADR-003 (zero-configuration)

Stays **Proposed** until every alternative path named in §Owners is deleted and the
gates in §Tests pass on the merged tree.

## Context

The Skills extension (`io.modelcontextprotocol/skills`, SEP-2640, moved to
`specification/beta/` by SEP-3392) lets a server publish [Agent Skills](https://agentskills.io/)
over the existing Resources primitive. It is **Beta**: upstream publishes no dated
revision, no tag and no schema file, so the TypeScript blocks in the specification
document are the schema.

### Pin

| File | Upstream repo | Upstream path | Commit | Content sha256 |
|---|---|---|---|---|
| `skills.mdx` | `modelcontextprotocol/ext-skills` | `specification/beta/skills.mdx` | `ff224337b01f0c9b18b81dcf99eefe66e9a52b5c` | `6e33a33762783a02dc4c13c889dda5b8d7c525649012cf77da12d80c04ce6851` |

Vendored at `crates/turul-mcp-ext-skills/schema/skills.mdx`, provenance in that
directory's `README.md`, checked offline by `scripts/check-schema-pin.sh`. Line
references below (`:NNN`) are to the pinned copy. Upstream head at pin time was
`649f7ae` (2026-10-30); `skills.mdx` is byte-identical there.

### What the specification asks of a server

| Id | Clause | Level |
|---|---|---|
| S-G1 | Declare `extensions["io.modelcontextprotocol/skills"] = { directoryRead? }` and the `resources` capability (:37-65) | MUST |
| S-G2 | Implement `skills/list` (paginated; entries atomic; `resultType`, `ttlMs`, `cacheScope`) and `skills/get` (unknown URI → `-32602`; answers for unlisted skills) (:249-454, :569-576) | MUST |
| S-G3 | `frontmatter` is the SKILL.md YAML frontmatter "rendered verbatim as a JSON object", identical in content (:208-210); hosts compare field-by-field after their own parse (:496) | MUST |
| S-G4 | `resources` is a complete `{uri, digest: "sha256:<64 lowercase hex>", size}` manifest over the raw bytes served, including `SKILL.md`, each file once, all within the skill (:223-231) | MUST |
| S-G5 | Final `<skill-path>` segment equals `frontmatter.name` and satisfies the Agent Skills naming rules (:95-98, :211) | MUST |
| S-G6 | Every manifest URI is readable through `resources/read` (:462-464, :573) | MUST |
| S-G7 | `resources/directory/read` when `directoryRead: true` (:63) | MUST if declared |
| S-G8 | 512 resources / 16 MiB per skill (:233-243) | SHOULD NOT exceed |

Core 2026-07-28 Streamable HTTP requires `Mcp-Name` only for `tools/call`,
`resources/read` and `prompts/get`; `skills/*` carry `Mcp-Method` only. The
transport (`turul-http-mcp-server/src/streamable_http.rs`, `_ => None` arm) already
behaves so; no change.

## Decision

### 1. Ownership

| Thing | Owner symbol (entrypoint) | Notes |
|---|---|---|
| SKILL.md parsing, YAML scalar typing flag, name rules, skill validation | `turul_agent_skills::{parse_skill_md, load_str, SkillName}` | Imported, never re-implemented here |
| Enumerating a skill directory's files | `turul_agent_skills::bundled_files(&Skill)` (handoff H1; built, release pending) | The framework contains no directory walker; until that release is on crates.io, catalogs take caller-supplied `(relative path, bytes)` |
| Extension identifier, capability value | `turul_mcp_ext_skills::v2026_07_28::capability::{EXTENSION_IDENTIFIER, SkillsCapability, capability}` | `capability(SkillsCapability)` is the only producer of the `extensions[...]` value |
| `skill://` URI | `v2026_07_28::uri::SkillPath` (`SkillPath::new`, `.skill_md_uri()`, `.file_uri()`) | The only place a `skill://` string is formatted |
| Digest and size | `v2026_07_28::manifest::SkillResource::of(uri, bytes)` | The only producer of a `sha256:` digest |
| Manifest | `v2026_07_28::manifest::Manifest::new` | Completeness, uniqueness, containment, limits, ordering |
| Frontmatter JSON | `v2026_07_28::frontmatter::Frontmatter::from_skill_md` | Projection of `RawSkill::entries`; refuses what it cannot render faithfully |
| Skill entry | `v2026_07_28::entry::SkillEntry::from_files(path, files)` | The only path from bytes to a `Skill` wire entry; composes the four owners above |
| Wire types | `v2026_07_28::types` (`Skill`, `SkillResources`, `ListSkillsResult`, `GetSkillRequestParams`, `GetSkillResult`, method constants, `META_PREFIX`) | Reuses core `PaginatedRequestParams`, `Cursor`, `ResultType`, `CacheScope`, `RequestMetaObject` |
| `skills/list`, `skills/get` handlers; discover advertisement; resource registration | `turul_mcp_server::ext_skills` (`McpServerBuilder::with_ext_skills(SkillCatalog)`) | ADR-028 keeps ext crates free of server runtime |

`SkillResource`, `Manifest` and `SkillEntry` have private fields; their constructors
are the only way to produce one locally, so a consumer cannot assemble a digest or URI
by hand. `SkillResource` also implements `Deserialize`, solely to read entries a peer
sent; that path asserts nothing about bytes.

### 2. URIs (S-G5)

`skill://<prefix…>/<name>/<file-path>`. `SkillPath::new(prefix, name: SkillName)`:
prefix segments are non-empty, consist of RFC 3986 unreserved characters only, and
are not `.`/`..`; the first segment (authority) therefore is a valid `reg-name`. The
final segment is a `SkillName`, so the Agent Skills naming rules are enforced by their
owner. File paths are `/`-joined relative paths whose segments are percent-encoded
outside the unreserved set; empty, `.` and `..` segments are refused. The skill-md URI
is `file_uri("SKILL.md")`. Percent-encoding also keeps `{`/`}` out of every URI, so no
skill file is mistaken for a URI template by the resource registry.

### 3. Frontmatter (S-G3)

The upstream text does not fix a YAML schema, so the JSON type of `1.0`, `yes` or an
empty value depends on the host's parser. First cut: `Frontmatter::from_skill_md`
projects `RawSkill::entries` (strings → JSON strings, sequences → arrays, mappings →
objects, source order) and **refuses** the skill if

- any scalar anywhere carries `RawValue::Str { typed: true }` — `turul-agent-skills`
  sets that flag for every plain scalar a YAML 1.2-core *or* 1.1 parser would type,
  a superset, so refusal errs safe;
- the parse needed lenient repair (`RawSkill::repaired.is_some()`), because a strict
  host parse would then disagree with what we serve;
- `name` or `description` is absent or not a string.

Validation is `turul_agent_skills::load_str` / `load`. Their diagnostics are refusals
with one exception: `UnknownField` ("kept as an extension") is allowed, because the
spec requires every field the author wrote — fields of future Agent Skills revisions
included — to pass through (:208). `YamlRepaired` and every other diagnostic refuse
the skill.

Residual, accepted: mapping *keys* carry no typed flag; JSON keys are strings in any
host. Serving typed values needs a public resolver from `turul-agent-skills`
(handoff H2) and its own revision.

### 4. Manifest (S-G4, S-G8)

`SkillResource::of` hashes the raw bytes with SHA-256 (`sha2`), formats
`sha256:<lowercase hex>`, and records `size = bytes.len()`. `Manifest::new` refuses
duplicates, URIs outside the skill root, a missing `SKILL.md` entry, more than 512
entries, or a total above 16 777 216 bytes, and orders entries by URI, so the same
files in any order produce byte-identical JSON. `"dynamic"` is a wire variant for
clients only; no producer emits it (no dynamic source is approved).

Nested skills: the enclosing skill's files are whatever the caller supplies for its
directory, nested `SKILL.md` included — completeness is the enumerator's contract
(H1). Each nested skill is published as its own flat entry.

### 5. Server wiring (S-G1, S-G2, S-G6)

`with_ext_skills(catalog)`:

- advertises `capability(SkillsCapability { directory_read: false })` and forces
  the `resources` capability, even for an empty catalog;
- registers `skills/list` (stable URI order, fixed page size, cursor = last URI
  served, unknown cursor → `-32602`) and `skills/get` (exact `SKILL.md` URI, else
  `-32602`); both `resultType: "complete"`, `ttlMs: 0`, `cacheScope: "public"`;
- registers every manifest file as an ordinary static resource serving exactly the
  hashed bytes (UTF-8 → text, otherwise blob). One URI, one content: nested skills
  share files with their enclosing skill (:230), so a URI listed by two entries is
  registered once and must carry the same digest in both; a different digest, or a
  URI some other resource already holds — registered before `with_ext_skills` or after
  it — fails `build()`, naming the URI. `SKILL.md` gets `mimeType:
  text/markdown` and `name`/`description` from the frontmatter;
- does **not** register `resources/directory/read`; undeclared, it answers as an
  unknown method (:578).

**Blocklist interaction (Q10).** The auto-generated read policy
(`McpServerBuilder::build_resource_security`) rejects any URI containing `..`,
`/etc/` or `/proc/` on every scheme, so such a skill file would be listed but
unreadable, violating S-G6. Decision: `build()` refuses a catalog with any manifest
URI that policy rejects, with an error naming the file. The check runs each URI through
the auto-generated policy's own `SecurityMiddleware::validate_request("resources/read", …)`
— the call `resources/read` makes — so the blocklist keeps one definition. Reason: no unreadable URI is ever advertised, and no security behaviour
changes. Scoping the blocklist to `file://` would be its own security slice.

**Not `discover`.** `turul_agent_skills::discover` resolves name collisions by
shadowing, which is right for a host but wrong here: the spec makes names labels and
allows `acme/billing/refunds` beside `acme/support/refunds` (:214-221). The catalog
takes explicit `(SkillPath, files)` pairs.

**Directory-backed entries (lands with turul-agent-skills 0.1.3, which adds `bundled_files`; the dependency is then pinned `"0.1.3"`, since `"0.1"` admits patches without it).** `SkillEntry::from_dir(path,
dir)` = `load(dir)` with empty diagnostics, then `bundled_files(&skill)` plus the loaded
`SKILL.md`, then the bytes of each, then `from_files`. `bundled_files` lists symlinks
without following them (turul-agent-skills ADR-011 D19); `from_dir` reads through a
symlink only when its canonical target lies inside the canonical skill directory, and
refuses the skill otherwise, naming the file. A server must not serve bytes from outside
the directory it was pointed at.

Lambda: not covered (`LambdaMcpServerBuilder` cannot advertise extensions; drift D-L).

### 6. Versioning

`turul-mcp-ext-skills` starts at `0.1.0`, versions independently (ADR-028 §5), and
publishes after `turul-mcp-protocol-2026-07-28`, before `turul-mcp-server`.
`turul-agent-skills` (external, crates.io) must already be published. The server
feature is additive: a patch bump of `turul-mcp-server`.

## Tests (executable contract)

Each test is named after the clause it proves.

- `capability`: `s_g1_capability_value_is_owned` (empty object when `directoryRead` is
  false, `{"directoryRead":true}` when true); `declared_by_server_round_trip`.
- `entry`: `s_g4_entry_manifest_matches_bytes`, `s_g5_name_must_equal_final_segment`,
  `repaired_skill_is_refused`, `s_g3_extension_fields_pass_through`,
  `nested_skill_files_are_supporting_files`.
- `uri`: `s_g5_final_segment_is_skill_name`, `distinct_paths_produce_distinct_uris`,
  `prefix_segment_rules`, `file_path_rules`.
- `manifest`: `s_g4_digest_format_and_size`, `distinct_bytes_produce_distinct_digests`,
  `manifest_is_deterministic_under_input_order`, `s_g4_manifest_requires_skill_md`,
  `s_g4_manifest_refuses_duplicates_and_foreign_uris`, `s_g8_limits`.
- `frontmatter`: `s_g3_string_fields_round_trip`, `s_g3_typed_scalar_refused`,
  `s_g3_repaired_refused`, `s_g3_required_fields`, `s_g3_independent_parse_agrees` (projection equals an
  independent YAML parse — the workspace's `serde_yml`, a separate implementation
  from `turul-agent-skills`' `saphyr-parser` — of the same bytes).
- `compliance_test`: `spec_examples_round_trip` (every `skills/list` / `skills/get`
  JSON example in the pinned `skills.mdx`), `resources_dynamic_and_manifest_parse`,
  `resources_other_values_rejected`.
- `owners_route_through_constructors`: no `skill://` or `sha256:` literal is
  formatted outside `uri.rs` / `manifest.rs` in either crate.
- `turul-mcp-server/tests/ext_skills_2026.rs`: `s_g1_discover_advertises_skills_and_resources`,
  `s_g1_empty_catalog_still_declares_resources`, `default_build_advertises_nothing`,
  `s_g2_list_paginates_atomic_entries`, `s_g2_get_by_uri_and_unknown_is_invalid_params`,
  `s_g6_read_bytes_match_manifest`, `skill_md_resource_metadata`,
  `nested_skill_publishes_flat`, `s_g7_directory_read_is_method_not_found`,
  `q10_blocklisted_manifest_uri_refused_at_build`,
  `one_uri_one_content_conflicting_digest_refused`,
  `one_uri_one_content_foreign_resource_refused`.
- Peer (not yet run; blocked on turul-agent-skills 0.1.3 for the directory-backed
  example): conformance `0.2.0-alpha.12` `sep-2640-*` server checks against
  `examples/ext-skills-server`, serving `plugins/turul-mcp-skills/skills/*`.

## Consequences

- Skills a server serves are exactly those whose bytes it hashed at build; content is
  static for the process lifetime. A changed skill needs a restart.
- Skills with typed frontmatter, or with files under `etc/`, `proc/` or containing
  `..`, cannot be served yet; the error says which and why.
- Directory-backed catalogs wait on H1. Until then the catalog is built from
  in-memory files, which also serves skills embedded in a binary.

## Revision log

- **2026-11-14**: Proposed. Same day, after review: `UnknownField` diagnostics no longer
  refuse a skill (:208 passthrough); one-URI-one-content rule added to §5; Q10 text names
  the call actually made.
- **2026-11-14 (initial)**: Pin `ff22433`; ownership table; Q10 decided as
  refuse-at-build through the existing read policy.
