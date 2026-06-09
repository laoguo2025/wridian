import type { CreativeSkill } from "../creativeSkills";

export type PromptContextPillKind =
  | "selection"
  | "active-file"
  | "file"
  | "image"
  | "url"
  | "tool"
  | "memory";

export type DraftKind = "prose" | "screenplay";

export type PromptContextPill = {
  id: string;
  kind: PromptContextPillKind;
  label: string;
  range?: PromptContextRange;
  sourcePath?: string;
  relativePath?: string;
  value: string;
};

export type PromptContextRange = {
  end: number;
  start: number;
};

export type PromptSuggestion = {
  id: string;
  label: string;
  detail: string;
  insertText: string;
  kind: "context" | "command";
  pillKind?: PromptContextPillKind;
  range?: PromptContextRange;
  relativePath?: string;
  sourcePath?: string;
};

export type PromptSuggestionInput = {
  creativeSkills?: CreativeSkill[];
  draftKind: DraftKind;
  knowledgeCards: PromptKnowledgeCardCandidate[];
  knowledgeCategories?: PromptKnowledgeCategoryCandidate[];
  selectedKnowledgeCategoryId?: string;
};

export type PromptKnowledgeCategoryCandidate = {
  id: string;
  title: string;
  detail: string;
};

export type PromptKnowledgeCardCandidate = {
  category?: string;
  categoryId?: string;
  id: string;
  relativePath?: string;
  sourcePath: string;
  title: string;
};

export function serializePromptContextPills(pills: PromptContextPill[]) {
  return pills.map((pill) => `【${pill.label}】\n${pill.value}`).join("\n\n").trim();
}

export function upsertPromptContextPill(pills: PromptContextPill[], pill: PromptContextPill) {
  return [...pills.filter((item) => item.id !== pill.id), pill];
}

export function createSelectionPromptPill(value: string, range?: PromptContextRange): PromptContextPill {
  return {
    id: "selection",
    kind: "selection",
    label: "选区",
    range,
    value,
  };
}

export function createFilePromptPill(name: string, path: string, relativePath?: string): PromptContextPill {
  return {
    id: `file:${path}`,
    kind: "file",
    label: name,
    relativePath,
    sourcePath: path,
    value: `路径：${path}`,
  };
}

export function createFileContentPromptPill(name: string, path: string, content: string): PromptContextPill {
  return {
    id: `file:${path}`,
    kind: "file",
    label: name,
    sourcePath: path,
    value: `路径：${path}\n\n${content}`,
  };
}

export function createReferencedFileContentPromptPill(
  name: string,
  path: string,
  relativePath: string,
  content: string,
): PromptContextPill {
  return {
    id: `file:${path}`,
    kind: "file",
    label: name,
    relativePath,
    sourcePath: path,
    value: content,
  };
}

export function createActiveFilePromptPill(label: string, value: string): PromptContextPill {
  return {
    id: "current-file",
    kind: "active-file",
    label,
    value,
  };
}

export function createUrlPromptPill(url: string): PromptContextPill {
  return {
    id: `url:${url}`,
    kind: "url",
    label: "URL",
    value: url,
  };
}

export function createImagePromptPill(name: string, size: number): PromptContextPill {
  return {
    id: `image:${name}:${size}`,
    kind: "image",
    label: name,
    value: `粘贴图片：${name}，大小 ${size} bytes。当前版本先作为视觉参考元数据进入上下文。`,
  };
}

export function createToolPromptPill(name: string, value: string): PromptContextPill {
  return {
    id: `tool:${name}`,
    kind: "tool",
    label: name,
    value,
  };
}

export function createMemoryPromptPill(label: string, value: string): PromptContextPill {
  return {
    id: `memory:${label}:${value.slice(0, 24)}`,
    kind: "memory",
    label,
    value,
  };
}

export function createPromptPillFromSuggestion(suggestion: PromptSuggestion): PromptContextPill {
  return {
    id: suggestion.id,
    kind: suggestion.pillKind ?? "selection",
    label: suggestion.label,
    relativePath: suggestion.relativePath,
    range: suggestion.range,
    sourcePath: suggestion.sourcePath,
    value: suggestion.insertText,
  };
}

export function buildPromptSuggestions(input: PromptSuggestionInput): PromptSuggestion[] {
  const suggestions: PromptSuggestion[] = [];

  if (!input.selectedKnowledgeCategoryId) {
    for (const category of input.knowledgeCategories ?? []) {
      suggestions.push({
        id: `knowledge-category:${category.id}`,
        label: category.title,
        detail: category.detail,
        insertText: `category:${category.id}`,
        kind: "context",
        pillKind: "tool",
      });
    }
  }

  for (const card of input.knowledgeCards
    .filter((card) => !input.selectedKnowledgeCategoryId || card.categoryId === input.selectedKnowledgeCategoryId)
    .slice(0, 40)) {
    suggestions.push({
      id: `memory:${card.id}`,
      label: card.title || card.category || "知识卡",
      detail: [card.category ?? "知识卡", card.relativePath ?? card.sourcePath].filter(Boolean).join(" · "),
      insertText: `path:${card.sourcePath}`,
      kind: "context",
      pillKind: "memory",
      relativePath: card.relativePath,
      sourcePath: card.sourcePath,
    });
  }

  for (const skill of input.creativeSkills ?? []) {
    suggestions.push({
      id: `creative-skill:${skill.id}`,
      label: skill.title,
      detail: skill.status,
      insertText: skill.prompt,
      kind: "command",
      pillKind: "tool",
    });
  }

  return suggestions;
}
