export function resolveEffectiveLanguage(language, framework, path) {
    if (language) {
        return language;
    }
    if (framework === "react-router") {
        return "typescript";
    }
    if (framework === "spring") {
        return "java";
    }
    if (path) {
        return inferLanguageFromPath(path);
    }
    return null;
}
export function resolveAnalyzerLanguage(language, framework, path) {
    const effective = resolveEffectiveLanguage(language, framework, path);
    if (effective === "go") {
        return "go";
    }
    if (effective === "java") {
        return "java";
    }
    if (effective === "python") {
        return "python";
    }
    if (effective === "rust") {
        return "rust";
    }
    if (effective === "typescript" || effective === "javascript") {
        return "typescript";
    }
    return "auto";
}
export function inferLanguageFromPath(path) {
    const normalized = path.toLowerCase();
    if (normalized.endsWith(".ts") || normalized.endsWith(".tsx")) {
        return "typescript";
    }
    if (normalized.endsWith(".js") || normalized.endsWith(".jsx")) {
        return "javascript";
    }
    if (normalized.endsWith(".go")) {
        return "go";
    }
    if (normalized.endsWith(".java")) {
        return "java";
    }
    if (normalized.endsWith(".py")) {
        return "python";
    }
    if (normalized.endsWith(".rs")) {
        return "rust";
    }
    return null;
}
