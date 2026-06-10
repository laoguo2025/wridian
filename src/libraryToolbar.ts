export type LibraryTab = "works" | "knowledge";

export function libraryFolderTooltip(tab: LibraryTab): string {
  return tab === "knowledge" ? "选择知识库文件夹" : "选择作品库文件夹";
}
