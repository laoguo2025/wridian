export type PromptContextPillKind =
  | "selection"
  | "active-file"
  | "file"
  | "url"
  | "tool"
  | "memory";

export type PromptContextPill = {
  id: string;
  kind: PromptContextPillKind;
  label: string;
  value: string;
};

export type PromptSuggestion = {
  id: string;
  label: string;
  detail: string;
  insertText: string;
  kind: "context" | "command";
  pillKind?: PromptContextPillKind;
};

export type PromptSuggestionInput = {
  draftSelectionEnd: number;
  draftSelectionStart: number;
  editorContent: string;
  editorTitle: string;
  selectedPath: string;
  titleFallback: string;
  workspaceFiles: PromptFileCandidate[];
};

export type PromptFileCandidate = {
  name: string;
  path: string;
};

export function serializePromptContextPills(pills: PromptContextPill[]) {
  return pills.map((pill) => `【${pill.label}】\n${pill.value}`).join("\n\n").trim();
}

export function upsertPromptContextPill(pills: PromptContextPill[], pill: PromptContextPill) {
  return [...pills.filter((item) => item.id !== pill.id), pill];
}

export function createSelectionPromptPill(value: string): PromptContextPill {
  return {
    id: "selection",
    kind: "selection",
    label: "选区",
    value,
  };
}

export function createFilePromptPill(name: string, path: string): PromptContextPill {
  return {
    id: `file:${path}`,
    kind: "file",
    label: name,
    value: `路径：${path}`,
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
    value: suggestion.insertText,
  };
}

export function buildPromptSuggestions(input: PromptSuggestionInput): PromptSuggestion[] {
  const suggestions: PromptSuggestion[] = [];
  const selectedDraftText = input.editorContent.slice(input.draftSelectionStart, input.draftSelectionEnd).trim();

  if (selectedDraftText) {
    suggestions.push({
      id: "selection",
      label: "当前选区",
      detail: "把正文里划选的片段作为本轮上下文",
      insertText: selectedDraftText,
      kind: "context",
      pillKind: "selection",
    });
  }

  if (input.selectedPath && input.editorContent.trim()) {
    suggestions.push(
      {
        id: "current-file",
        label: "当前文件",
        detail: input.editorTitle || input.titleFallback,
        insertText: `标题：${input.editorTitle || input.titleFallback}\n\n${input.editorContent}`,
        kind: "context",
        pillKind: "active-file",
      },
      {
        id: "current-draft",
        label: "当前正文",
        detail: "把全文作为重点上下文",
        insertText: input.editorContent,
        kind: "context",
        pillKind: "active-file",
      },
    );
  }

  for (const file of input.workspaceFiles.slice(0, 20)) {
    if (file.path === input.selectedPath) continue;
    suggestions.push({
      id: `file:${file.path}`,
      label: file.name,
      detail: file.path,
      insertText: `路径：${file.path}`,
      kind: "context",
      pillKind: "file",
    });
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
    label: "提取记忆",
    detail: "提取人物、设定、伏笔、风格、禁区和剧本规则",
    insertText: "请从当前稿件中提取可以进入写作记忆的人物、设定、伏笔、风格、禁区和剧本规则。",
    kind: "command",
  },
];
