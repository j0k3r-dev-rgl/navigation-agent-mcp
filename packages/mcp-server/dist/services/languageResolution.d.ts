import type { PublicFramework, PublicLanguage } from "../contracts/public/code.js";
import type { AnalyzerLanguage } from "../engine/protocol.js";
export declare function resolveEffectiveLanguage(language: PublicLanguage | null | undefined, framework: PublicFramework | null | undefined, path?: string | null): PublicLanguage | null;
export declare function resolveAnalyzerLanguage(language: PublicLanguage | null | undefined, framework: PublicFramework | null | undefined, path?: string | null): AnalyzerLanguage;
export declare function inferLanguageFromPath(path: string): PublicLanguage | null;
