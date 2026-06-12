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

export async function getProjectState() {
  return invoke<ProjectState>("wridian_get_project_state");
}

export async function selectProject(id: string | null) {
  return invoke<ProjectState>("wridian_select_project", { input: { id } });
}
