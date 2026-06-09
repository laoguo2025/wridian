export type LibraryTab = "works" | "knowledge";

export type LibraryToolbarWorkspace = {
  activeWorkRoot?: string | null;
  filesRootPath?: string | null;
  knowledgeRootPath?: string | null;
  vaultPath?: string | null;
};

export function libraryFolderTooltip(tab: LibraryTab): string {
  return tab === "knowledge" ? "打开本地知识库" : "打开本地作品库";
}

export function libraryFolderPath(tab: LibraryTab, workspace: LibraryToolbarWorkspace | null): string {
  if (!workspace) return "";
  if (tab === "knowledge") return workspace.knowledgeRootPath ?? "";
  return workspace.filesRootPath || workspace.activeWorkRoot || workspace.vaultPath || "";
}
