import { invoke } from "@tauri-apps/api/core";

export type ProjectConfig = {
  id: string;
  name: string;
  description: string;
  model?: string | null;
  systemPrompt: string;
  inclusions: string[];
  exclusions: string[];
  webUrls: string[];
  updatedAt: string;
};

export type ProjectState = {
  activeProjectId?: string | null;
  projects: ProjectConfig[];
};

export type RelevantNote = {
  path: string;
  title: string;
  snippet: string;
  score: number;
  hasOutgoingLinks: boolean;
  hasBacklinks: boolean;
};

export async function getProjectState() {
  return invoke<ProjectState>("wridian_get_project_state");
}

export async function saveProject(input: {
  id?: string;
  name: string;
  description?: string;
  model?: string;
  systemPrompt?: string;
  inclusions?: string[];
  exclusions?: string[];
  webUrls?: string[];
}) {
  return invoke<ProjectState>("wridian_save_project", { input });
}

export async function selectProject(id: string | null) {
  return invoke<ProjectState>("wridian_select_project", { input: { id } });
}

export async function findRelevantNotes(input: {
  sourcePath: string;
  content: string;
  query?: string;
  limit?: number;
}) {
  return invoke<RelevantNote[]>("wridian_find_relevant_notes", { input });
}
