import { invoke } from "@tauri-apps/api/core";
import type { DraftKind } from "./promptContext";

export type CoCreateEdit = {
  target: string;
  replacement: string;
  rationale?: string | null;
};

export type CoCreateResponse = {
  reply: string;
  edits: CoCreateEdit[];
  memoriesUsed: string[];
};

export type CoCreateRequest = {
  content: string;
  draftKind: DraftKind;
  selectedText: string;
  sourcePath: string;
  title: string;
  userInput: string;
};

export async function requestCocreation(request: CoCreateRequest) {
  return invoke<CoCreateResponse>("wridian_cocreate", {
    input: {
      sourcePath: request.sourcePath || "未选择文件",
      title: request.title || "未选择文件",
      content: request.content,
      draftKind: request.draftKind,
      userInput: request.userInput,
      selectedText: request.selectedText || null,
    },
  });
}
