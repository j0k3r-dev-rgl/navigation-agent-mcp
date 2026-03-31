import {
  codeToolSchemas,
  type CodeToolName,
  type FindSymbolInput,
  type InspectTreeInput,
} from "../contracts/public/code.ts";
import type { ResponseEnvelope } from "../contracts/public/common.ts";
import type {
  NonMigratedCodeToolName,
  PythonFallbackBridge,
} from "../runtime/pythonFallback.ts";

export interface RegisteredCodeTool {
  name: CodeToolName;
  title: string;
  description: string;
  inputSchema: Record<string, unknown>;
  execute(payload: Record<string, unknown>): Promise<ResponseEnvelope<unknown>>;
}

export interface RegisterCodeToolsOptions {
  fallbackBridge: PythonFallbackBridge;
  inspectTreeHandler?: (
    payload: Record<string, unknown>,
  ) => Promise<ResponseEnvelope<unknown>>;
  findSymbolHandler?: (
    payload: Record<string, unknown>,
  ) => Promise<ResponseEnvelope<unknown>>;
}

const toolMetadata: Array<
  Omit<RegisteredCodeTool, "execute" | "inputSchema"> & {
    inputSchema: Record<string, unknown>;
  }
> = [
  {
    name: "code.inspect_tree",
    title: "Inspect workspace tree",
    description:
      "Inspect the workspace file tree without reading file contents. Supports path scoping, depth limits, extension filters, filename globs, and optional stats.",
    inputSchema: { ...codeToolSchemas["code.inspect_tree"] },
  },
  {
    name: "code.list_endpoints",
    title: "List endpoints and routes",
    description:
      "List backend endpoints and frontend routes in the workspace. Supports path scoping plus language, framework, kind, and limit filters.",
    inputSchema: { ...codeToolSchemas["code.list_endpoints"] },
  },
  {
    name: "code.find_symbol",
    title: "Find symbol definitions",
    description:
      "Locate symbol definitions in the workspace by name. Supports exact or fuzzy matching, path scoping, and language/framework/kind filtering.",
    inputSchema: { ...codeToolSchemas["code.find_symbol"] },
  },
  {
    name: "code.search_text",
    title: "Search text",
    description:
      "Search text or regex patterns across the workspace with file, language, path, and context controls.",
    inputSchema: { ...codeToolSchemas["code.search_text"] },
  },
  {
    name: "code.trace_symbol",
    title: "Trace symbol forward",
    description:
      "Trace a symbol forward from a starting file to related workspace files. The starting path must exist inside the workspace.",
    inputSchema: { ...codeToolSchemas["code.trace_symbol"] },
  },
  {
    name: "code.trace_callers",
    title: "Trace incoming callers",
    description:
      "Trace incoming callers for a symbol from a starting file. Recursive mode supports reverse traversal up to a bounded max_depth and may return a partial response for safety.",
    inputSchema: { ...codeToolSchemas["code.trace_callers"] },
  },
];

export function registerCodeTools(
  options: RegisterCodeToolsOptions,
): RegisteredCodeTool[] {
  return toolMetadata.map((tool) => {
    if (tool.name === "code.inspect_tree") {
      return {
        ...tool,
        execute: async (payload) => {
          if (!options.inspectTreeHandler) {
            throw new Error(
              "Inspect tree migrated handler is scaffolded but not yet wired.",
            );
          }

          return options.inspectTreeHandler(payload as InspectTreeInput);
        },
      };
    }

    if (tool.name === "code.find_symbol") {
      return {
        ...tool,
        execute: async (payload) => {
          if (!options.findSymbolHandler) {
            throw new Error(
              "Find symbol migrated handler is scaffolded but not yet wired.",
            );
          }

          return options.findSymbolHandler(payload as FindSymbolInput);
        },
      };
    }

    return {
      ...tool,
      execute: async (payload) =>
        options.fallbackBridge.execute(
          tool.name as NonMigratedCodeToolName,
          payload,
        ),
    };
  });
}
