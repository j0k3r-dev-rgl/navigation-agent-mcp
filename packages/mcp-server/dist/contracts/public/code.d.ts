export declare const PUBLIC_LANGUAGES: readonly ["typescript", "javascript", "java", "python", "rust"];
export type PublicLanguage = (typeof PUBLIC_LANGUAGES)[number];
export declare const PUBLIC_FRAMEWORKS: readonly ["react-router", "spring"];
export type PublicFramework = (typeof PUBLIC_FRAMEWORKS)[number];
export declare const PUBLIC_ENDPOINT_KINDS: readonly ["any", "graphql", "rest", "route"];
export type PublicEndpointKind = (typeof PUBLIC_ENDPOINT_KINDS)[number];
export declare const PUBLIC_SYMBOL_KINDS: readonly ["any", "class", "interface", "function", "method", "type", "enum", "constructor", "annotation"];
export type PublicSymbolKind = (typeof PUBLIC_SYMBOL_KINDS)[number];
export declare const MATCH_MODES: readonly ["exact", "fuzzy"];
export type MatchMode = (typeof MATCH_MODES)[number];
export declare const CODE_TOOL_NAMES: readonly ["code.inspect_tree", "code.list_endpoints", "code.find_symbol", "code.search_text", "code.trace_symbol", "code.trace_callers"];
export type CodeToolName = (typeof CODE_TOOL_NAMES)[number];
export interface InspectTreeInput {
    path?: string | null;
    max_depth: number;
    extensions: string[];
    file_pattern?: string | null;
    include_stats: boolean;
    include_hidden: boolean;
}
export interface InspectTreeItemStats {
    sizeBytes: number;
    modifiedAt: string;
    isSymlink: boolean;
}
export interface InspectTreeItem {
    path: string;
    name: string;
    type: string;
    depth: number;
    extension?: string | null;
    stats?: InspectTreeItemStats | null;
}
export interface InspectTreeData {
    root: string;
    entryCount: number;
    items: InspectTreeItem[];
}
export interface FindSymbolInput {
    symbol: string;
    language?: PublicLanguage | null;
    framework?: PublicFramework | null;
    kind: PublicSymbolKind;
    match: MatchMode;
    path?: string | null;
    limit: number;
}
export interface PublicSymbolDefinition {
    symbol: string;
    kind: PublicSymbolKind;
    path: string;
    line: number;
    language: PublicLanguage | null;
}
export interface FindSymbolData {
    count: number;
    returnedCount: number;
    totalMatched: number;
    items: PublicSymbolDefinition[];
}
export interface ListEndpointsInput {
    path?: string | null;
    language?: PublicLanguage | null;
    framework?: PublicFramework | null;
    kind: PublicEndpointKind;
    limit: number;
}
export interface EndpointDefinition {
    name: string;
    kind: PublicEndpointKind;
    path: string | null;
    file: string;
    line: number;
    language: PublicLanguage | null;
    framework: PublicFramework | null;
}
export interface ListEndpointsCounts {
    byKind: Record<string, number>;
    byLanguage: Record<string, number>;
    byFramework: Record<string, number>;
}
export interface ListEndpointsData {
    totalCount: number;
    returnedCount: number;
    counts: ListEndpointsCounts;
    items: EndpointDefinition[];
}
export interface SearchTextInput {
    query: string;
    path?: string | null;
    language?: PublicLanguage | null;
    framework?: PublicFramework | null;
    include?: string | null;
    regex: boolean;
    context: number;
    limit: number;
}
export interface SearchTextContextLine {
    line: number;
    text: string;
}
export interface SearchTextSubmatch {
    start: number;
    end: number;
    text: string;
}
export interface SearchTextMatch {
    line: number;
    text: string;
    submatches: SearchTextSubmatch[];
    before: SearchTextContextLine[];
    after: SearchTextContextLine[];
}
export interface SearchTextFileMatch {
    path: string;
    language: PublicLanguage | null;
    matchCount: number;
    matches: SearchTextMatch[];
}
export interface SearchTextData {
    fileCount: number;
    matchCount: number;
    totalFileCount: number;
    totalMatchCount: number;
    items: SearchTextFileMatch[];
}
export interface TraceSymbolInput {
    path: string;
    symbol: string;
    language?: PublicLanguage | null;
    framework?: PublicFramework | null;
}
export interface TraceSymbolEntrypoint {
    path: string;
    symbol: string;
    language: PublicLanguage | null;
}
export interface TraceSymbolFile {
    path: string;
    language: PublicLanguage | null;
}
export interface TraceSymbolData {
    entrypoint: TraceSymbolEntrypoint;
    fileCount: number;
    items: TraceSymbolFile[];
}
export interface TraceCallersInput {
    path: string;
    symbol: string;
    language?: PublicLanguage | null;
    framework?: PublicFramework | null;
    recursive: boolean;
    max_depth?: number | null;
}
export interface TraceCallersTarget {
    path: string;
    symbol: string;
    language: PublicLanguage | null;
}
export interface TraceCallerRecord {
    path: string;
    line: number;
    column: number | null;
    caller: string;
    callerSymbol: string | null;
    relation: string;
    language: PublicLanguage | null;
    snippet: string | null;
    receiverType: string | null;
}
export interface TraceCallersRecursiveVia {
    relation: string | null;
    line: number | null;
    column: number | null;
    snippet: string | null;
}
export interface TraceCallersRecursiveNode {
    key: string;
    path: string;
    symbol: string;
    depth: number;
    via: TraceCallersRecursiveVia | null;
}
export interface TraceCallersRecursivePathSegment {
    path: string;
    symbol: string;
    depth: number;
}
export interface TraceCallersRecursiveCycle {
    fromKey: string;
    toKey: string;
    path: string[];
}
export interface TraceCallersRecursiveTruncatedNode {
    key: string;
    path: string;
    symbol: string;
    depth: number;
}
export interface TraceCallersProbableEntryPoint {
    key: string | null;
    path: string;
    symbol: string;
    depth: number | null;
    reasons: string[];
    probable: boolean | null;
    pathFromTarget: TraceCallersRecursivePathSegment[];
}
export interface TraceCallersCallsTarget {
    path: string;
    symbol: string;
}
export interface TraceCallersClassificationRecord {
    path: string;
    symbol: string;
    caller: string;
    depth: number;
    line: number;
    column: number | null;
    relation: string;
    language: PublicLanguage | null;
    receiverType: string | null;
    snippet: string | null;
    calls: TraceCallersCallsTarget;
    pathFromTarget: TraceCallersRecursivePathSegment[];
}
export interface TraceCallersImplementationInterface {
    name: string | null;
    path: string | null;
    symbol: string | null;
}
export interface TraceCallersImplementationReference {
    path: string;
    symbol: string | null;
}
export interface TraceCallersImplementationInterfaceChain {
    kind: string;
    probable: boolean | null;
    interface: TraceCallersImplementationInterface | null;
    implementation: TraceCallersImplementationReference | null;
    implementations: TraceCallersImplementationReference[];
    callers: TraceCallersClassificationRecord[];
}
export interface TraceCallersRecursiveSummary {
    directCallerCount: number;
    indirectCallerCount: number;
    probablePublicEntryPointCount: number;
    implementationInterfaceChainCount: number;
}
export interface TraceCallersRecursiveClassifications {
    summary: TraceCallersRecursiveSummary;
    directCallers: TraceCallersClassificationRecord[];
    indirectCallers: TraceCallersClassificationRecord[];
    probablePublicEntryPoints: TraceCallersProbableEntryPoint[];
    implementationInterfaceChain: TraceCallersImplementationInterfaceChain[];
}
export interface TraceCallersRecursiveData {
    enabled: boolean;
    root: TraceCallersRecursiveNode;
    maxDepth: number;
    maxDepthObserved: number;
    nodeCount: number;
    edgeCount: number;
    pathCount: number;
    nodes: TraceCallersRecursiveNode[];
    adjacency: Record<string, string[]>;
    paths: TraceCallersRecursivePathSegment[][];
    cycles: TraceCallersRecursiveCycle[];
    truncated: TraceCallersRecursiveTruncatedNode[];
    probableEntryPoints: TraceCallersProbableEntryPoint[];
    classifications: TraceCallersRecursiveClassifications;
}
export interface TraceCallersData {
    target: TraceCallersTarget;
    count: number;
    returnedCount: number;
    items: TraceCallerRecord[];
    recursive: TraceCallersRecursiveData | null;
}
export interface ValidationIssue {
    field: string;
    message: string;
    type: string;
}
export declare const inspectTreeInputSchema: {
    readonly type: "object";
    readonly properties: {
        readonly path: {
            readonly anyOf: readonly [{
                readonly type: "string";
            }, {
                readonly type: "null";
            }];
            readonly default: null;
            readonly description: "Optional workspace-relative or absolute file/directory scope.";
        };
        readonly max_depth: {
            readonly type: "integer";
            readonly default: 3;
            readonly minimum: 0;
            readonly maximum: 20;
            readonly description: "Maximum depth relative to the resolved scope root.";
        };
        readonly extensions: {
            readonly anyOf: readonly [{
                readonly type: "array";
                readonly items: {
                    readonly type: "string";
                };
            }, {
                readonly type: "null";
            }];
            readonly default: null;
            readonly description: "Optional file extension filter such as ['.py', '.ts']. Directories remain visible.";
        };
        readonly file_pattern: {
            readonly anyOf: readonly [{
                readonly type: "string";
            }, {
                readonly type: "null";
            }];
            readonly default: null;
            readonly description: "Optional filename glob such as '*.py'.";
        };
        readonly include_stats: {
            readonly type: "boolean";
            readonly default: false;
            readonly description: "Include size, modified time, and symlink metadata.";
        };
        readonly include_hidden: {
            readonly type: "boolean";
            readonly default: false;
            readonly description: "Include hidden entries except the hard ignore list.";
        };
    };
    readonly required: readonly [];
};
export declare const listEndpointsInputSchema: {
    readonly type: "object";
    readonly properties: {
        readonly path: {
            readonly anyOf: readonly [{
                readonly type: "string";
            }, {
                readonly type: "null";
            }];
            readonly default: null;
        };
        readonly language: {
            readonly anyOf: readonly [{
                readonly $ref: "#/$defs/PublicLanguage";
            }, {
                readonly type: "null";
            }];
            readonly default: null;
        };
        readonly framework: {
            readonly anyOf: readonly [{
                readonly $ref: "#/$defs/PublicFramework";
            }, {
                readonly type: "null";
            }];
            readonly default: null;
        };
        readonly kind: {
            readonly allOf: readonly [{
                readonly $ref: "#/$defs/PublicEndpointKind";
            }];
            readonly default: "any";
        };
        readonly limit: {
            readonly type: "integer";
            readonly default: 50;
            readonly minimum: 1;
            readonly maximum: 200;
        };
    };
    readonly required: readonly [];
    readonly $defs: {
        readonly PublicLanguage: {
            readonly type: "string";
            readonly enum: readonly ["typescript", "javascript", "java", "python", "rust"];
        };
        readonly PublicFramework: {
            readonly type: "string";
            readonly enum: readonly ["react-router", "spring"];
        };
        readonly PublicEndpointKind: {
            readonly type: "string";
            readonly enum: readonly ["any", "graphql", "rest", "route"];
        };
        readonly PublicSymbolKind: {
            readonly type: "string";
            readonly enum: readonly ["any", "class", "interface", "function", "method", "type", "enum", "constructor", "annotation"];
        };
        readonly MatchMode: {
            readonly type: "string";
            readonly enum: readonly ["exact", "fuzzy"];
        };
    };
};
export declare const findSymbolInputSchema: {
    readonly type: "object";
    readonly properties: {
        readonly symbol: {
            readonly type: "string";
            readonly minLength: 1;
        };
        readonly language: {
            readonly anyOf: readonly [{
                readonly $ref: "#/$defs/PublicLanguage";
            }, {
                readonly type: "null";
            }];
            readonly default: null;
        };
        readonly framework: {
            readonly anyOf: readonly [{
                readonly $ref: "#/$defs/PublicFramework";
            }, {
                readonly type: "null";
            }];
            readonly default: null;
        };
        readonly kind: {
            readonly allOf: readonly [{
                readonly $ref: "#/$defs/PublicSymbolKind";
            }];
            readonly default: "any";
        };
        readonly match: {
            readonly allOf: readonly [{
                readonly $ref: "#/$defs/MatchMode";
            }];
            readonly default: "exact";
        };
        readonly path: {
            readonly anyOf: readonly [{
                readonly type: "string";
            }, {
                readonly type: "null";
            }];
            readonly default: null;
        };
        readonly limit: {
            readonly type: "integer";
            readonly default: 50;
            readonly minimum: 1;
            readonly maximum: 200;
        };
    };
    readonly required: readonly ["symbol"];
    readonly $defs: {
        readonly PublicLanguage: {
            readonly type: "string";
            readonly enum: readonly ["typescript", "javascript", "java", "python", "rust"];
        };
        readonly PublicFramework: {
            readonly type: "string";
            readonly enum: readonly ["react-router", "spring"];
        };
        readonly PublicEndpointKind: {
            readonly type: "string";
            readonly enum: readonly ["any", "graphql", "rest", "route"];
        };
        readonly PublicSymbolKind: {
            readonly type: "string";
            readonly enum: readonly ["any", "class", "interface", "function", "method", "type", "enum", "constructor", "annotation"];
        };
        readonly MatchMode: {
            readonly type: "string";
            readonly enum: readonly ["exact", "fuzzy"];
        };
    };
};
export declare const searchTextInputSchema: {
    readonly type: "object";
    readonly properties: {
        readonly query: {
            readonly type: "string";
            readonly minLength: 1;
        };
        readonly path: {
            readonly anyOf: readonly [{
                readonly type: "string";
            }, {
                readonly type: "null";
            }];
            readonly default: null;
        };
        readonly language: {
            readonly anyOf: readonly [{
                readonly $ref: "#/$defs/PublicLanguage";
            }, {
                readonly type: "null";
            }];
            readonly default: null;
        };
        readonly framework: {
            readonly anyOf: readonly [{
                readonly $ref: "#/$defs/PublicFramework";
            }, {
                readonly type: "null";
            }];
            readonly default: null;
        };
        readonly include: {
            readonly anyOf: readonly [{
                readonly type: "string";
            }, {
                readonly type: "null";
            }];
            readonly default: null;
        };
        readonly regex: {
            readonly type: "boolean";
            readonly default: false;
        };
        readonly context: {
            readonly type: "integer";
            readonly default: 1;
            readonly minimum: 0;
            readonly maximum: 10;
        };
        readonly limit: {
            readonly type: "integer";
            readonly default: 50;
            readonly minimum: 1;
            readonly maximum: 200;
        };
    };
    readonly required: readonly ["query"];
    readonly $defs: {
        readonly PublicLanguage: {
            readonly type: "string";
            readonly enum: readonly ["typescript", "javascript", "java", "python", "rust"];
        };
        readonly PublicFramework: {
            readonly type: "string";
            readonly enum: readonly ["react-router", "spring"];
        };
        readonly PublicEndpointKind: {
            readonly type: "string";
            readonly enum: readonly ["any", "graphql", "rest", "route"];
        };
        readonly PublicSymbolKind: {
            readonly type: "string";
            readonly enum: readonly ["any", "class", "interface", "function", "method", "type", "enum", "constructor", "annotation"];
        };
        readonly MatchMode: {
            readonly type: "string";
            readonly enum: readonly ["exact", "fuzzy"];
        };
    };
};
export declare const traceSymbolInputSchema: {
    readonly type: "object";
    readonly properties: {
        readonly path: {
            readonly type: "string";
            readonly minLength: 1;
        };
        readonly symbol: {
            readonly type: "string";
            readonly minLength: 1;
        };
        readonly language: {
            readonly anyOf: readonly [{
                readonly $ref: "#/$defs/PublicLanguage";
            }, {
                readonly type: "null";
            }];
            readonly default: null;
        };
        readonly framework: {
            readonly anyOf: readonly [{
                readonly $ref: "#/$defs/PublicFramework";
            }, {
                readonly type: "null";
            }];
            readonly default: null;
        };
    };
    readonly required: readonly ["path", "symbol"];
    readonly $defs: {
        readonly PublicLanguage: {
            readonly type: "string";
            readonly enum: readonly ["typescript", "javascript", "java", "python", "rust"];
        };
        readonly PublicFramework: {
            readonly type: "string";
            readonly enum: readonly ["react-router", "spring"];
        };
        readonly PublicEndpointKind: {
            readonly type: "string";
            readonly enum: readonly ["any", "graphql", "rest", "route"];
        };
        readonly PublicSymbolKind: {
            readonly type: "string";
            readonly enum: readonly ["any", "class", "interface", "function", "method", "type", "enum", "constructor", "annotation"];
        };
        readonly MatchMode: {
            readonly type: "string";
            readonly enum: readonly ["exact", "fuzzy"];
        };
    };
};
export declare const traceCallersInputSchema: {
    readonly type: "object";
    readonly properties: {
        readonly path: {
            readonly type: "string";
            readonly minLength: 1;
        };
        readonly symbol: {
            readonly type: "string";
            readonly minLength: 1;
        };
        readonly language: {
            readonly anyOf: readonly [{
                readonly $ref: "#/$defs/PublicLanguage";
            }, {
                readonly type: "null";
            }];
            readonly default: null;
        };
        readonly framework: {
            readonly anyOf: readonly [{
                readonly $ref: "#/$defs/PublicFramework";
            }, {
                readonly type: "null";
            }];
            readonly default: null;
        };
        readonly recursive: {
            readonly type: "boolean";
            readonly default: false;
        };
        readonly max_depth: {
            readonly anyOf: readonly [{
                readonly type: "integer";
                readonly minimum: 1;
                readonly maximum: 8;
            }, {
                readonly type: "null";
            }];
            readonly default: null;
        };
    };
    readonly required: readonly ["path", "symbol"];
    readonly $defs: {
        readonly PublicLanguage: {
            readonly type: "string";
            readonly enum: readonly ["typescript", "javascript", "java", "python", "rust"];
        };
        readonly PublicFramework: {
            readonly type: "string";
            readonly enum: readonly ["react-router", "spring"];
        };
        readonly PublicEndpointKind: {
            readonly type: "string";
            readonly enum: readonly ["any", "graphql", "rest", "route"];
        };
        readonly PublicSymbolKind: {
            readonly type: "string";
            readonly enum: readonly ["any", "class", "interface", "function", "method", "type", "enum", "constructor", "annotation"];
        };
        readonly MatchMode: {
            readonly type: "string";
            readonly enum: readonly ["exact", "fuzzy"];
        };
    };
};
export declare const codeToolSchemas: {
    readonly "code.inspect_tree": {
        readonly type: "object";
        readonly properties: {
            readonly path: {
                readonly anyOf: readonly [{
                    readonly type: "string";
                }, {
                    readonly type: "null";
                }];
                readonly default: null;
                readonly description: "Optional workspace-relative or absolute file/directory scope.";
            };
            readonly max_depth: {
                readonly type: "integer";
                readonly default: 3;
                readonly minimum: 0;
                readonly maximum: 20;
                readonly description: "Maximum depth relative to the resolved scope root.";
            };
            readonly extensions: {
                readonly anyOf: readonly [{
                    readonly type: "array";
                    readonly items: {
                        readonly type: "string";
                    };
                }, {
                    readonly type: "null";
                }];
                readonly default: null;
                readonly description: "Optional file extension filter such as ['.py', '.ts']. Directories remain visible.";
            };
            readonly file_pattern: {
                readonly anyOf: readonly [{
                    readonly type: "string";
                }, {
                    readonly type: "null";
                }];
                readonly default: null;
                readonly description: "Optional filename glob such as '*.py'.";
            };
            readonly include_stats: {
                readonly type: "boolean";
                readonly default: false;
                readonly description: "Include size, modified time, and symlink metadata.";
            };
            readonly include_hidden: {
                readonly type: "boolean";
                readonly default: false;
                readonly description: "Include hidden entries except the hard ignore list.";
            };
        };
        readonly required: readonly [];
    };
    readonly "code.list_endpoints": {
        readonly type: "object";
        readonly properties: {
            readonly path: {
                readonly anyOf: readonly [{
                    readonly type: "string";
                }, {
                    readonly type: "null";
                }];
                readonly default: null;
            };
            readonly language: {
                readonly anyOf: readonly [{
                    readonly $ref: "#/$defs/PublicLanguage";
                }, {
                    readonly type: "null";
                }];
                readonly default: null;
            };
            readonly framework: {
                readonly anyOf: readonly [{
                    readonly $ref: "#/$defs/PublicFramework";
                }, {
                    readonly type: "null";
                }];
                readonly default: null;
            };
            readonly kind: {
                readonly allOf: readonly [{
                    readonly $ref: "#/$defs/PublicEndpointKind";
                }];
                readonly default: "any";
            };
            readonly limit: {
                readonly type: "integer";
                readonly default: 50;
                readonly minimum: 1;
                readonly maximum: 200;
            };
        };
        readonly required: readonly [];
        readonly $defs: {
            readonly PublicLanguage: {
                readonly type: "string";
                readonly enum: readonly ["typescript", "javascript", "java", "python", "rust"];
            };
            readonly PublicFramework: {
                readonly type: "string";
                readonly enum: readonly ["react-router", "spring"];
            };
            readonly PublicEndpointKind: {
                readonly type: "string";
                readonly enum: readonly ["any", "graphql", "rest", "route"];
            };
            readonly PublicSymbolKind: {
                readonly type: "string";
                readonly enum: readonly ["any", "class", "interface", "function", "method", "type", "enum", "constructor", "annotation"];
            };
            readonly MatchMode: {
                readonly type: "string";
                readonly enum: readonly ["exact", "fuzzy"];
            };
        };
    };
    readonly "code.find_symbol": {
        readonly type: "object";
        readonly properties: {
            readonly symbol: {
                readonly type: "string";
                readonly minLength: 1;
            };
            readonly language: {
                readonly anyOf: readonly [{
                    readonly $ref: "#/$defs/PublicLanguage";
                }, {
                    readonly type: "null";
                }];
                readonly default: null;
            };
            readonly framework: {
                readonly anyOf: readonly [{
                    readonly $ref: "#/$defs/PublicFramework";
                }, {
                    readonly type: "null";
                }];
                readonly default: null;
            };
            readonly kind: {
                readonly allOf: readonly [{
                    readonly $ref: "#/$defs/PublicSymbolKind";
                }];
                readonly default: "any";
            };
            readonly match: {
                readonly allOf: readonly [{
                    readonly $ref: "#/$defs/MatchMode";
                }];
                readonly default: "exact";
            };
            readonly path: {
                readonly anyOf: readonly [{
                    readonly type: "string";
                }, {
                    readonly type: "null";
                }];
                readonly default: null;
            };
            readonly limit: {
                readonly type: "integer";
                readonly default: 50;
                readonly minimum: 1;
                readonly maximum: 200;
            };
        };
        readonly required: readonly ["symbol"];
        readonly $defs: {
            readonly PublicLanguage: {
                readonly type: "string";
                readonly enum: readonly ["typescript", "javascript", "java", "python", "rust"];
            };
            readonly PublicFramework: {
                readonly type: "string";
                readonly enum: readonly ["react-router", "spring"];
            };
            readonly PublicEndpointKind: {
                readonly type: "string";
                readonly enum: readonly ["any", "graphql", "rest", "route"];
            };
            readonly PublicSymbolKind: {
                readonly type: "string";
                readonly enum: readonly ["any", "class", "interface", "function", "method", "type", "enum", "constructor", "annotation"];
            };
            readonly MatchMode: {
                readonly type: "string";
                readonly enum: readonly ["exact", "fuzzy"];
            };
        };
    };
    readonly "code.search_text": {
        readonly type: "object";
        readonly properties: {
            readonly query: {
                readonly type: "string";
                readonly minLength: 1;
            };
            readonly path: {
                readonly anyOf: readonly [{
                    readonly type: "string";
                }, {
                    readonly type: "null";
                }];
                readonly default: null;
            };
            readonly language: {
                readonly anyOf: readonly [{
                    readonly $ref: "#/$defs/PublicLanguage";
                }, {
                    readonly type: "null";
                }];
                readonly default: null;
            };
            readonly framework: {
                readonly anyOf: readonly [{
                    readonly $ref: "#/$defs/PublicFramework";
                }, {
                    readonly type: "null";
                }];
                readonly default: null;
            };
            readonly include: {
                readonly anyOf: readonly [{
                    readonly type: "string";
                }, {
                    readonly type: "null";
                }];
                readonly default: null;
            };
            readonly regex: {
                readonly type: "boolean";
                readonly default: false;
            };
            readonly context: {
                readonly type: "integer";
                readonly default: 1;
                readonly minimum: 0;
                readonly maximum: 10;
            };
            readonly limit: {
                readonly type: "integer";
                readonly default: 50;
                readonly minimum: 1;
                readonly maximum: 200;
            };
        };
        readonly required: readonly ["query"];
        readonly $defs: {
            readonly PublicLanguage: {
                readonly type: "string";
                readonly enum: readonly ["typescript", "javascript", "java", "python", "rust"];
            };
            readonly PublicFramework: {
                readonly type: "string";
                readonly enum: readonly ["react-router", "spring"];
            };
            readonly PublicEndpointKind: {
                readonly type: "string";
                readonly enum: readonly ["any", "graphql", "rest", "route"];
            };
            readonly PublicSymbolKind: {
                readonly type: "string";
                readonly enum: readonly ["any", "class", "interface", "function", "method", "type", "enum", "constructor", "annotation"];
            };
            readonly MatchMode: {
                readonly type: "string";
                readonly enum: readonly ["exact", "fuzzy"];
            };
        };
    };
    readonly "code.trace_symbol": {
        readonly type: "object";
        readonly properties: {
            readonly path: {
                readonly type: "string";
                readonly minLength: 1;
            };
            readonly symbol: {
                readonly type: "string";
                readonly minLength: 1;
            };
            readonly language: {
                readonly anyOf: readonly [{
                    readonly $ref: "#/$defs/PublicLanguage";
                }, {
                    readonly type: "null";
                }];
                readonly default: null;
            };
            readonly framework: {
                readonly anyOf: readonly [{
                    readonly $ref: "#/$defs/PublicFramework";
                }, {
                    readonly type: "null";
                }];
                readonly default: null;
            };
        };
        readonly required: readonly ["path", "symbol"];
        readonly $defs: {
            readonly PublicLanguage: {
                readonly type: "string";
                readonly enum: readonly ["typescript", "javascript", "java", "python", "rust"];
            };
            readonly PublicFramework: {
                readonly type: "string";
                readonly enum: readonly ["react-router", "spring"];
            };
            readonly PublicEndpointKind: {
                readonly type: "string";
                readonly enum: readonly ["any", "graphql", "rest", "route"];
            };
            readonly PublicSymbolKind: {
                readonly type: "string";
                readonly enum: readonly ["any", "class", "interface", "function", "method", "type", "enum", "constructor", "annotation"];
            };
            readonly MatchMode: {
                readonly type: "string";
                readonly enum: readonly ["exact", "fuzzy"];
            };
        };
    };
    readonly "code.trace_callers": {
        readonly type: "object";
        readonly properties: {
            readonly path: {
                readonly type: "string";
                readonly minLength: 1;
            };
            readonly symbol: {
                readonly type: "string";
                readonly minLength: 1;
            };
            readonly language: {
                readonly anyOf: readonly [{
                    readonly $ref: "#/$defs/PublicLanguage";
                }, {
                    readonly type: "null";
                }];
                readonly default: null;
            };
            readonly framework: {
                readonly anyOf: readonly [{
                    readonly $ref: "#/$defs/PublicFramework";
                }, {
                    readonly type: "null";
                }];
                readonly default: null;
            };
            readonly recursive: {
                readonly type: "boolean";
                readonly default: false;
            };
            readonly max_depth: {
                readonly anyOf: readonly [{
                    readonly type: "integer";
                    readonly minimum: 1;
                    readonly maximum: 8;
                }, {
                    readonly type: "null";
                }];
                readonly default: null;
            };
        };
        readonly required: readonly ["path", "symbol"];
        readonly $defs: {
            readonly PublicLanguage: {
                readonly type: "string";
                readonly enum: readonly ["typescript", "javascript", "java", "python", "rust"];
            };
            readonly PublicFramework: {
                readonly type: "string";
                readonly enum: readonly ["react-router", "spring"];
            };
            readonly PublicEndpointKind: {
                readonly type: "string";
                readonly enum: readonly ["any", "graphql", "rest", "route"];
            };
            readonly PublicSymbolKind: {
                readonly type: "string";
                readonly enum: readonly ["any", "class", "interface", "function", "method", "type", "enum", "constructor", "annotation"];
            };
            readonly MatchMode: {
                readonly type: "string";
                readonly enum: readonly ["exact", "fuzzy"];
            };
        };
    };
};
export declare function normalizeInspectTreeInput(payload: Record<string, unknown>): {
    ok: true;
    value: InspectTreeInput;
} | {
    ok: false;
    issues: ValidationIssue[];
};
export declare function normalizeFindSymbolInput(payload: Record<string, unknown>): {
    ok: true;
    value: FindSymbolInput;
} | {
    ok: false;
    issues: ValidationIssue[];
};
export declare function normalizeListEndpointsInput(payload: Record<string, unknown>): {
    ok: true;
    value: ListEndpointsInput;
} | {
    ok: false;
    issues: ValidationIssue[];
};
export declare function normalizeSearchTextInput(payload: Record<string, unknown>): {
    ok: true;
    value: SearchTextInput;
} | {
    ok: false;
    issues: ValidationIssue[];
};
export declare function normalizeTraceSymbolInput(payload: Record<string, unknown>): {
    ok: true;
    value: TraceSymbolInput;
} | {
    ok: false;
    issues: ValidationIssue[];
};
export declare function normalizeTraceCallersInput(payload: Record<string, unknown>): {
    ok: true;
    value: TraceCallersInput;
} | {
    ok: false;
    issues: ValidationIssue[];
};
