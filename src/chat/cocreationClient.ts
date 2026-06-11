import { invoke } from "@tauri-apps/api/core";
import type { DraftKind, PromptContextLoadStatus, PromptContextPill } from "./promptContext";

export type CoCreateEdit = {
  target: string;
  replacement: string;
  rationale?: string | null;
};

export type CoCreateResponse = {
  contextLoadStatus: PromptContextLoadStatus[];
  reply: string;
  edits: CoCreateEdit[];
  memoriesUsed: string[];
  memoriesWritten: string[];
};

export type CoCreateRequest = {
  content: string;
  contextItems: PromptContextPill[];
  draftKind: DraftKind;
  requestId: string;
  selectedModelId?: string;
  selectedText: string;
  sourcePath: string;
  title: string;
  userInput: string;
};

export async function requestCocreation(request: CoCreateRequest) {
  return invoke<CoCreateResponse>("wridian_cocreate", {
    input: {
      requestId: request.requestId,
      sourcePath: request.sourcePath || "未选择文件",
      title: request.title || "未选择文件",
      content: request.content,
      contextItems: request.contextItems.map((item) => ({
        kind: item.kind,
        label: item.label,
        relativePath: item.relativePath ?? null,
        sourcePath: item.sourcePath ?? null,
        value: item.value,
      })),
      draftKind: request.draftKind,
      selectedModelId: request.selectedModelId || null,
      userInput: request.userInput,
      selectedText: request.selectedText || null,
    },
  });
}

export async function abortCocreation(requestId: string) {
  if (!requestId.trim()) return;
  await invoke("wridian_abort_cocreate", {
    input: { requestId },
  });
}
