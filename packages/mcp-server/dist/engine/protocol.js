export const ENGINE_CAPABILITIES = [
    "workspace.inspect_tree",
    "workspace.find_symbol",
    "workspace.list_endpoints",
    "workspace.search_text",
    "workspace.trace_flow",
    "workspace.trace_callers",
];
let requestSequence = 0;
export function nextRequestId(prefix = "req") {
    requestSequence += 1;
    return `${prefix}-${requestSequence}`;
}
