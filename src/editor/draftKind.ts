import type { DraftKind } from "../chat/promptContext";

export function baseName(path: string) {
  return path.replace(/[\\/]+$/g, "").split(/[\\/]/).pop() || path;
}

export function detectDraftKind(path: string, content: string): DraftKind {
  const lowerPath = path.toLowerCase();
  if (lowerPath.endsWith(".fountain")) return "screenplay";

  const sceneSignals = (content.match(/(^|\n)\s*(INT\.|EXT\.|内景|外景|第[一二三四五六七八九十\d]+[集场])/g) ?? []).length;
  const dialogueSignals = (content.match(/(^|\n)\s*[\u4e00-\u9fa5A-Za-z0-9_]{2,12}[：:]/g) ?? []).length;
  return sceneSignals >= 2 || dialogueSignals >= 4 ? "screenplay" : "prose";
}
