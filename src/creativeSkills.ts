export type CreativeSkillId = "knowledgeOps" | "workDecompose" | "knowledgeCard" | "authorDistill";

export type CreativeSkill = {
  id: CreativeSkillId;
  title: string;
  status: string;
  prompt: string;
};

const BUILTIN_SKILL_PROTOCOL = "Wridian 内置技能资源";

export const CREATIVE_SKILLS: CreativeSkill[] = [
  {
    id: "knowledgeOps",
    title: "知识库运维",
    status: "体检知识库结构、质量和调用记录，给出清理与进化建议。",
    prompt: `${BUILTIN_SKILL_PROTOCOL}：知识库运维。`,
  },
  {
    id: "workDecompose",
    title: "作品拆解",
    status: "分析作品结构、人物、冲突、节奏和可复用写法。",
    prompt: `${BUILTIN_SKILL_PROTOCOL}：作品拆解。`,
  },
  {
    id: "knowledgeCard",
    title: "知识卡提炼",
    status: "把材料提炼成可复用的知识卡和写作方法。",
    prompt: `${BUILTIN_SKILL_PROTOCOL}：知识卡提炼。`,
  },
  {
    id: "authorDistill",
    title: "大神蒸馏",
    status: "从多部作品中提炼作者稳定的创作基因。",
    prompt: `${BUILTIN_SKILL_PROTOCOL}：大神蒸馏。`,
  },
];

export const DEFAULT_CREATIVE_SKILL_STATE: Record<CreativeSkillId, boolean> = {
  knowledgeOps: true,
  workDecompose: true,
  knowledgeCard: true,
  authorDistill: true,
};
