use super::*;

/// Build the unified `tools/list` result.
///
/// Tool order:
/// 1. Gateway discovery / pooling meta-tools (`list_dcc_instances`,
///    `get_dcc_instance`, `connect_to_dcc`, `acquire_dcc_instance`,
///    `release_dcc_instance`).
/// 2. Skill-management tools (one canonical set for the whole gateway).
///
/// Backend tools are **not** fanned out. Agents discover and invoke
/// backend capabilities through the dynamic wrapper layer
/// (`search_tools` → `describe_tool` → `call_tool`).
///
/// Pagination uses the same cursor scheme as the per-DCC server:
/// `cursor` is an opaque hex-encoded offset into the flat tool list.
pub async fn aggregate_tools_list(_gs: &GatewayState, cursor: Option<&str>) -> Value {
    let mut tools: Vec<Value> = Vec::new();

    // Tier1 + 2: local gateway tools (meta + skill management).
    if let Value::Array(local) = gateway_tool_defs() {
        tools.extend(local);
    }
    tools.extend(skill_management_tool_defs());

    // ── Pagination ───────────────────────────────────────────────────────
    let offset = cursor.and_then(decode_cursor).unwrap_or(0);
    let total = tools.len();
    let page_end = (offset + TOOLS_LIST_PAGE_SIZE).min(total);
    let page: Vec<Value> = if offset < total {
        tools.drain(offset..page_end).collect()
    } else {
        Vec::new()
    };

    let mut result = json!({"tools": page});
    if page_end < total {
        result["nextCursor"] = json!(encode_cursor(page_end));
    }
    result
}
