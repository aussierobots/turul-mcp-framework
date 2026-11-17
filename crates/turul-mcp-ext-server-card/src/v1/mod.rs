//! Server Card schema family `v1` (ext-server-card `schema.ts` at `526201b`).
//!
//! Experimental upstream. See the crate README.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use turul_mcp_protocol_2026_07_28::icons::Icon;
use turul_mcp_protocol_2026_07_28::serde_maps;

/// The `$schema` every v1 Server Card carries.
pub const SCHEMA_URL: &str =
    "https://static.modelcontextprotocol.io/schemas/v1/server-card.schema.json";

/// The media type a hosted card is served with.
pub const MEDIA_TYPE: &str = "application/mcp-server-card+json";

/// Appended to the streamable-HTTP URL to form the reserved card location.
pub const PATH_SUFFIX: &str = "/server-card";

/// A static metadata document describing a remote MCP server.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerCard {
    /// Always [`SCHEMA_URL`].
    #[serde(rename = "$schema")]
    pub schema: String,
    /// Reverse-DNS name with exactly one `/`, e.g. `com.example/weather`.
    pub name: String,
    /// The server's version; equals `Implementation.version`.
    pub version: String,
    /// What the server does, 1 to 100 characters.
    pub description: String,
    /// Display name, 1 to 100 characters.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// Homepage or documentation URL.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub website_url: Option<String>,
    /// Source repository.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repository: Option<Repository>,
    /// Icons for display.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icons: Option<Vec<Icon>>,
    /// HTTP endpoints a client can connect to.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remotes: Option<Vec<Remote>>,
    /// Vendor metadata under reverse-DNS keys.
    #[serde(
        rename = "_meta",
        skip_serializing_if = "Option::is_none",
        serialize_with = "serde_maps::sorted_opt"
    )]
    pub meta: Option<HashMap<String, Value>>,
}

/// Source repository metadata.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Repository {
    /// Repository URL.
    pub url: String,
    /// Hosting service, e.g. `"github"`.
    pub source: String,
    /// Relative path to the server inside a monorepo.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subfolder: Option<String>,
    /// Stable repository id from the hosting service.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
}

/// Transport of a remote endpoint.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RemoteType {
    /// Streamable HTTP.
    #[serde(rename = "streamable-http")]
    StreamableHttp,
    /// Legacy HTTP+SSE.
    #[serde(rename = "sse")]
    Sse,
}

/// A remote (HTTP) endpoint of the server.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Remote {
    /// Transport type.
    #[serde(rename = "type")]
    pub remote_type: RemoteType,
    /// URL template: `http(s)://…` or starting with a `{variable}`.
    pub url: String,
    /// Headers required or accepted by the endpoint.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headers: Option<Vec<KeyValueInput>>,
    /// Variables referenced as `{name}` in `url`.
    #[serde(
        skip_serializing_if = "Option::is_none",
        serialize_with = "serde_maps::sorted_opt"
    )]
    pub variables: Option<HashMap<String, Input>>,
    /// MCP protocol versions the endpoint supports.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub supported_protocol_versions: Option<Vec<String>>,
}

/// A user-supplied or pre-set input value.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Input {
    /// Human-readable explanation that clients can show the user as context.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Whether the input must be supplied for the connection to succeed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_required: Option<bool>,
    /// Whether the input is a secret (for example a password or token) that
    /// clients should handle securely.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_secret: Option<bool>,
    /// Format of the input value.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<InputFormat>,
    /// Default value; it should be a valid value for the input.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<String>,
    /// Example or guidance shown during configuration about the expected form
    /// of the input.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub placeholder: Option<String>,
    /// Pre-set value that end users should not configure; `{name}` identifiers
    /// are replaced from the enclosing `variables` map, if any.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    /// Allowed values; when present, the user must select one of them.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub choices: Option<Vec<String>>,
}

/// How an [`Input`] value is interpreted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum InputFormat {
    /// A string.
    String,
    /// A number, written as a decimal value when converted to a string.
    Number,
    /// A boolean, written as `"true"` or `"false"` when converted to a string.
    Boolean,
    /// A path to a file on the user's filesystem.
    Filepath,
}

/// A named [`Input`], used for HTTP headers.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct KeyValueInput {
    /// Header name.
    pub name: String,
    /// The header's input definition, flattened into the header object.
    #[serde(flatten)]
    pub input: Input,
    /// Variables referenced as `{name}` in `value`.
    #[serde(
        skip_serializing_if = "Option::is_none",
        serialize_with = "serde_maps::sorted_opt"
    )]
    pub variables: Option<HashMap<String, Input>>,
}

/// A card that breaks a `schema.json` constraint.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvalidServerCard(pub String);

impl std::fmt::Display for InvalidServerCard {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "invalid server card: {}", self.0)
    }
}

impl std::error::Error for InvalidServerCard {}

impl ServerCard {
    /// Creates a card with the v1 `$schema`, a name and a description.
    ///
    /// `version` starts empty; `turul-mcp-server` fills it from the server's
    /// `Implementation`.
    pub fn new(name: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            schema: SCHEMA_URL.to_string(),
            name: name.into(),
            version: String::new(),
            description: description.into(),
            title: None,
            website_url: None,
            repository: None,
            icons: None,
            remotes: None,
            meta: None,
        }
    }

    /// Sets `version` and returns the card.
    pub fn with_version(mut self, version: impl Into<String>) -> Self {
        self.version = version.into();
        self
    }

    /// Sets the display `title` and returns the card.
    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Sets `websiteUrl` and returns the card.
    pub fn with_website_url(mut self, url: impl Into<String>) -> Self {
        self.website_url = Some(url.into());
        self
    }

    /// Sets `repository` and returns the card.
    pub fn with_repository(mut self, repository: Repository) -> Self {
        self.repository = Some(repository);
        self
    }

    /// Sets `icons` and returns the card.
    pub fn with_icons(mut self, icons: Vec<Icon>) -> Self {
        self.icons = Some(icons);
        self
    }

    /// Appends a remote endpoint to `remotes` and returns the card.
    pub fn with_remote(mut self, remote: Remote) -> Self {
        self.remotes.get_or_insert_with(Vec::new).push(remote);
        self
    }

    /// Sets the extension `_meta` map and returns the card.
    pub fn with_meta(mut self, meta: HashMap<String, Value>) -> Self {
        self.meta = Some(meta);
        self
    }

    /// Checks the constraints `schema.json` asserts, plus the documented
    /// refusal of version ranges.
    ///
    /// # Errors
    ///
    /// Returns [`InvalidServerCard`] when `$schema` is not [`SCHEMA_URL`], the
    /// `name` length or pattern is wrong, `version` exceeds 255 characters or is
    /// a range, `description` or `title` is not 1 to 100 characters, or a remote
    /// `url` is not an `http(s)://` or `{variable}` URL without whitespace.
    pub fn validate(&self) -> Result<(), InvalidServerCard> {
        let bad = |m: String| Err(InvalidServerCard(m));
        if self.schema != SCHEMA_URL {
            return bad(format!(
                "$schema must be {SCHEMA_URL}, got {:?}",
                self.schema
            ));
        }
        let name_len = self.name.chars().count();
        if !(3..=200).contains(&name_len) {
            return bad(format!("name must be 3 to 200 characters, got {name_len}"));
        }
        if !is_reverse_dns_name(&self.name) {
            return bad(format!(
                "name {:?} must match ^[a-zA-Z0-9.-]+/[a-zA-Z0-9._-]+$",
                self.name
            ));
        }
        if self.version.chars().count() > 255 {
            return bad("version is longer than 255 characters".into());
        }
        if is_version_range(&self.version) {
            return bad(format!(
                "version {:?} is a range, not a version",
                self.version
            ));
        }
        check_len("description", &self.description)?;
        if let Some(title) = &self.title {
            check_len("title", title)?;
        }
        for remote in self.remotes.iter().flatten() {
            if !is_remote_url(&remote.url) {
                return bad(format!(
                    "remote url {:?} must start with http://, https:// or a {{variable}} and contain no whitespace",
                    remote.url
                ));
            }
        }
        Ok(())
    }
}

impl Remote {
    /// Creates a remote of the given transport at `url`, with no headers,
    /// variables or protocol versions.
    pub fn new(remote_type: RemoteType, url: impl Into<String>) -> Self {
        Self {
            remote_type,
            url: url.into(),
            headers: None,
            variables: None,
            supported_protocol_versions: None,
        }
    }

    /// Appends a header to `headers` and returns the remote.
    pub fn with_header(mut self, header: KeyValueInput) -> Self {
        self.headers.get_or_insert_with(Vec::new).push(header);
        self
    }

    /// Inserts the variable referenced as `{name}` in `url`, replacing any
    /// previous variable of that name, and returns the remote.
    pub fn with_variable(mut self, name: impl Into<String>, input: Input) -> Self {
        self.variables
            .get_or_insert_with(HashMap::new)
            .insert(name.into(), input);
        self
    }

    /// Sets `supportedProtocolVersions` and returns the remote.
    pub fn with_supported_protocol_versions(mut self, versions: Vec<String>) -> Self {
        self.supported_protocol_versions = Some(versions);
        self
    }
}

fn check_len(field: &str, s: &str) -> Result<(), InvalidServerCard> {
    let n = s.chars().count();
    if (1..=100).contains(&n) {
        Ok(())
    } else {
        Err(InvalidServerCard(format!(
            "{field} must be 1 to 100 characters, got {n}"
        )))
    }
}

/// `^[a-zA-Z0-9.-]+/[a-zA-Z0-9._-]+$`
fn is_reverse_dns_name(s: &str) -> bool {
    let Some((ns, name)) = s.split_once('/') else {
        return false;
    };
    !ns.is_empty()
        && !name.is_empty()
        && ns
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-')
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '_' || c == '-')
}

/// `^(https?://[^\s]+|\{[a-zA-Z_][a-zA-Z0-9_]*\}[^\s]*)$`
fn is_remote_url(s: &str) -> bool {
    if s.chars().any(char::is_whitespace) {
        return false;
    }
    if let Some(rest) = s
        .strip_prefix("https://")
        .or_else(|| s.strip_prefix("http://"))
    {
        return !rest.is_empty();
    }
    let Some(rest) = s.strip_prefix('{') else {
        return false;
    };
    let Some((var, _)) = rest.split_once('}') else {
        return false;
    };
    let mut chars = var.chars();
    matches!(chars.next(), Some(c) if c.is_ascii_alphabetic() || c == '_')
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// The ranges the schema's `version` description rejects: `^1.2.3`, `~1.2.3`,
/// `>=1.2.3`, `1.x`, `1.*`.
fn is_version_range(v: &str) -> bool {
    v.starts_with(['^', '~', '>', '<', '='])
        || v.split('.').any(|seg| matches!(seg, "x" | "X" | "*"))
}

/// The served form of a card: its bytes and their `ETag`. The one producer of
/// both.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServerCardDocument {
    body: Vec<u8>,
    etag: String,
}

impl ServerCardDocument {
    /// Validates `card`, serialises it (map keys sorted), and derives the strong
    /// `ETag` `"<first 32 hex digits of sha256(body)>"`.
    ///
    /// # Errors
    ///
    /// Returns [`InvalidServerCard`] when [`ServerCard::validate`] fails or
    /// serialisation fails.
    pub fn new(card: &ServerCard) -> Result<Self, InvalidServerCard> {
        card.validate()?;
        let body = serde_json::to_vec(card)
            .map_err(|e| InvalidServerCard(format!("serialisation failed: {e}")))?;
        let digest = Sha256::digest(&body);
        let hex: String = digest.iter().take(16).map(|b| format!("{b:02x}")).collect();
        Ok(Self {
            body,
            etag: format!("\"{hex}\""),
        })
    }

    /// The JSON bytes served.
    pub fn body(&self) -> &[u8] {
        &self.body
    }

    /// The quoted strong entity tag.
    pub fn etag(&self) -> &str {
        &self.etag
    }
}

#[cfg(test)]
mod tests;
