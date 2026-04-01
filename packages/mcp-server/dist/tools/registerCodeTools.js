import { codeToolSchemas, MATCH_MODES, PUBLIC_ENDPOINT_KINDS, PUBLIC_FRAMEWORKS, PUBLIC_LANGUAGES, PUBLIC_SYMBOL_KINDS, } from "../contracts/public/code.js";
import * as z from "zod/v4";
const toolMetadata = [
    {
        name: "code.inspect_tree",
        title: "Inspect workspace tree",
        description: "Inspect the workspace file tree without reading file contents. Supports path scoping, depth limits, extension filters, filename globs, and optional stats.",
        inputSchema: { ...codeToolSchemas["code.inspect_tree"] },
        sdkInputSchema: {
            path: z
                .string()
                .nullable()
                .optional()
                .default(null)
                .describe("Optional workspace-relative or absolute file/directory scope."),
            max_depth: z
                .int()
                .min(0)
                .max(20)
                .optional()
                .default(3)
                .describe("Maximum depth relative to the resolved scope root."),
            extensions: z
                .array(z.string())
                .nullable()
                .optional()
                .default(null)
                .describe("Optional file extension filter such as ['.py', '.ts']. Directories remain visible."),
            file_pattern: z
                .string()
                .nullable()
                .optional()
                .default(null)
                .describe("Optional filename glob such as '*.py'."),
            include_stats: z
                .boolean()
                .optional()
                .default(false)
                .describe("Include size, modified time, and symlink metadata."),
            include_hidden: z
                .boolean()
                .optional()
                .default(false)
                .describe("Include hidden entries except the hard ignore list."),
        },
    },
    {
        name: "code.list_endpoints",
        title: "List endpoints and routes",
        description: "List backend endpoints and frontend routes in the workspace. Supports path scoping plus language, framework, kind, and limit filters.",
        inputSchema: { ...codeToolSchemas["code.list_endpoints"] },
        sdkInputSchema: {
            path: z.string().nullable().optional().default(null),
            language: z.enum(PUBLIC_LANGUAGES).nullable().optional().default(null),
            framework: z.enum(PUBLIC_FRAMEWORKS).nullable().optional().default(null),
            kind: z.enum(PUBLIC_ENDPOINT_KINDS).optional().default("any"),
            limit: z.int().min(1).max(200).optional().default(50),
        },
    },
    {
        name: "code.find_symbol",
        title: "Find symbol definitions",
        description: "Locate symbol definitions in the workspace by name. Supports exact or fuzzy matching, path scoping, and language/framework/kind filtering.",
        inputSchema: { ...codeToolSchemas["code.find_symbol"] },
        sdkInputSchema: {
            symbol: z.string().trim().min(1),
            language: z.enum(PUBLIC_LANGUAGES).nullable().optional().default(null),
            framework: z.enum(PUBLIC_FRAMEWORKS).nullable().optional().default(null),
            kind: z.enum(PUBLIC_SYMBOL_KINDS).optional().default("any"),
            match: z.enum(MATCH_MODES).optional().default("exact"),
            path: z.string().nullable().optional().default(null),
            limit: z.int().min(1).max(200).optional().default(50),
        },
    },
    {
        name: "code.search_text",
        title: "Search text",
        description: "Search text or regex patterns across the workspace with file, language, path, and context controls.",
        inputSchema: { ...codeToolSchemas["code.search_text"] },
        sdkInputSchema: {
            query: z.string().trim().min(1),
            path: z.string().nullable().optional().default(null),
            language: z.enum(PUBLIC_LANGUAGES).nullable().optional().default(null),
            framework: z.enum(PUBLIC_FRAMEWORKS).nullable().optional().default(null),
            include: z.string().nullable().optional().default(null),
            regex: z.boolean().optional().default(false),
            context: z.int().min(0).max(10).optional().default(1),
            limit: z.int().min(1).max(200).optional().default(50),
        },
    },
    {
        name: "code.trace_flow",
        title: "Trace execution flow forward",
        description: "Trace execution flow forward from a starting file and symbol to related workspace files. The starting path must exist inside the workspace.",
        inputSchema: { ...codeToolSchemas["code.trace_flow"] },
        sdkInputSchema: {
            path: z.string().trim().min(1),
            symbol: z.string().trim().min(1),
            language: z.enum(PUBLIC_LANGUAGES).nullable().optional().default(null),
            framework: z.enum(PUBLIC_FRAMEWORKS).nullable().optional().default(null),
        },
    },
    {
        name: "code.trace_callers",
        title: "Trace incoming callers",
        description: "Trace incoming callers for a symbol from a starting file. Recursive mode supports reverse traversal up to a bounded max_depth and may return a partial response for safety.",
        inputSchema: { ...codeToolSchemas["code.trace_callers"] },
        sdkInputSchema: {
            path: z.string().trim().min(1),
            symbol: z.string().trim().min(1),
            language: z.enum(PUBLIC_LANGUAGES).nullable().optional().default(null),
            framework: z.enum(PUBLIC_FRAMEWORKS).nullable().optional().default(null),
            recursive: z.boolean().optional().default(false),
            max_depth: z.int().min(1).max(8).nullable().optional().default(null),
        },
    },
];
export function registerCodeTools(options) {
    return toolMetadata.map((tool) => {
        if (tool.name === "code.inspect_tree") {
            return {
                ...tool,
                execute: async (payload) => {
                    if (!options.inspectTreeHandler) {
                        throw new Error("Inspect tree migrated handler is scaffolded but not yet wired.");
                    }
                    return options.inspectTreeHandler(payload);
                },
            };
        }
        if (tool.name === "code.find_symbol") {
            return {
                ...tool,
                execute: async (payload) => {
                    if (!options.findSymbolHandler) {
                        throw new Error("Find symbol migrated handler is scaffolded but not yet wired.");
                    }
                    return options.findSymbolHandler(payload);
                },
            };
        }
        if (tool.name === "code.list_endpoints") {
            return {
                ...tool,
                execute: async (payload) => {
                    if (!options.listEndpointsHandler) {
                        throw new Error("List endpoints migrated handler is scaffolded but not yet wired.");
                    }
                    return options.listEndpointsHandler(payload);
                },
            };
        }
        if (tool.name === "code.search_text") {
            return {
                ...tool,
                execute: async (payload) => {
                    if (!options.searchTextHandler) {
                        throw new Error("Search text migrated handler is scaffolded but not yet wired.");
                    }
                    return options.searchTextHandler(payload);
                },
            };
        }
        if (tool.name === "code.trace_flow") {
            return {
                ...tool,
                execute: async (payload) => {
                    if (!options.traceFlowHandler) {
                        throw new Error("Trace flow migrated handler is scaffolded but not yet wired.");
                    }
                    return options.traceFlowHandler(payload);
                },
            };
        }
        if (tool.name === "code.trace_callers") {
            return {
                ...tool,
                execute: async (payload) => {
                    if (!options.traceCallersHandler) {
                        throw new Error("Trace callers migrated handler is scaffolded but not yet wired.");
                    }
                    return options.traceCallersHandler(payload);
                },
            };
        }
        throw new Error(`Tool '${tool.name}' is not implemented.`);
    });
}
