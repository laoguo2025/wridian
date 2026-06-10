export type CreativeSkillId = "knowledgeOps" | "workDecompose" | "knowledgeCard" | "authorDistill";

export type CreativeSkill = {
  id: CreativeSkillId;
  title: string;
  status: string;
  prompt: string;
};

export const CREATIVE_SKILLS: CreativeSkill[] = [
  {
    id: "knowledgeOps",
    title: "知识库运维",
    status: "接入 zhishiku-skill 规则",
    prompt: "请调用知识库运维技能，按当前知识库真实目录做体检；先给出问题清单、风险等级和建议动作，不要直接改动文件。",
  },
  {
    id: "workDecompose",
    title: "作品拆解",
    status: "拆解报告与案例分析",
    prompt: "请调用作品拆解技能，围绕当前作品或选中内容做结构、人物、冲突、节奏和可复用方法拆解。",
  },
  {
    id: "knowledgeCard",
    title: "知识卡提炼",
    status: "将知识卡打造成可复用skill",
    prompt: "请调用知识卡提炼技能，把当前材料提炼成可复用 skill：说明输入、处理逻辑、输出结果、使用场景和失效边界。",
  },
  {
    id: "authorDistill",
    title: "大神蒸馏",
    status: "至少2部作品，即可将作者的创作基因蒸馏成skill",
    prompt: "请调用大神蒸馏技能，基于至少2部作品拆解，提炼作者的稳定创作基因、方法论和可复用 skill 草案。",
  },
];

export const DEFAULT_CREATIVE_SKILL_STATE: Record<CreativeSkillId, boolean> = {
  knowledgeOps: true,
  workDecompose: true,
  knowledgeCard: true,
  authorDistill: true,
};
