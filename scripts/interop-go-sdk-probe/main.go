// Command interop-go-sdk-probe drives the official MCP Go SDK client (peer,
// no turul code in this half of the loop) against a local logging proxy in
// front of the turul interop-fixture-server, and asserts on the bytes the
// proxy captured — never on the SDK's self-report.
//
// Peer: github.com/modelcontextprotocol/go-sdk v1.7.0.
package main

import (
	"bytes"
	"context"
	"encoding/json"
	"flag"
	"fmt"
	"io"
	"net/http"
	"os"
	"strings"
	"sync"
	"time"

	"github.com/modelcontextprotocol/go-sdk/mcp"
)

// captured is one HTTP round trip the proxy relayed between the SDK client
// and the turul server.
type captured struct {
	rpcMethod    string // JSON-RPC "method" from the request body ("" for GET)
	reqProtoVer  string
	reqMcpMethod string
	reqMcpName   string
	reqSessionID string
	respStatus   int
	respBody     []byte
}

var (
	mu   sync.Mutex
	caps []*captured
)

func record(c *captured) {
	mu.Lock()
	defer mu.Unlock()
	caps = append(caps, c)
}

func snapshot() []*captured {
	mu.Lock()
	defer mu.Unlock()
	out := make([]*captured, len(caps))
	copy(out, caps)
	return out
}

// proxyHandler forwards every request to upstream verbatim, byte for byte,
// after recording the headers and bodies that matter for the 2026-07-28
// contract. The response returned to the SDK client is untouched.
func proxyHandler(upstream string) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		if r.Method != http.MethodPost {
			// A 2026-07-28 stateless client must never open the legacy
			// standalone GET SSE stream (removed under SEP-2575) or send a
			// session-termination DELETE (no session was ever established).
			// Record the surprise instead of silently accepting it.
			record(&captured{rpcMethod: r.Method + " " + r.URL.Path, respStatus: http.StatusMethodNotAllowed})
			w.WriteHeader(http.StatusMethodNotAllowed)
			return
		}

		body, err := io.ReadAll(r.Body)
		if err != nil {
			http.Error(w, err.Error(), http.StatusInternalServerError)
			return
		}
		var parsed struct {
			Method string `json:"method"`
		}
		_ = json.Unmarshal(body, &parsed)

		c := &captured{
			rpcMethod:    parsed.Method,
			reqProtoVer:  r.Header.Get("MCP-Protocol-Version"),
			reqMcpMethod: r.Header.Get("Mcp-Method"),
			reqMcpName:   r.Header.Get("Mcp-Name"),
			reqSessionID: r.Header.Get("Mcp-Session-Id"),
		}

		req, err := http.NewRequest(http.MethodPost, upstream, bytes.NewReader(body))
		if err != nil {
			http.Error(w, err.Error(), http.StatusInternalServerError)
			return
		}
		for k, vv := range r.Header {
			lk := strings.ToLower(k)
			if lk == "host" || lk == "content-length" {
				continue
			}
			for _, v := range vv {
				req.Header.Add(k, v)
			}
		}

		resp, err := http.DefaultClient.Do(req)
		if err != nil {
			c.respStatus = http.StatusBadGateway
			record(c)
			http.Error(w, err.Error(), http.StatusBadGateway)
			return
		}
		defer resp.Body.Close()
		respBody, err := io.ReadAll(resp.Body)
		if err != nil {
			c.respStatus = http.StatusBadGateway
			record(c)
			http.Error(w, err.Error(), http.StatusBadGateway)
			return
		}
		c.respStatus = resp.StatusCode
		c.respBody = respBody
		record(c)

		for k, vv := range resp.Header {
			for _, v := range vv {
				w.Header().Add(k, v)
			}
		}
		w.WriteHeader(resp.StatusCode)
		_, _ = w.Write(respBody)
	}
}

// cacheableMethods is the exact set of J1+J2 methods whose result type
// extends CacheableResult in schema/schema.ts (grepped, not assumed):
// DiscoverResult, ListToolsResult, ListResourcesResult,
// ListResourceTemplatesResult, ListPromptsResult, ReadResourceResult.
// GetPromptResult and CompleteResult do not extend it.
var cacheableMethods = map[string]bool{
	"server/discover":          true,
	"tools/list":               true,
	"resources/list":           true,
	"resources/read":           true,
	"resources/templates/list": true,
	"prompts/list":             true,
}

var allMethods = []string{
	"server/discover", "tools/list", "tools/call",
	"resources/list", "resources/read", "resources/templates/list",
	"prompts/list", "prompts/get", "completion/complete",
}

func main() {
	proxyPort := flag.String("proxy-port", "", "local proxy port the SDK client connects to")
	upstreamPort := flag.String("upstream-port", "", "turul interop-fixture-server port")
	flag.Parse()
	if *proxyPort == "" || *upstreamPort == "" {
		fmt.Fprintln(os.Stderr, "usage: interop-go-sdk-probe -proxy-port P -upstream-port P")
		os.Exit(2)
	}

	upstream := fmt.Sprintf("http://127.0.0.1:%s/mcp", *upstreamPort)
	proxyAddr := fmt.Sprintf("127.0.0.1:%s", *proxyPort)
	proxyURL := fmt.Sprintf("http://%s/mcp", proxyAddr)

	srv := &http.Server{Addr: proxyAddr, Handler: proxyHandler(upstream)}
	go func() {
		if err := srv.ListenAndServe(); err != nil && err != http.ErrServerClosed {
			fmt.Fprintln(os.Stderr, "proxy error:", err)
		}
	}()
	// Give the listener a moment to come up before the client dials it.
	time.Sleep(150 * time.Millisecond)

	var failures []string
	note := func(step string, err error) {
		if err != nil {
			failures = append(failures, fmt.Sprintf("%s: %v", step, err))
			fmt.Printf("FAIL %-28s %v\n", step, err)
		} else {
			fmt.Printf("OK   %s\n", step)
		}
	}

	ctx, cancel := context.WithTimeout(context.Background(), 20*time.Second)
	defer cancel()

	client := mcp.NewClient(&mcp.Implementation{Name: "turul-interop-go-probe", Version: "1.0.0"}, nil)
	cs, err := client.Connect(ctx, &mcp.StreamableClientTransport{Endpoint: proxyURL}, nil)
	if err != nil {
		fmt.Println("CONNECT FAILED:", err)
		printCapture()
		fmt.Println("\nFAILURES:\n  - server/discover (via Connect): " + err.Error())
		os.Exit(1)
	}
	defer cs.Close()

	// --- J1: modern core ---
	toolsRes, err := cs.ListTools(ctx, nil)
	note("tools/list", err)
	if err == nil {
		names := map[string]bool{}
		for _, t := range toolsRes.Tools {
			names[t.Name] = true
		}
		fmt.Printf("     tools: %v\n", toolNames(toolsRes))
		if !names["echo"] || !names["add"] {
			failures = append(failures, fmt.Sprintf("tools/list: expected echo+add, got %v", toolNames(toolsRes)))
		}
	}

	// The fixture server auto-generates structuredContent (a {"result": ...}
	// envelope) from the tool's typed return value; content is the same value
	// JSON-encoded as text. structuredContent is the precise field to assert on.
	echoRes, err := cs.CallTool(ctx, &mcp.CallToolParams{Name: "echo", Arguments: json.RawMessage(`{"text":"hello"}`)})
	note("tools/call echo", err)
	if err == nil {
		if got, ok := structuredResult(echoRes.StructuredContent); !ok || got != "Echo: hello" {
			failures = append(failures, fmt.Sprintf("tools/call echo: structuredContent.result = %v, want %q", got, "Echo: hello"))
		}
	}

	addRes, err := cs.CallTool(ctx, &mcp.CallToolParams{Name: "add", Arguments: json.RawMessage(`{"a":2,"b":3}`)})
	note("tools/call add", err)
	if err == nil {
		if got, ok := structuredResult(addRes.StructuredContent); !ok || got != float64(5) {
			failures = append(failures, fmt.Sprintf("tools/call add: structuredContent.result = %v, want %v", got, float64(5)))
		}
	}

	// --- J2: full read surface ---
	resourcesRes, err := cs.ListResources(ctx, nil)
	note("resources/list", err)
	if err == nil {
		fmt.Printf("     resources: %d\n", len(resourcesRes.Resources))
	}

	readRes, err := cs.ReadResource(ctx, &mcp.ReadResourceParams{URI: "file:///fixture/readme.md"})
	note("resources/read", err)
	if err == nil && len(readRes.Contents) > 0 && !strings.Contains(readRes.Contents[0].Text, "Interop fixture") {
		failures = append(failures, fmt.Sprintf("resources/read: contents = %q, want substring %q", readRes.Contents[0].Text, "Interop fixture"))
	}

	tmplRes, err := cs.ListResourceTemplates(ctx, nil)
	note("resources/templates/list", err)
	if err == nil && tmplRes.ResourceTemplates == nil {
		failures = append(failures, "resources/templates/list: resourceTemplates was null, want an empty array")
	} else if err == nil && len(tmplRes.ResourceTemplates) != 0 {
		failures = append(failures, fmt.Sprintf("resources/templates/list: got %d templates, want 0 (none registered)", len(tmplRes.ResourceTemplates)))
	}

	promptsRes, err := cs.ListPrompts(ctx, nil)
	note("prompts/list", err)
	if err == nil {
		fmt.Printf("     prompts: %d\n", len(promptsRes.Prompts))
	}

	getPromptRes, err := cs.GetPrompt(ctx, &mcp.GetPromptParams{Name: "greeting", Arguments: map[string]string{"name": "World"}})
	note("prompts/get", err)
	if err == nil && len(getPromptRes.Messages) > 0 {
		if got, ok := getPromptRes.Messages[0].Content.(*mcp.TextContent); !ok || got.Text != "Hello, World!" {
			failures = append(failures, fmt.Sprintf("prompts/get: message text = %#v, want %q", getPromptRes.Messages[0].Content, "Hello, World!"))
		}
	}

	completeRes, err := cs.Complete(ctx, &mcp.CompleteParams{
		Ref:      &mcp.CompleteReference{Type: "ref/prompt", Name: "greeting"},
		Argument: mcp.CompleteParamsArgument{Name: "name", Value: "a"},
	})
	note("completion/complete", err)
	if err == nil {
		want := map[string]bool{"ada": true, "alan": true}
		got := map[string]bool{}
		for _, v := range completeRes.Completion.Values {
			got[v] = true
		}
		if len(got) != len(want) || !got["ada"] || !got["alan"] {
			failures = append(failures, fmt.Sprintf("completion/complete: values = %v, want [ada alan]", completeRes.Completion.Values))
		}
	}

	_ = cs.Close()
	time.Sleep(150 * time.Millisecond) // let the proxy finish recording the last round trip
	_ = srv.Close()

	entries := printCapture()

	// --- assertions on captured bytes ---
	seen := map[string]bool{}
	for _, c := range entries {
		if !strings.HasPrefix(c.rpcMethod, "GET ") && !strings.HasPrefix(c.rpcMethod, "DELETE ") && c.rpcMethod != "" {
			seen[c.rpcMethod] = true
		}
		if strings.Contains(c.rpcMethod, " /mcp") {
			failures = append(failures, fmt.Sprintf("unexpected %s: a 2026-07-28 stateless client must not open the legacy GET SSE stream or send a session DELETE", c.rpcMethod))
			continue
		}
		if c.reqProtoVer != "2026-07-28" {
			failures = append(failures, fmt.Sprintf("%s: MCP-Protocol-Version header was %q, want \"2026-07-28\"", c.rpcMethod, c.reqProtoVer))
		}
		if c.reqSessionID != "" {
			failures = append(failures, fmt.Sprintf("%s: sent Mcp-Session-Id %q; 2026-07-28 is stateless", c.rpcMethod, c.reqSessionID))
		}
		if c.rpcMethod == "initialize" || c.rpcMethod == "notifications/initialized" {
			failures = append(failures, "client sent removed lifecycle method "+c.rpcMethod)
		}
		if c.reqMcpMethod != c.rpcMethod {
			failures = append(failures, fmt.Sprintf("%s: Mcp-Method header was %q, want %q", c.rpcMethod, c.reqMcpMethod, c.rpcMethod))
		}

		var env struct {
			Result json.RawMessage `json:"result"`
			Error  json.RawMessage `json:"error"`
		}
		if err := json.Unmarshal(c.respBody, &env); err != nil {
			failures = append(failures, fmt.Sprintf("%s: response body did not decode as JSON-RPC: %v", c.rpcMethod, err))
			continue
		}
		if len(env.Error) > 0 {
			failures = append(failures, fmt.Sprintf("%s: server returned a JSON-RPC error: %s", c.rpcMethod, string(env.Error)))
			continue
		}
		var result map[string]any
		if err := json.Unmarshal(env.Result, &result); err != nil {
			continue
		}
		if _, ok := result["resultType"]; !ok {
			failures = append(failures, fmt.Sprintf("%s: result missing resultType", c.rpcMethod))
		}
		if cacheableMethods[c.rpcMethod] {
			_, hasTTL := result["ttlMs"]
			_, hasScope := result["cacheScope"]
			if !hasTTL || !hasScope {
				failures = append(failures, fmt.Sprintf("%s: cacheable result missing ttlMs/cacheScope (hasTTL=%v hasCacheScope=%v)", c.rpcMethod, hasTTL, hasScope))
			}
		}
	}
	for _, m := range allMethods {
		if !seen[m] {
			failures = append(failures, "client never sent "+m)
		}
	}

	if len(failures) > 0 {
		fmt.Println("\nFAILURES:")
		for _, f := range failures {
			fmt.Println("  -", f)
		}
		os.Exit(1)
	}
	fmt.Println("\nPASS: Go SDK client completed J1+J2 over the stateless 2026-07-28 journey")
}

func printCapture() []*captured {
	entries := snapshot()
	fmt.Println("\n=== wire capture ===")
	for _, c := range entries {
		fmt.Printf("  rpc=%-26q protoVer=%-12q mcpMethod=%-26q mcpName=%-30q sessionId=%-8q status=%d\n",
			c.rpcMethod, c.reqProtoVer, c.reqMcpMethod, c.reqMcpName, c.reqSessionID, c.respStatus)
	}
	return entries
}

func toolNames(r *mcp.ListToolsResult) []string {
	out := make([]string, 0, len(r.Tools))
	for _, t := range r.Tools {
		out = append(out, t.Name)
	}
	return out
}

// structuredResult extracts the "result" field the fixture server's tools
// wrap their typed return value in (see turul's output_field convention).
func structuredResult(sc any) (any, bool) {
	m, ok := sc.(map[string]any)
	if !ok {
		return nil, false
	}
	v, ok := m["result"]
	return v, ok
}
