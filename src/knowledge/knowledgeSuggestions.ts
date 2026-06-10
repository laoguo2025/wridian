import type { WorkFileNode } from "../appTypes";

export type KnowledgeCategory = {
  detail: string;
  id: string;
  title: string;
};

export type KnowledgeCardSuggestion = {
  category: string;
  categoryId: string;
  id: string;
  relativePath: string;
  sourcePath: string;
  title: string;
};

export function buildKnowledgeSuggestionIndex(nodes: WorkFileNode[]) {
  const categories = new Map<string, KnowledgeCategory>();
  const cards: KnowledgeCardSuggestion[] = [];
  const visit = (node: WorkFileNode, categoryId = "") => {
    if (node.folder) {
      const id = node.relativePath || node.path;
      categories.set(id, {
        detail: `${countMarkdownCards(node)} 张知识卡`,
        id,
        title: node.name,
      });
      node.children.forEach((child) => visit(child, id));
      return;
    }
    if (!/\.(md|markdown)$/i.test(node.name)) return;
    const fallbackCategory = categoryId || "__root__";
    if (!categoryId && !categories.has(fallbackCategory)) {
      categories.set(fallbackCategory, {
        detail: "知识库根目录",
        id: fallbackCategory,
        title: "根目录",
      });
    }
    cards.push({
      category: categories.get(fallbackCategory)?.title ?? "知识卡",
      categoryId: fallbackCategory,
      id: node.path,
      relativePath: node.relativePath,
      sourcePath: node.path,
      title: node.name.replace(/\.(md|markdown)$/i, ""),
    });
  };
  nodes.forEach((node) => visit(node));
  return {
    cards,
    categories: [...categories.values()].filter((category) =>
      cards.some((card) => card.categoryId === category.id),
    ),
  };
}

function countMarkdownCards(node: WorkFileNode): number {
  if (!node.folder) return /\.(md|markdown)$/i.test(node.name) ? 1 : 0;
  return node.children.reduce((total, child) => total + countMarkdownCards(child), 0);
}
