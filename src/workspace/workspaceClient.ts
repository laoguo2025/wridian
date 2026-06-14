import { invoke } from "@tauri-apps/api/core";
import type { OpenFileResponse, PreviewAssetResponse, PreviewFileResponse, SaveFileResponse, WorkspaceInfo } from "../appTypes";

export function initWorkspace() {
  return invoke<WorkspaceInfo>("wridian_init_workspace");
}

export function setLibraryRoot(path: string, library: "works" | "knowledge") {
  const command = library === "knowledge" ? "wridian_set_knowledge_root" : "wridian_set_work_root";
  return invoke<WorkspaceInfo>(command, { input: { path } });
}

export function openWorkFile(path: string) {
  return invoke<OpenFileResponse>("wridian_open_file", { input: { path } });
}

export function previewWorkFile(path: string) {
  return invoke<PreviewFileResponse>("wridian_preview_file", { input: { path } });
}

export function previewWorkAsset(path: string) {
  return invoke<PreviewAssetResponse>("wridian_preview_asset", { input: { path } });
}

export function saveWorkFile(path: string, content: string) {
  return invoke<SaveFileResponse>("wridian_save_file", { input: { path, content } });
}

export type WorkspaceLibrary = "works" | "knowledge";

export function createWorkFile(parentPath: string, name: string, library: WorkspaceLibrary) {
  return invoke<WorkspaceInfo>("wridian_create_work_file", { input: { library, parentPath, name } });
}

export function createWorkFolder(parentPath: string, name: string, library: WorkspaceLibrary) {
  return invoke<WorkspaceInfo>("wridian_create_work_folder", { input: { library, parentPath, name } });
}

export function duplicateWorkNode(path: string) {
  return invoke<WorkspaceInfo>("wridian_duplicate_work_node", { input: { path } });
}

export function renameWorkNode(path: string, newName: string, library: WorkspaceLibrary) {
  return invoke<WorkspaceInfo>("wridian_rename_work_node", { input: { library, path, newName } });
}

export function trashWorkNode(path: string) {
  return invoke<WorkspaceInfo>("wridian_trash_work_node", { input: { path } });
}
