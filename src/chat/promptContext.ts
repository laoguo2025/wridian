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

  if (input.draftKind === "screenplay") {
    suggestions.push(...SCREENPLAY_COMMAND_SUGGESTIONS);
  }
  suggestions.push(...WRITING_COMMAND_SUGGESTIONS);

  return suggestions;
}

const WRITING_COMMAND_SUGGESTIONS: PromptSuggestion[] = [
  {
    id: "rewrite-dialogue",
    label: "改对白",
    detail: "让对白更像角色本人、更适合小说或短剧表演",
    insertText: "请把这段对白改得更符合角色口吻，并增强短剧冲突。",
    kind: "command",
  },
  {
    id: "raise-conflict",
    label: "增强冲突",
    detail: "提高场景里的阻力、误会、压迫感或选择成本",
    insertText: "请增强这一段的戏剧冲突，但不要改变既有人物关系和事件顺序。",
    kind: "command",
  },
  {
    id: "add-hook",
    label: "加结尾钩子",
    detail: "补一个适合章节、分场或短剧结尾的悬念",
    insertText: "请给这一段补一个结尾钩子，让读者或观众想继续看下一段。",
    kind: "command",
  },
  {
    id: "voice-check",
    label: "检查角色口吻",
    detail: "检查人物说话是否串味，指出并改写",
    insertText: "请检查这一段的角色口吻是否一致，指出问题并给出修改建议。",
    kind: "command",
  },
  {
    id: "rename-character",
    label: "批量修改角色名",
    detail: "跨段落替换当前文件里的角色名",
    insertText: "请把当前文件里的角色名从「旧名字」批量改成「新名字」，并保持上下文自然。",
    kind: "command",
  },
  {
    id: "extract-memory",
    label: "整理到记忆树",
    detail: "整理人物、设定、伏笔、风格、禁区和剧本规则",
    insertText: "请从当前稿件中整理适合写入记忆树的人物、设定、伏笔、风格、禁区和剧本规则，并说明建议写入哪个记忆树文件。",
    kind: "command",
  },
];

const SCREENPLAY_COMMAND_SUGGESTIONS: PromptSuggestion[] = [
  {
    id: "episode-rhythm",
    label: "拆分分集节奏",
    detail: "按短剧节奏拆分信息点、冲突点和结尾钩子",
    insertText: "请按短剧节奏拆分这一段的分集节奏，标出每集信息点、冲突点和结尾钩子。",
    kind: "command",
  },
  {
    id: "scene-hook",
    label: "强化场景钩子",
    detail: "让本场结尾更适合短剧转场或卡点",
    insertText: "请强化这一场的结尾钩子，让它更适合短剧转场、卡点或下一集开头。",
    kind: "command",
  },
  {
    id: "performable-dialogue",
    label: "对白口语化",
    detail: "把对白改得更短、更可表演、更有冲突",
    insertText: "请把这段对白改得更口语化、更可表演，并保留角色关系和核心信息。",
    kind: "command",
  },
  {
    id: "budget-scene-check",
    label: "场景成本检查",
    detail: "检查场景、人物和动作是否适合低成本短剧拍摄",
    insertText: "请检查这一段的场景、人物调度和动作是否适合低成本短剧拍摄，并给出精简建议。",
    kind: "command",
  },
];
