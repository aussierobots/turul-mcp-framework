//! MCP Handler System
//!
//! This module provides a standardized handler system for MCP endpoints.

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use serde_json::Value;
#[cfg(feature = "protocol-2025-11-25")]
use serde_json::json;
use tracing::debug;

use crate::resource::{McpResource, resource_to_descriptor};

use crate::{McpResult, SessionContext};
use turul_mcp_protocol::McpError;

//pub mod response;
//pub use response::*;

/// Extract limit parameter from raw params (supports both params.limit and params._meta.limit)
///
/// MCP spec allows arbitrary extension fields via `[key: string]: unknown` in both
/// params root and _meta. This helper extracts limit from either location before
/// parsing to typed params.
fn extract_limit_from_params(params: &Option<Value>) -> Option<usize> {
    params.as_ref().and_then(|p| {
        // Try direct params.limit first
        p.get("limit")
            .or_else(|| p.get("_meta").and_then(|m| m.get("limit")))
            .and_then(|v| v.as_u64())
            .map(|n| n as usize)
    })
}

/// Read the pagination cursor from raw `params.cursor`.
fn extract_cursor_from_params(params: &Option<Value>) -> Option<turul_mcp_protocol::meta::Cursor> {
    params
        .as_ref()
        .and_then(|p| p.get("cursor"))
        .and_then(|c| c.as_str())
        .map(turul_mcp_protocol::meta::Cursor::from)
}

/// Caller-supplied `_meta` keys to echo back onto the result, excluding the
/// request-scoped routing keys defined by the protocol meta object.
/// MRTR (SEP-2322): convert a provider's `McpError::InputRequired` outcome
/// into a successful `InputRequiredResult` value, after enforcing that every
/// input request targets a capability the client declared in the request's
/// `_meta` `clientCapabilities` (undeclared → `-32003`, HTTP 400). Gating is
/// mode-aware: URL-mode elicitation needs `elicitation.url`, form mode rides
/// the empty object ("an empty capabilities object is equivalent to declaring
/// support for form mode only"), and tool-enabled sampling (`tools` /
/// `toolChoice` present) needs `sampling.tools`. Shared by the three methods
/// permitted to return `input_required`: `tools/call`, `resources/read`,
/// `prompts/get`.
#[cfg(feature = "protocol-2026-07-28")]
pub(crate) fn input_required_to_result(
    input_requests: Option<turul_mcp_protocol::input_required::InputRequests>,
    request_state: Option<String>,
    caps: &turul_mcp_protocol::initialize::ClientCapabilities,
) -> McpResult<Value> {
    use turul_mcp_protocol::input_required::{InputRequest, InputRequiredResult};

    if let Some(ref requests) = input_requests {
        for request in requests.values() {
            #[allow(deprecated)]
            let missing = match request {
                InputRequest::Elicit(elicit) => match &caps.elicitation {
                    None => Some(serde_json::json!({ "elicitation": {} })),
                    Some(e) => {
                        use turul_mcp_protocol::elicitation::ElicitRequestParams;
                        match &elicit.params {
                            // URL mode needs the explicit url sub-capability.
                            ElicitRequestParams::Url(_) if e.url.is_none() => {
                                Some(serde_json::json!({ "elicitation": { "url": {} } }))
                            }
                            // Form mode: an empty capabilities object declares
                            // form-only support; an explicit url-only object
                            // does NOT imply form.
                            ElicitRequestParams::Form(_) if e.form.is_none() && e.url.is_some() => {
                                Some(serde_json::json!({ "elicitation": { "form": {} } }))
                            }
                            _ => None,
                        }
                    }
                },
                InputRequest::CreateMessage(create) => match &caps.sampling {
                    None => Some(serde_json::json!({ "sampling": {} })),
                    // Tool-enabled sampling needs the tools sub-capability.
                    Some(sc)
                        if (create.params.tools.is_some()
                            || create.params.tool_choice.is_some())
                            && sc.tools.is_none() =>
                    {
                        Some(serde_json::json!({ "sampling": { "tools": {} } }))
                    }
                    Some(_) => None,
                },
                InputRequest::ListRoots(_) if caps.roots.is_none() => {
                    Some(serde_json::json!({ "roots": {} }))
                }
                _ => None,
            };
            if let Some(required) = missing {
                return Err(McpError::MissingRequiredClientCapability { required });
            }
        }
    }

    let result = match (input_requests, request_state) {
        (Some(requests), Some(state)) => {
            InputRequiredResult::with_requests_and_state(requests, state)
        }
        (Some(requests), None) => InputRequiredResult::with_requests(requests),
        (None, Some(state)) => InputRequiredResult::with_state(state),
        (None, None) => {
            // Schema invariant: at least one field must be present.
            return Err(McpError::ToolExecutionError(
                "InputRequired with neither inputRequests nor requestState".into(),
            ));
        }
    };
    serde_json::to_value(result).map_err(McpError::from)
}

pub(crate) fn extract_request_meta_extra(params: &Option<Value>) -> HashMap<String, Value> {
    const RESERVED: &[&str] = &[
        "progressToken",
        "io.modelcontextprotocol/protocolVersion",
        "io.modelcontextprotocol/clientInfo",
        "io.modelcontextprotocol/clientCapabilities",
        "io.modelcontextprotocol/logLevel",
    ];
    params
        .as_ref()
        .and_then(|p| p.get("_meta"))
        .and_then(|m| m.as_object())
        .map(|obj| {
            obj.iter()
                .filter(|(k, _)| !RESERVED.contains(&k.as_str()))
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect()
        })
        .unwrap_or_default()
}

/// Finalize a paginated list result with cursor/total/hasMore context and the
/// caller's echoed `_meta` extras.
///
/// On 2025-11-25 the cursor/total/hasMore travel in a `PaginatedResponse`
/// `_meta` envelope. On DRAFT-2026-v1 the result carries `nextCursor` directly
/// and `_meta` holds only the echoed caller extras (the schema `Result` has no
/// total/hasMore fields).
fn paginate_list_response<T: serde::Serialize>(
    base_response: T,
    next_cursor: Option<turul_mcp_protocol::meta::Cursor>,
    total: Option<u64>,
    has_more: bool,
    request_meta_extra: HashMap<String, Value>,
) -> McpResult<Value> {
    #[cfg(feature = "protocol-2025-11-25")]
    {
        let _ = &total;
        let mut response = turul_mcp_protocol::meta::PaginatedResponse::with_pagination(
            base_response,
            next_cursor,
            total,
            has_more,
        );
        if !request_meta_extra.is_empty() {
            use turul_mcp_protocol::meta::WithMeta;
            let mut response_meta = response.meta().cloned().unwrap_or_else(|| {
                turul_mcp_protocol::meta::Meta::with_pagination(None, total, has_more)
            });
            for (key, value) in request_meta_extra {
                response_meta.extra.insert(key, value);
            }
            response = response.with_meta(response_meta);
        }
        serde_json::to_value(response).map_err(McpError::from)
    }
    #[cfg(not(feature = "protocol-2025-11-25"))]
    {
        let _ = (&next_cursor, &total, &has_more);
        let mut value = serde_json::to_value(base_response).map_err(McpError::from)?;
        if !request_meta_extra.is_empty()
            && let Some(obj) = value.as_object_mut()
        {
            let meta = obj
                .entry("_meta")
                .or_insert_with(|| Value::Object(serde_json::Map::new()));
            if let Some(meta_obj) = meta.as_object_mut() {
                for (key, val) in request_meta_extra {
                    meta_obj.insert(key, val);
                }
            }
        }
        Ok(value)
    }
}

/// Generic MCP handler trait
#[async_trait]
pub trait McpHandler: Send + Sync {
    /// Handle an MCP request
    async fn handle(&self, params: Option<Value>) -> McpResult<Value>;

    /// Handle an MCP request with session context (default implementation calls handle)
    async fn handle_with_session(
        &self,
        params: Option<Value>,
        _session: Option<SessionContext>,
    ) -> McpResult<Value> {
        self.handle(params).await
    }

    /// Get the methods this handler supports
    fn supported_methods(&self) -> Vec<String>;
}

/// Ping handler for ping endpoint
pub struct PingHandler;

#[async_trait]
impl McpHandler for PingHandler {
    async fn handle(&self, _params: Option<Value>) -> McpResult<Value> {
        Ok(serde_json::json!({}))
    }

    fn supported_methods(&self) -> Vec<String> {
        vec!["ping".to_string()]
    }
}

/// Completion handler for the completion/complete endpoint.
///
/// Routes to registered [`McpCompletion`](crate::McpCompletion) providers:
/// highest `priority()` first, first `can_handle()` wins. Responses honor the
/// spec's 100-item cap on `completion.values` (oversized provider output is
/// truncated with `total`/`hasMore` reflecting the cut). With no matching
/// provider the result carries empty values.
#[derive(Default)]
pub struct CompletionHandler {
    providers: Vec<Arc<dyn crate::McpCompletion>>,
}

impl CompletionHandler {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_providers(mut self, providers: Vec<Arc<dyn crate::McpCompletion>>) -> Self {
        self.providers = providers;
        // Stable sort: priority desc, insertion order as the tiebreak.
        self.providers
            .sort_by_key(|p| std::cmp::Reverse(p.priority()));
        self
    }
}

/// Does a provider's declared reference target the request's reference?
fn completion_reference_matches(
    declared: &turul_mcp_protocol::completion::CompletionReference,
    requested: &turul_mcp_protocol::completion::CompletionReference,
) -> bool {
    use turul_mcp_protocol::completion::CompletionReference as Ref;
    match (declared, requested) {
        (Ref::Prompt(a), Ref::Prompt(b)) => a.name == b.name,
        (Ref::ResourceTemplate(a), Ref::ResourceTemplate(b)) => a.uri == b.uri,
        _ => false,
    }
}

/// `completion.values` carries "Maximum 100 items" per the Completion spec.
const COMPLETION_VALUES_CAP: usize = 100;

#[async_trait]
impl McpHandler for CompletionHandler {
    async fn handle(&self, params: Option<Value>) -> McpResult<Value> {
        use turul_mcp_protocol::completion::{CompleteRequest, CompleteResult, CompletionResult};
        // The params struct is `CompleteRequestParams` on 2026-07-28 (required
        // `_meta`) and `CompleteParams` on the frozen 2025-11-25 snapshot.
        #[cfg(feature = "protocol-2025-11-25")]
        use turul_mcp_protocol::completion::CompleteParams as TypedCompleteParams;
        #[cfg(feature = "protocol-2026-07-28")]
        use turul_mcp_protocol::completion::CompleteRequestParams as TypedCompleteParams;

        let params = params.ok_or_else(|| {
            McpError::InvalidParameters("completion/complete requires params".to_string())
        })?;
        let typed: TypedCompleteParams = serde_json::from_value(params)
            .map_err(|e| McpError::InvalidParameters(format!("invalid completion params: {e}")))?;
        // The untagged reference union accepts any `type` string — enforce
        // the schema's literals ("ref/prompt" / "ref/resource") here.
        let ref_type = match &typed.reference {
            turul_mcp_protocol::completion::CompletionReference::Prompt(p) => {
                (&p.ref_type, "ref/prompt")
            }
            turul_mcp_protocol::completion::CompletionReference::ResourceTemplate(r) => {
                (&r.ref_type, "ref/resource")
            }
        };
        if ref_type.0 != ref_type.1 {
            return Err(McpError::InvalidParameters(format!(
                "unknown completion reference type {:?} (expected \"ref/prompt\" or \"ref/resource\")",
                ref_type.0
            )));
        }
        let request = CompleteRequest {
            method: "completion/complete".to_string(),
            params: typed,
        };

        // Providers declaring the request's exact reference win; a provider
        // whose can_handle accepts the request is the fallback. Both passes
        // run in (priority desc, insertion) order, so routing is
        // deterministic.
        let provider = self
            .providers
            .iter()
            .find(|p| {
                completion_reference_matches(p.reference(), &request.params.reference)
                    && p.can_handle(&request)
            })
            .or_else(|| self.providers.iter().find(|p| p.can_handle(&request)));
        let result = match provider {
            Some(provider) => {
                crate::McpCompletion::validate_request(provider.as_ref(), &request).await?;
                let mut result = provider.complete(request).await?;
                let len = result.completion.values.len();
                if len > COMPLETION_VALUES_CAP {
                    result.completion.values.truncate(COMPLETION_VALUES_CAP);
                    result.completion.has_more = Some(true);
                    result.completion.total.get_or_insert(len as u32);
                }
                result
            }
            None => CompleteResult::new(CompletionResult::new(Vec::new())),
        };
        serde_json::to_value(result).map_err(McpError::from)
    }

    fn supported_methods(&self) -> Vec<String> {
        vec!["completion/complete".to_string()]
    }
}

/// Prompts list handler for prompts/list endpoint only
pub struct PromptsListHandler {
    prompts: HashMap<String, Arc<dyn McpPrompt>>,
}

impl Default for PromptsListHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl PromptsListHandler {
    pub fn new() -> Self {
        Self {
            prompts: HashMap::new(),
        }
    }

    pub fn add_prompt<P: McpPrompt + 'static>(mut self, prompt: P) -> Self {
        self.prompts
            .insert(prompt.name().to_string(), Arc::new(prompt));
        self
    }

    pub fn add_prompt_arc(mut self, prompt: Arc<dyn McpPrompt>) -> Self {
        self.prompts.insert(prompt.name().to_string(), prompt);
        self
    }
}

#[async_trait]
impl McpHandler for PromptsListHandler {
    async fn handle(&self, params: Option<Value>) -> McpResult<Value> {
        // Handle prompts/list with pagination support
        use turul_mcp_protocol::meta::Cursor;
        use turul_mcp_protocol::prompts::{ListPromptsResult, Prompt};

        // Extract limit from raw params before parsing to typed params (MCP extension field)
        // Clamp to 1000 for DoS protection, reject zero
        const DEFAULT_PAGE_SIZE: usize = 50;
        const MAX_PAGE_SIZE: usize = 1000;
        const MIN_PAGE_SIZE: usize = 1;

        let page_size = match extract_limit_from_params(&params) {
            Some(0) => {
                return Err(McpError::InvalidParameters(
                    "limit must be at least 1 (zero would return empty pages forever)".to_string(),
                ));
            }
            Some(n) => n.clamp(MIN_PAGE_SIZE, MAX_PAGE_SIZE),
            None => DEFAULT_PAGE_SIZE,
        };

        let cursor = extract_cursor_from_params(&params);
        let request_meta_extra = extract_request_meta_extra(&params);

        debug!(
            "Listing prompts with cursor: {:?}, limit: {}",
            cursor, page_size
        );

        // Convert all prompts and sort by name for stable ordering
        let mut all_prompts: Vec<Prompt> = self
            .prompts
            .values()
            .map(|p| {
                let mut prompt = Prompt::new(p.name());
                if let Some(title) = p.title() {
                    prompt = prompt.with_title(title);
                }
                if let Some(desc) = p.description() {
                    prompt = prompt.with_description(desc);
                }
                // Include arguments from the prompt object
                if let Some(args) = p.arguments() {
                    prompt = prompt.with_arguments(args.clone());
                }
                if let Some(icons) = p.icons() {
                    prompt = prompt.with_icons(icons.to_vec());
                }
                if let Some(meta) = p.prompt_meta() {
                    prompt = prompt.with_meta(meta.clone());
                }
                prompt
            })
            .collect();

        // Sort by name to ensure stable pagination ordering (MCP 2025-11-25 requirement)
        all_prompts.sort_by(|a, b| a.name.cmp(&b.name));

        // Find starting index based on cursor
        let start_index = if let Some(cursor) = &cursor {
            // Cursor contains the last name from previous page
            let cursor_name = cursor.as_str();
            // Pagination §Error Handling: "Invalid cursors SHOULD result in an
            // error with code -32602" — a cursor must be one this server
            // issued (the last prompt name of a previously served page).
            if !all_prompts.iter().any(|p| p.name.as_str() == cursor_name) {
                return Err(McpError::InvalidParameters(format!(
                    "invalid pagination cursor {:?}",
                    cursor_name
                )));
            }

            // Find the position after the cursor name
            all_prompts
                .iter()
                .position(|p| p.name.as_str() > cursor_name)
                .unwrap_or(all_prompts.len())
        } else {
            0 // No cursor = start from beginning
        };

        // Calculate end index for this page
        let end_index = std::cmp::min(start_index + page_size, all_prompts.len());

        // Extract page of prompts
        let page_prompts: Vec<Prompt> = all_prompts[start_index..end_index].to_vec();

        // Determine if there are more prompts after this page
        let has_more = end_index < all_prompts.len();

        // Generate next cursor if there are more prompts
        let next_cursor = if has_more {
            // Cursor should be the name of the last item in current page
            page_prompts.last().map(|p| Cursor::new(&p.name))
        } else {
            None
        };

        debug!(
            "Prompt pagination: start={}, end={}, page_size={}, has_more={}, next_cursor={:?}",
            start_index,
            end_index,
            page_prompts.len(),
            has_more,
            next_cursor
        );

        let mut base_response = ListPromptsResult::new(page_prompts);

        if let Some(ref cursor) = next_cursor {
            base_response = base_response.with_next_cursor(cursor.clone());
        }

        let total = Some(all_prompts.len() as u64);

        paginate_list_response(
            base_response,
            next_cursor,
            total,
            has_more,
            request_meta_extra,
        )
    }

    fn supported_methods(&self) -> Vec<String> {
        vec!["prompts/list".to_string()]
    }
}

/// Prompts get handler for prompts/get endpoint only
pub struct PromptsGetHandler {
    prompts: HashMap<String, Arc<dyn McpPrompt>>,
}

impl Default for PromptsGetHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl PromptsGetHandler {
    pub fn new() -> Self {
        Self {
            prompts: HashMap::new(),
        }
    }

    pub fn add_prompt<P: McpPrompt + 'static>(mut self, prompt: P) -> Self {
        self.prompts
            .insert(prompt.name().to_string(), Arc::new(prompt));
        self
    }

    pub fn add_prompt_arc(mut self, prompt: Arc<dyn McpPrompt>) -> Self {
        self.prompts.insert(prompt.name().to_string(), prompt);
        self
    }
}

#[async_trait]
impl McpHandler for PromptsGetHandler {
    async fn handle(&self, params: Option<Value>) -> McpResult<Value> {
        use std::collections::HashMap as StdHashMap;
        use turul_mcp_protocol::prompts::GetPromptResult;

        let request_meta_extra = extract_request_meta_extra(&params);

        // Parse get prompt parameters
        let params = params.ok_or_else(|| McpError::missing_param("GetPromptParams"))?;
        #[cfg(feature = "protocol-2025-11-25")]
        let get_params: turul_mcp_protocol::prompts::GetPromptParams =
            serde_json::from_value(params)?;
        #[cfg(not(feature = "protocol-2025-11-25"))]
        let get_params: turul_mcp_protocol::prompts::GetPromptRequestParams =
            serde_json::from_value(params)?;

        debug!(
            "Getting prompt: {} with arguments: {:?}",
            get_params.name, get_params.arguments
        );

        // Find the prompt by name
        let prompt = self.prompts.get(&get_params.name).ok_or_else(|| {
            McpError::invalid_param_type("name", "existing prompt name", &get_params.name)
        })?;

        // Validate required arguments against prompt definition (MCP 2025-11-25 compliance)
        if let Some(prompt_arguments) = prompt.arguments() {
            let empty_args = StdHashMap::new();
            let provided_args = get_params.arguments.as_ref().unwrap_or(&empty_args);

            for arg_def in prompt_arguments {
                let is_required = arg_def.required.unwrap_or(false);
                if is_required && !provided_args.contains_key(&arg_def.name) {
                    return Err(McpError::InvalidParameters(format!(
                        "Missing required argument '{}' for prompt '{}'",
                        arg_def.name, get_params.name
                    )));
                }
            }
        }

        // Convert arguments from HashMap<String, String> to HashMap<String, Value> if needed
        let arguments = match get_params.arguments {
            Some(args) => {
                let mut value_args = StdHashMap::new();
                for (key, value) in args {
                    value_args.insert(key, serde_json::Value::String(value));
                }
                value_args
            }
            None => StdHashMap::new(),
        };

        // MRTR retry leg: `McpPrompt::render` has no session parameter, so the
        // retry's inputResponses / requestState ride the render args under
        // reserved io.modelcontextprotocol/* keys (documented on McpPrompt).
        #[cfg(feature = "protocol-2026-07-28")]
        let arguments = {
            let mut arguments = arguments;
            if let Some(ref responses) = get_params.input_responses
                && let Ok(v) = serde_json::to_value(responses)
            {
                arguments.insert("io.modelcontextprotocol/inputResponses".to_string(), v);
            }
            if let Some(ref state) = get_params.request_state {
                arguments.insert(
                    "io.modelcontextprotocol/requestState".to_string(),
                    serde_json::Value::String(state.clone()),
                );
            }
            arguments
        };

        // Generate prompt messages using the prompt implementation
        // Note: MCP 2025-11-25 spec enforces only 'user' and 'assistant' roles via Role enum - no 'system' role
        let messages = match prompt.render(Some(arguments)).await {
            #[cfg(feature = "protocol-2026-07-28")]
            Err(McpError::InputRequired {
                input_requests,
                request_state,
            }) => {
                return input_required_to_result(
                    input_requests,
                    request_state,
                    &get_params.meta.client_capabilities,
                );
            }
            other => other?,
        };

        // Create response with messages
        let mut response = GetPromptResult::new(messages);
        if let Some(desc) = prompt.description() {
            response = response.with_description(desc);
        }

        // Propagate caller-supplied _meta extras from request to response.
        if !request_meta_extra.is_empty() {
            response = response.with_meta(request_meta_extra);
        }

        serde_json::to_value(response).map_err(McpError::from)
    }

    fn supported_methods(&self) -> Vec<String> {
        vec!["prompts/get".to_string()]
    }
}

/// Legacy handler for backward compatibility - use PromptsListHandler instead
pub type PromptsHandler = PromptsListHandler;

/// Import the proper McpPrompt trait from the prompt module
pub use crate::prompt::McpPrompt;

/// Resources list handler for resources/list endpoint only
pub struct ResourcesListHandler {
    resources: HashMap<String, Arc<dyn McpResource>>,
}

impl Default for ResourcesListHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl ResourcesListHandler {
    pub fn new() -> Self {
        Self {
            resources: HashMap::new(),
        }
    }

    pub fn add_resource<R: McpResource + 'static>(mut self, resource: R) -> Self {
        self.resources
            .insert(resource.uri().to_string(), Arc::new(resource));
        self
    }

    pub fn add_resource_arc(mut self, resource: Arc<dyn McpResource>) -> Self {
        self.resources.insert(resource.uri().to_string(), resource);
        self
    }
}

#[async_trait]
impl McpHandler for ResourcesListHandler {
    async fn handle(&self, params: Option<Value>) -> McpResult<Value> {
        use turul_mcp_protocol::meta::Cursor;
        use turul_mcp_protocol::resources::{ListResourcesResult, Resource};

        // Extract limit from raw params before parsing to typed params (MCP extension field)
        // Clamp to 1000 for DoS protection, reject zero
        const DEFAULT_PAGE_SIZE: usize = 50;
        const MAX_PAGE_SIZE: usize = 1000;
        const MIN_PAGE_SIZE: usize = 1;

        let page_size = match extract_limit_from_params(&params) {
            Some(0) => {
                return Err(McpError::InvalidParameters(
                    "limit must be at least 1 (zero would return empty pages forever)".to_string(),
                ));
            }
            Some(n) => n.clamp(MIN_PAGE_SIZE, MAX_PAGE_SIZE),
            None => DEFAULT_PAGE_SIZE,
        };

        let cursor = extract_cursor_from_params(&params);
        let request_meta_extra = extract_request_meta_extra(&params);

        debug!(
            "Listing resources with cursor: {:?}, limit: {}",
            cursor, page_size
        );

        // Convert all resources to descriptors and sort by URI for stable ordering
        let mut all_resources: Vec<Resource> = self
            .resources
            .values()
            .map(|r| resource_to_descriptor(r.as_ref()))
            .collect();

        // Sort by URI to ensure stable pagination ordering (MCP 2025-11-25 requirement)
        all_resources.sort_by(|a, b| a.uri.cmp(&b.uri));

        // Find starting index based on cursor
        let start_index = if let Some(cursor) = &cursor {
            // Cursor contains the last URI from previous page
            let cursor_uri = cursor.as_str();
            // Pagination §Error Handling: "Invalid cursors SHOULD result in an
            // error with code -32602" — a cursor must be one this server
            // issued (the last resource uri of a previously served page).
            if !all_resources.iter().any(|r| r.uri.as_str() == cursor_uri) {
                return Err(McpError::InvalidParameters(format!(
                    "invalid pagination cursor {:?}",
                    cursor_uri
                )));
            }

            // Find the position after the cursor URI
            all_resources
                .iter()
                .position(|r| r.uri.as_str() > cursor_uri)
                .unwrap_or(all_resources.len())
        } else {
            0 // No cursor = start from beginning
        };

        // Calculate end index for this page
        let end_index = std::cmp::min(start_index + page_size, all_resources.len());

        // Extract page of resources
        let page_resources: Vec<Resource> = all_resources[start_index..end_index].to_vec();

        // Determine if there are more resources after this page
        let has_more = end_index < all_resources.len();

        // Generate next cursor if there are more resources
        let next_cursor = if has_more {
            // Cursor should be the URI of the last item in current page
            page_resources.last().map(|r| Cursor::new(&r.uri))
        } else {
            None
        };

        debug!(
            "Resource pagination: start={}, end={}, page_size={}, has_more={}, next_cursor={:?}",
            start_index,
            end_index,
            page_resources.len(),
            has_more,
            next_cursor
        );

        let mut base_response = ListResourcesResult::new(page_resources);

        if let Some(ref cursor) = next_cursor {
            base_response = base_response.with_next_cursor(cursor.clone());
        }

        let total = Some(all_resources.len() as u64);

        paginate_list_response(
            base_response,
            next_cursor,
            total,
            has_more,
            request_meta_extra,
        )
    }

    fn supported_methods(&self) -> Vec<String> {
        vec!["resources/list".to_string()]
    }
}

/// Resources read handler for resources/read endpoint only
pub struct ResourcesReadHandler {
    resources: HashMap<String, Arc<dyn McpResource>>,
    uri_registry: Arc<crate::uri_template::UriTemplateRegistry>,
    security_middleware: Option<Arc<crate::security::SecurityMiddleware>>,
}

impl Default for ResourcesReadHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl ResourcesReadHandler {
    pub fn new() -> Self {
        Self {
            resources: HashMap::new(),
            uri_registry: Arc::new(crate::uri_template::UriTemplateRegistry::new()),
            security_middleware: Some(Arc::new(crate::security::SecurityMiddleware::default())),
        }
    }

    /// Create handler with custom security middleware
    pub fn with_security(mut self, middleware: Arc<crate::security::SecurityMiddleware>) -> Self {
        self.security_middleware = Some(middleware);
        self
    }

    /// Create handler without security middleware (for testing or trusted environments)
    pub fn without_security(mut self) -> Self {
        self.security_middleware = None;
        self
    }

    pub fn add_resource<R: McpResource + 'static>(mut self, resource: R) -> Self {
        self.resources
            .insert(resource.uri().to_string(), Arc::new(resource));
        self
    }

    pub fn add_resource_arc(mut self, resource: Arc<dyn McpResource>) -> Self {
        self.resources.insert(resource.uri().to_string(), resource);
        self
    }

    /// Add a dynamic resource with URI template support
    pub fn add_template_resource<R: McpResource + 'static>(
        mut self,
        template: crate::uri_template::UriTemplate,
        resource: R,
    ) -> Self {
        // Register the template in the registry
        Arc::get_mut(&mut self.uri_registry)
            .expect("URI registry should not be shared yet")
            .register(template.clone());

        // Store the resource using the template pattern as key
        let pattern = template.pattern().to_string();
        self.resources.insert(pattern, Arc::new(resource));
        self
    }

    /// Add a dynamic resource with URI template support (Arc version)
    pub fn add_template_resource_arc(
        mut self,
        template: crate::uri_template::UriTemplate,
        resource: Arc<dyn McpResource>,
    ) -> Self {
        // Register the template in the registry
        Arc::get_mut(&mut self.uri_registry)
            .expect("URI registry should not be shared yet")
            .register(template.clone());

        // Store the resource using the template pattern as key
        let pattern = template.pattern().to_string();
        self.resources.insert(pattern, resource);
        self
    }
}

#[async_trait]
impl McpHandler for ResourcesReadHandler {
    async fn handle(&self, params: Option<Value>) -> McpResult<Value> {
        // Delegate to handle_with_session with no session
        self.handle_with_session(params, None).await
    }

    async fn handle_with_session(
        &self,
        params: Option<Value>,
        session: Option<SessionContext>,
    ) -> McpResult<Value> {
        use turul_mcp_protocol::resources::ReadResourceResult;

        // Security validation
        if let Some(security_middleware) = &self.security_middleware {
            security_middleware.validate_request(
                "resources/read",
                params.as_ref(),
                session.as_ref(),
            )?;
        }

        // Parse read resource parameters
        let params = params.ok_or_else(|| McpError::missing_param("ReadResourceParams"))?;
        #[cfg(feature = "protocol-2025-11-25")]
        let read_params: turul_mcp_protocol::resources::ReadResourceParams =
            serde_json::from_value(params)?;
        #[cfg(not(feature = "protocol-2025-11-25"))]
        let read_params: turul_mcp_protocol::resources::ReadResourceRequestParams =
            serde_json::from_value(params)?;

        debug!("Reading resource with URI: {}", read_params.uri);

        // MRTR retry leg: surface the client's inputResponses / requestState to
        // the provider via the session extensions (SessionContext::input_responses).
        #[cfg(feature = "protocol-2026-07-28")]
        let session = {
            let mut session = session;
            if let Some(ref mut ctx) = session {
                if let Some(ref responses) = read_params.input_responses
                    && let Ok(v) = serde_json::to_value(responses)
                {
                    ctx.extensions
                        .insert("mcp:mrtr:inputResponses".to_string(), v);
                }
                if let Some(ref state) = read_params.request_state {
                    ctx.extensions.insert(
                        "mcp:mrtr:requestState".to_string(),
                        Value::String(state.clone()),
                    );
                }
                if let Some(ref token) = read_params.meta.progress_token
                    && let Ok(v) = serde_json::to_value(token)
                {
                    ctx.extensions.insert("mcp:progressToken".to_string(), v);
                }
            }
            session
        };

        // Additional security validation for the specific URI
        if let Some(security_middleware) = &self.security_middleware {
            // Re-validate the URI after parsing (defense in depth)
            let uri_params = serde_json::json!({"uri": read_params.uri});
            security_middleware.validate_request(
                "resources/read",
                Some(&uri_params),
                session.as_ref(),
            )?;
        }

        // First try to match against URI templates
        if let Some(template) = self.uri_registry.find_matching(&read_params.uri) {
            debug!("Found matching URI template: {}", template.pattern());

            // Extract variables from the URI
            let template_vars = template.extract(&read_params.uri)?;
            debug!("Extracted template variables: {:?}", template_vars);

            // Find the resource by template pattern
            let resource = self.resources.get(template.pattern()).ok_or_else(|| {
                McpError::invalid_param_type(
                    "template",
                    "registered template pattern",
                    template.pattern(),
                )
            })?;

            // Create enhanced params with template variables
            let mut enhanced_params = serde_json::to_value(&read_params)?;
            if let Some(params_obj) = enhanced_params.as_object_mut() {
                params_obj.insert(
                    "template_variables".to_string(),
                    serde_json::to_value(template_vars)?,
                );
            }

            let contents = match resource.read(Some(enhanced_params), session.as_ref()).await {
                #[cfg(feature = "protocol-2026-07-28")]
                Err(McpError::InputRequired {
                    input_requests,
                    request_state,
                }) => {
                    return input_required_to_result(
                        input_requests,
                        request_state,
                        &read_params.meta.client_capabilities,
                    );
                }
                other => other?,
            };

            // Validate content before returning
            if let Some(security_middleware) = &self.security_middleware {
                for content in &contents {
                    match content {
                        turul_mcp_protocol::resources::ResourceContent::Text(text_content) => {
                            if let Some(mime_type) = &text_content.mime_type {
                                security_middleware
                                    .resource_access_control()
                                    .validate_mime_type(mime_type)?;
                            }
                            let size = text_content.text.len() as u64;
                            security_middleware
                                .resource_access_control()
                                .validate_size(size)?;
                        }
                        turul_mcp_protocol::resources::ResourceContent::Blob(blob_content) => {
                            if let Some(mime_type) = &blob_content.mime_type {
                                security_middleware
                                    .resource_access_control()
                                    .validate_mime_type(mime_type)?;
                            }
                            let size = blob_content.blob.len() as u64;
                            security_middleware
                                .resource_access_control()
                                .validate_size(size)?;
                        }
                    }
                }
            }

            // "Binary data MUST be properly encoded" — reject invalid base64
            // before it ships as a wire payload (provider bug, not client's).
            for content in &contents {
                if let turul_mcp_protocol::resources::ResourceContent::Blob(blob) = content
                    && base64::Engine::decode(
                        &base64::engine::general_purpose::STANDARD,
                        blob.blob.as_bytes(),
                    )
                    .is_err()
                {
                    return Err(McpError::ToolExecutionError(format!(
                        "resource {} returned blob contents that are not valid base64",
                        blob.uri
                    )));
                }
            }
            let response = ReadResourceResult::new(contents);
            return serde_json::to_value(response).map_err(McpError::from);
        }

        // Fall back to exact URI matching
        let resource = self.resources.get(&read_params.uri).ok_or_else(|| {
            McpError::invalid_param_type(
                "uri",
                "existing resource URI or template pattern",
                &read_params.uri,
            )
        })?;

        // Call the resource's read method with original params
        let params = Some(serde_json::to_value(&read_params)?);
        let contents = match resource.read(params, session.as_ref()).await {
            #[cfg(feature = "protocol-2026-07-28")]
            Err(McpError::InputRequired {
                input_requests,
                request_state,
            }) => {
                return input_required_to_result(
                    input_requests,
                    request_state,
                    &read_params.meta.client_capabilities,
                );
            }
            other => other?,
        };

        // Validate content before returning
        if let Some(security_middleware) = &self.security_middleware {
            for content in &contents {
                match content {
                    turul_mcp_protocol::resources::ResourceContent::Text(text_content) => {
                        if let Some(mime_type) = &text_content.mime_type {
                            security_middleware
                                .resource_access_control()
                                .validate_mime_type(mime_type)?;
                        }
                        let size = text_content.text.len() as u64;
                        security_middleware
                            .resource_access_control()
                            .validate_size(size)?;
                    }
                    turul_mcp_protocol::resources::ResourceContent::Blob(blob_content) => {
                        if let Some(mime_type) = &blob_content.mime_type {
                            security_middleware
                                .resource_access_control()
                                .validate_mime_type(mime_type)?;
                        }
                        let size = blob_content.blob.len() as u64;
                        security_middleware
                            .resource_access_control()
                            .validate_size(size)?;
                    }
                }
            }
        }

        // "Binary data MUST be properly encoded" — reject invalid base64
        // before it ships as a wire payload (provider bug, not client's).
        for content in &contents {
            if let turul_mcp_protocol::resources::ResourceContent::Blob(blob) = content
                && base64::Engine::decode(
                    &base64::engine::general_purpose::STANDARD,
                    blob.blob.as_bytes(),
                )
                .is_err()
            {
                return Err(McpError::ToolExecutionError(format!(
                    "resource {} returned blob contents that are not valid base64",
                    blob.uri
                )));
            }
        }
        let response = ReadResourceResult::new(contents);
        serde_json::to_value(response).map_err(McpError::from)
    }

    fn supported_methods(&self) -> Vec<String> {
        vec!["resources/read".to_string()]
    }
}

/// Legacy handler for backward compatibility - use ResourcesListHandler instead
pub type ResourcesHandler = ResourcesListHandler;

/// Logging handler for logging/setLevel endpoint
#[cfg(feature = "protocol-2025-11-25")]
pub struct LoggingHandler;

#[cfg(feature = "protocol-2025-11-25")]
#[async_trait]
impl McpHandler for LoggingHandler {
    async fn handle(&self, params: Option<Value>) -> McpResult<Value> {
        // Fallback for when no session is provided
        use turul_mcp_protocol::logging::SetLevelParams;

        if let Some(params) = params {
            let set_level_params: SetLevelParams = serde_json::from_value(params)?;

            // Without session context, just log the request but can't store per-session
            tracing::warn!(
                "LoggingHandler.handle() called without session context - cannot store level per-session"
            );
            tracing::info!("Would set log level to: {:?}", set_level_params.level);

            // MCP logging/setLevel doesn't return data, just success
            serde_json::to_value(json!({})).map_err(McpError::from)
        } else {
            Err(McpError::missing_param("SetLevelParams"))
        }
    }

    async fn handle_with_session(
        &self,
        params: Option<Value>,
        session: Option<SessionContext>,
    ) -> McpResult<Value> {
        use turul_mcp_protocol::logging::SetLevelParams;

        // Parse params - returns InvalidParams (-32602) if fails
        let params = params.ok_or_else(|| McpError::missing_param("params"))?;

        let set_level_params: SetLevelParams = serde_json::from_value(params)?;

        // Require session - returns configuration error if missing
        let session_ctx = session
            .ok_or_else(|| McpError::configuration("Session required for logging/setLevel"))?;

        // Check initialization - returns configuration error if not initialized
        if !(session_ctx.is_initialized)().await {
            return Err(McpError::configuration(
                "Session must be initialized before setting logging level",
            ));
        }

        // Set the level
        session_ctx.set_logging_level(set_level_params.level).await;

        tracing::debug!(
            "🎯 Set logging level for session {}: {:?}",
            session_ctx.session_id,
            set_level_params.level
        );

        // Verify persistence - returns configuration error if fails
        let stored_level = session_ctx.get_logging_level().await;
        if stored_level != set_level_params.level {
            return Err(McpError::configuration(
                "Failed to persist logging level in session storage",
            ));
        }

        // Send confirmation notification
        session_ctx
            .notify_log(
                turul_mcp_protocol::logging::LoggingLevel::Info,
                serde_json::json!(format!(
                    "Logging level changed to: {:?}",
                    set_level_params.level
                )),
                None,
                None,
            )
            .await;

        // Success returns empty object per MCP spec
        Ok(json!({}))
    }

    fn supported_methods(&self) -> Vec<String> {
        vec!["logging/setLevel".to_string()]
    }
}

/// Roots handler for roots/list endpoint
// `Root` is deprecated-but-present in 2026-07-28 (SEP-2577); roots remain a valid feature.
#[allow(deprecated)]
pub struct RootsHandler {
    roots: Vec<turul_mcp_protocol::roots::Root>,
}

impl Default for RootsHandler {
    fn default() -> Self {
        Self::new()
    }
}

#[allow(deprecated)]
impl RootsHandler {
    pub fn new() -> Self {
        Self { roots: Vec::new() }
    }

    pub fn add_root(mut self, root: turul_mcp_protocol::roots::Root) -> Self {
        self.roots.push(root);
        self
    }
}

#[async_trait]
#[allow(deprecated)]
impl McpHandler for RootsHandler {
    async fn handle(&self, params: Option<Value>) -> McpResult<Value> {
        use turul_mcp_protocol::meta::Cursor;
        use turul_mcp_protocol::roots::ListRootsResult;

        let cursor = extract_cursor_from_params(&params);

        debug!("Listing roots with cursor: {:?}", cursor);

        // Sort roots by URI for stable ordering
        let mut all_roots = self.roots.clone();
        all_roots.sort_by(|a, b| a.uri.cmp(&b.uri));

        // Implement cursor-based pagination
        const DEFAULT_PAGE_SIZE: usize = 50; // MCP suggested default
        let page_size = DEFAULT_PAGE_SIZE;

        // Find starting index based on cursor
        let start_index = if let Some(cursor) = &cursor {
            // Cursor contains the last URI from previous page
            let cursor_uri = cursor.as_str();
            // Pagination §Error Handling: "Invalid cursors SHOULD result in an
            // error with code -32602" — a cursor must be one this server
            // issued (the last root uri of a previously served page).
            if !all_roots.iter().any(|r| r.uri.as_str() == cursor_uri) {
                return Err(McpError::InvalidParameters(format!(
                    "invalid pagination cursor {:?}",
                    cursor_uri
                )));
            }

            // Find the position after the cursor URI
            all_roots
                .iter()
                .position(|r| r.uri.as_str() > cursor_uri)
                .unwrap_or(all_roots.len())
        } else {
            0 // No cursor = start from beginning
        };

        // Calculate end index for this page
        let end_index = std::cmp::min(start_index + page_size, all_roots.len());

        // Extract page of roots
        let page_roots = all_roots[start_index..end_index].to_vec();

        // Determine if there are more roots after this page
        let has_more = end_index < all_roots.len();

        // Generate next cursor if there are more roots
        let next_cursor = if has_more {
            // Cursor should be the URI of the last item in current page
            page_roots.last().map(|r| Cursor::new(&r.uri))
        } else {
            None
        };

        debug!(
            "Root pagination: start={}, end={}, page_size={}, has_more={}, next_cursor={:?}",
            start_index,
            end_index,
            page_roots.len(),
            has_more,
            next_cursor
        );

        let base_response = ListRootsResult::new(page_roots);

        // ListRootsResult has no nextCursor field — roots are not paginatable per spec.

        let total = Some(all_roots.len() as u64);

        paginate_list_response(base_response, next_cursor, total, has_more, HashMap::new())
    }

    fn supported_methods(&self) -> Vec<String> {
        vec!["roots/list".to_string()]
    }
}

/// Sampling handler for sampling/createMessage endpoint (default/mock implementation)
#[cfg(feature = "protocol-2025-11-25")]
pub struct SamplingHandler;

#[cfg(feature = "protocol-2025-11-25")]
#[async_trait]
impl McpHandler for SamplingHandler {
    async fn handle(&self, params: Option<Value>) -> McpResult<Value> {
        use turul_mcp_protocol::meta::{ProgressResponse, ProgressToken};
        use turul_mcp_protocol::sampling::CreateMessageResult;

        // Parse progress token if provided
        let progress_token = params
            .as_ref()
            .and_then(|p| p.get("progressToken"))
            .and_then(|t| t.as_str())
            .map(ProgressToken::from);

        // Default implementation - return a simple message
        let base_response = CreateMessageResult {
            role: turul_mcp_protocol::sampling::Role::Assistant,
            content: turul_mcp_protocol::prompts::ContentBlock::text(
                "This is a sample message generated by the MCP server",
            ),
            model: "mock-model-v1".to_string(),
            stop_reason: Some("stop".to_string()),
            meta: None,
        };

        // Add progress metadata for message generation operations
        // In a real implementation, progress would track token generation steps
        let progress_response = ProgressResponse::with_progress(
            base_response,
            progress_token.or_else(|| Some(ProgressToken::new("sampling-default"))),
            1.0, // Complete since we're returning immediately
            Some(1),
            Some(1),
        );

        serde_json::to_value(progress_response).map_err(McpError::from)
    }

    fn supported_methods(&self) -> Vec<String> {
        vec!["sampling/createMessage".to_string()]
    }
}

/// Sampling handler that dispatches to registered sampling providers
///
/// This handler validates requests and dispatches to actual McpSampling
/// implementations registered via .sampling_provider(). It replaces the
/// default SamplingHandler when providers are configured.
#[cfg(feature = "protocol-2025-11-25")]
pub struct ProvidedSamplingHandler {
    providers: HashMap<String, Arc<dyn crate::McpSampling>>,
}

#[cfg(feature = "protocol-2025-11-25")]
impl ProvidedSamplingHandler {
    pub fn new(providers: HashMap<String, Arc<dyn crate::McpSampling>>) -> Self {
        Self { providers }
    }
}

#[cfg(feature = "protocol-2025-11-25")]
#[async_trait]
impl McpHandler for ProvidedSamplingHandler {
    async fn handle(&self, params: Option<Value>) -> McpResult<Value> {
        use turul_mcp_protocol::meta::{ProgressResponse, ProgressToken};
        use turul_mcp_protocol::sampling::{CreateMessageParams, CreateMessageRequest};

        // Extract progress token if provided
        let progress_token = params
            .as_ref()
            .and_then(|p| p.get("progressToken"))
            .and_then(|t| t.as_str())
            .map(ProgressToken::from);

        // Parse params into CreateMessageParams
        let message_params: CreateMessageParams =
            serde_json::from_value(params.ok_or_else(|| McpError::missing_param("params"))?)?;

        // Construct full CreateMessageRequest for the provider
        let request = CreateMessageRequest {
            method: "sampling/createMessage".to_string(),
            params: message_params,
        };

        // Select provider - use first for now (can enhance with can_handle/priority later)
        let provider = self
            .providers
            .values()
            .next()
            .ok_or_else(|| McpError::configuration("No sampling provider available"))?;

        // Validate request - THIS is where maxTokens=0 gets caught!
        provider.validate_request(&request).await?;

        // Generate message using the provider
        let result = provider.sample(request).await?;

        // Wrap in progress response for consistency with default handler
        let progress_response = ProgressResponse::with_progress(
            result,
            progress_token.or_else(|| Some(ProgressToken::new("sampling-provided"))),
            1.0, // Complete since we're returning immediately
            Some(1),
            Some(1),
        );

        serde_json::to_value(progress_response).map_err(McpError::from)
    }

    fn supported_methods(&self) -> Vec<String> {
        vec!["sampling/createMessage".to_string()]
    }
}

/// Resource templates handler for resources/templates/list endpoint
pub struct ResourceTemplatesHandler {
    templates: Vec<(crate::uri_template::UriTemplate, Arc<dyn McpResource>)>,
}

impl Default for ResourceTemplatesHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl ResourceTemplatesHandler {
    pub fn new() -> Self {
        Self {
            templates: Vec::new(),
        }
    }

    pub fn with_templates(
        mut self,
        templates: Vec<(crate::uri_template::UriTemplate, Arc<dyn McpResource>)>,
    ) -> Self {
        self.templates = templates;
        self
    }
}

#[async_trait]
impl McpHandler for ResourceTemplatesHandler {
    async fn handle(&self, params: Option<Value>) -> McpResult<Value> {
        use turul_mcp_protocol::meta::Cursor;

        // Extract limit from raw params before parsing to typed params (MCP extension field)
        // Clamp to 1000 for DoS protection, reject zero
        const DEFAULT_PAGE_SIZE: usize = 50;
        const MAX_PAGE_SIZE: usize = 1000;
        const MIN_PAGE_SIZE: usize = 1;

        let page_size = match extract_limit_from_params(&params) {
            Some(0) => {
                return Err(McpError::InvalidParameters(
                    "limit must be at least 1 (zero would return empty pages forever)".to_string(),
                ));
            }
            Some(n) => n.clamp(MIN_PAGE_SIZE, MAX_PAGE_SIZE),
            None => DEFAULT_PAGE_SIZE,
        };

        let cursor = extract_cursor_from_params(&params);
        let request_meta_extra = extract_request_meta_extra(&params);
        debug!(
            "Listing resource templates with cursor: {:?}, limit: {}",
            cursor, page_size
        );

        tracing::info!(
            "Resource templates list requested - {} templates registered",
            self.templates.len()
        );

        use turul_mcp_protocol::resources::{ListResourceTemplatesResult, ResourceTemplate};

        // Convert registered templates to ResourceTemplate objects and sort by template for stable ordering
        let mut all_templates: Vec<ResourceTemplate> = self
            .templates
            .iter()
            .map(|(uri_template, resource)| {
                let template_name = resource.name();
                let mut template = ResourceTemplate::new(template_name, uri_template.pattern());
                if let Some(desc) = resource.description() {
                    template = template.with_description(desc);
                }
                // Add MIME type if the resource provides it
                if let Some(mime_type) = resource.mime_type() {
                    template = template.with_mime_type(mime_type);
                }
                template
            })
            .collect();

        // Sort by uri_template to ensure stable pagination ordering (MCP 2025-11-25 requirement)
        all_templates.sort_by(|a, b| a.uri_template.cmp(&b.uri_template));

        // Find starting index based on cursor
        let start_index = if let Some(cursor) = &cursor {
            // Cursor contains the last uri_template from previous page
            let cursor_template = cursor.as_str();
            // Pagination §Error Handling: "Invalid cursors SHOULD result in an
            // error with code -32602" — a cursor must be one this server
            // issued (the last template uri of a previously served page).
            if !all_templates
                .iter()
                .any(|t| t.uri_template.as_str() == cursor_template)
            {
                return Err(McpError::InvalidParameters(format!(
                    "invalid pagination cursor {:?}",
                    cursor_template
                )));
            }

            // Find the position after the cursor template
            all_templates
                .iter()
                .position(|t| t.uri_template.as_str() > cursor_template)
                .unwrap_or(all_templates.len())
        } else {
            0 // No cursor = start from beginning
        };

        // Calculate end index for this page
        let end_index = std::cmp::min(start_index + page_size, all_templates.len());

        // Extract the page
        let page_templates = all_templates[start_index..end_index].to_vec();

        // Calculate pagination metadata
        let total = Some(all_templates.len() as u64);
        let has_more = end_index < all_templates.len();
        let next_cursor = if has_more {
            // Next cursor is the last template name in this page
            page_templates.last().map(|t| Cursor::new(&t.uri_template))
        } else {
            None // No more pages
        };

        debug!(
            "Resource template pagination: page_size={}, has_more={}, next_cursor={:?}",
            page_templates.len(),
            has_more,
            next_cursor
        );

        let mut base_response = ListResourceTemplatesResult::new(page_templates);

        if let Some(ref cursor) = next_cursor {
            base_response = base_response.with_next_cursor(cursor.clone());
        }

        paginate_list_response(
            base_response,
            next_cursor,
            total,
            has_more,
            request_meta_extra,
        )
    }

    fn supported_methods(&self) -> Vec<String> {
        vec!["resources/templates/list".to_string()]
    }
}

/// Trait for custom elicitation UI implementations
///
/// This trait enables server implementations to provide custom user interfaces
/// for collecting structured input from users via JSON Schema-defined forms.
#[cfg(feature = "protocol-2025-11-25")]
#[async_trait]
pub trait ElicitationProvider: Send + Sync {
    /// Present an elicitation request to the user and return their response
    async fn elicit(
        &self,
        request: &turul_mcp_protocol::elicitation::ElicitCreateRequest,
    ) -> McpResult<turul_mcp_protocol::elicitation::ElicitResult>;

    /// Check if this provider can handle a specific elicitation schema
    fn can_handle(&self, _request: &turul_mcp_protocol::elicitation::ElicitCreateRequest) -> bool {
        // Default implementation accepts all requests
        true
    }
}

/// Default console-based elicitation provider for demonstration
#[cfg(feature = "protocol-2025-11-25")]
pub struct MockElicitationProvider;

#[cfg(feature = "protocol-2025-11-25")]
#[async_trait]
impl ElicitationProvider for MockElicitationProvider {
    async fn elicit(
        &self,
        request: &turul_mcp_protocol::elicitation::ElicitCreateRequest,
    ) -> McpResult<turul_mcp_protocol::elicitation::ElicitResult> {
        use turul_mcp_protocol::elicitation::ElicitResult;

        // Mock implementation based on message content
        let mut mock_data = std::collections::HashMap::new();
        mock_data.insert("mock_response".to_string(), serde_json::json!(true));
        mock_data.insert(
            "message".to_string(),
            serde_json::json!(&request.params.message),
        );
        mock_data.insert(
            "note".to_string(),
            serde_json::json!("This is a mock elicitation response for testing"),
        );

        // Simple mock logic based on message content
        match request.params.message.as_str() {
            msg if msg.contains("cancel") => Ok(ElicitResult::cancel()),
            msg if msg.contains("decline") => Ok(ElicitResult::decline()),
            _ => Ok(ElicitResult::accept(mock_data)),
        }
    }

    fn can_handle(&self, _request: &turul_mcp_protocol::elicitation::ElicitCreateRequest) -> bool {
        true // Mock provider handles all requests
    }
}

/// Elicitation handler for elicitation/create endpoint
#[cfg(feature = "protocol-2025-11-25")]
pub struct ElicitationHandler {
    provider: Arc<dyn ElicitationProvider>,
}

#[cfg(feature = "protocol-2025-11-25")]
impl ElicitationHandler {
    pub fn new(provider: Arc<dyn ElicitationProvider>) -> Self {
        Self { provider }
    }

    pub fn with_mock_provider() -> Self {
        Self::new(Arc::new(MockElicitationProvider))
    }
}

#[cfg(feature = "protocol-2025-11-25")]
#[async_trait]
impl McpHandler for ElicitationHandler {
    async fn handle(&self, params: Option<Value>) -> McpResult<Value> {
        use turul_mcp_protocol::elicitation::ElicitCreateParams;

        if let Some(params) = params {
            let request_params: ElicitCreateParams = serde_json::from_value(params)?;

            tracing::info!("Processing elicitation request: {}", request_params.message);

            // Create full request object from parameters
            use turul_mcp_protocol::elicitation::ElicitCreateRequest;
            let create_request = ElicitCreateRequest {
                method: "elicitation/create".to_string(),
                params: request_params.clone(),
            };

            // Check if provider can handle this request
            if !self.provider.can_handle(&create_request) {
                let error_response = turul_mcp_protocol::elicitation::ElicitResult::cancel();
                return serde_json::to_value(error_response).map_err(McpError::from);
            }

            // Delegate to the elicitation provider
            let result = self.provider.elicit(&create_request).await?;

            serde_json::to_value(result).map_err(McpError::from)
        } else {
            Err(McpError::missing_param("ElicitCreateParams"))
        }
    }

    fn supported_methods(&self) -> Vec<String> {
        vec!["elicitation/create".to_string()]
    }
}

use crate::session::SessionManager;

/// Generic notifications handler for most notification endpoints
pub struct NotificationsHandler;

#[async_trait]
impl McpHandler for NotificationsHandler {
    async fn handle(&self, params: Option<Value>) -> McpResult<Value> {
        // Notifications are typically one-way, so we just log them
        tracing::info!("Received notification: {:?}", params);
        Ok(Value::Null)
    }

    fn supported_methods(&self) -> Vec<String> {
        vec![
            "notifications/message".to_string(),
            "notifications/progress".to_string(),
            "notifications/cancelled".to_string(),
            // MCP 2025-11-25 spec (underscore)
            "notifications/resources/list_changed".to_string(),
            "notifications/tools/list_changed".to_string(),
            "notifications/prompts/list_changed".to_string(),
            "notifications/roots/list_changed".to_string(),
            // Legacy compat (camelCase) — accepted but NOT spec-compliant
            "notifications/resources/listChanged".to_string(),
            "notifications/tools/listChanged".to_string(),
            "notifications/prompts/listChanged".to_string(),
            "notifications/roots/listChanged".to_string(),
            "notifications/resources/updated".to_string(),
        ]
    }
}

/// Special handler for notifications/initialized that manages session lifecycle
pub struct InitializedNotificationHandler {
    session_manager: Arc<SessionManager>,
}

impl InitializedNotificationHandler {
    pub fn new(session_manager: Arc<SessionManager>) -> Self {
        Self { session_manager }
    }
}

#[async_trait]
impl McpHandler for InitializedNotificationHandler {
    async fn handle(&self, _params: Option<Value>) -> McpResult<Value> {
        // This should not be called directly without session context
        tracing::warn!("notifications/initialized received without session context");
        Ok(Value::Null)
    }

    async fn handle_with_session(
        &self,
        params: Option<Value>,
        session: Option<SessionContext>,
    ) -> McpResult<Value> {
        tracing::debug!("📨 Received notifications/initialized: {:?}", params);

        if let Some(session_ctx) = session {
            tracing::debug!(
                "🔄 Processing notifications/initialized for session: {}",
                session_ctx.session_id
            );

            // Check if session is already initialized
            if self
                .session_manager
                .is_session_initialized(&session_ctx.session_id)
                .await
            {
                tracing::info!(
                    "ℹ️ Session {} already initialized, ignoring duplicate notifications/initialized",
                    session_ctx.session_id
                );
                return Ok(Value::Null);
            }

            // Get client info from session state (it should have been stored during the initialize request)
            let client_info_value = self
                .session_manager
                .get_session_state(&session_ctx.session_id, "client_info")
                .await;
            let capabilities_value = self
                .session_manager
                .get_session_state(&session_ctx.session_id, "client_capabilities")
                .await;
            let negotiated_version_value = self
                .session_manager
                .get_session_state(&session_ctx.session_id, "negotiated_version")
                .await;

            if let (
                Some(client_info_value),
                Some(capabilities_value),
                Some(negotiated_version_value),
            ) = (
                client_info_value,
                capabilities_value,
                negotiated_version_value,
            ) {
                // Deserialize the stored values
                use turul_mcp_protocol::{ClientCapabilities, Implementation, McpVersion};

                if let (Ok(client_info), Ok(client_capabilities), Ok(negotiated_version)) = (
                    serde_json::from_value::<Implementation>(client_info_value),
                    serde_json::from_value::<ClientCapabilities>(capabilities_value),
                    serde_json::from_value::<McpVersion>(negotiated_version_value),
                ) {
                    // Mark session as initialized now that we received the notification
                    if let Err(e) = self
                        .session_manager
                        .initialize_session_with_version(
                            &session_ctx.session_id,
                            client_info,
                            client_capabilities,
                            negotiated_version,
                        )
                        .await
                    {
                        tracing::error!(
                            "❌ Failed to initialize session {}: {}",
                            session_ctx.session_id,
                            e
                        );
                        return Err(turul_mcp_protocol::McpError::configuration(&format!(
                            "Failed to initialize session: {}",
                            e
                        )));
                    }

                    tracing::debug!(
                        "✅ Session {} successfully initialized after receiving notifications/initialized",
                        session_ctx.session_id
                    );
                } else {
                    tracing::error!(
                        "❌ Failed to deserialize stored client info/capabilities/version for session {}",
                        session_ctx.session_id
                    );
                    return Err(turul_mcp_protocol::McpError::configuration(
                        "Failed to deserialize stored client info",
                    ));
                }
            } else {
                tracing::error!(
                    "❌ Missing stored client info/capabilities/version for session {}",
                    session_ctx.session_id
                );
                return Err(turul_mcp_protocol::McpError::configuration(
                    "Missing stored client info - session must call initialize first",
                ));
            }
        } else {
            tracing::warn!("⚠️ notifications/initialized received without session context");
        }

        Ok(Value::Null)
    }

    fn supported_methods(&self) -> Vec<String> {
        vec!["notifications/initialized".to_string()]
    }
}
