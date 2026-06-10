export type CreativeSkillId = "knowledgeOps" | "workDecompose" | "knowledgeCard" | "authorDistill";

export type CreativeSkill = {
  id: CreativeSkillId;
  title: string;
  status: string;
  prompt: string;
};

export function buildCreativeSkillContext(
  skill: CreativeSkill,
  input: {
    knowledgeRootPath?: string;
    skillAvailable?: boolean;
    skillPath?: string;
  },
) {
  if (skill.id !== "knowledgeOps") {
    return "";
  }
  return [
    "Wridian 技能协议：知识库运维",
    `当前知识库根目录：${input.knowledgeRootPath || "未定位"}`,
    `本机 zhishiku-skill：${input.skillAvailable ? input.skillPath || "已识别" : "未识别，仅使用内置最小协议"}`,
    "",
    "执行边界：",
    "- 文件系统是唯一事实来源，以当前知识库真实目录为准。",
    "- 不使用未结构化文件候选箱；01原始资料可以保持未加工状态。",
    "- 02拆解报告保存分析产物和 A/B/C 候选。",
    "- 03-07 只接收通过 zhishiku-skill / tilian-skill 质量闸门的 S 级知识卡。",
    "- 清理默认给出归档建议，目标是 09文件归档；不得建议直接物理删除。",
    "- 高风险语义改写、批量移动、覆盖文件必须先列清单并等待用户确认。",
    "",
    "运维检查重点：",
    "- 00-09 一级目录是否完整，00知识库治理是否有治理说明和调用记录台账。",
    "- 03-07 知识卡是否具备输入、处理逻辑、输出结果、使用场景和失效边界。",
    "- 调用记录台账是否能形成质量 × 频率四象限和下一步进化动作。",
    "- 08大神蒸馏中的作者 skill 是否有索引、安装记录和版本记录。",
  ].join("\n");
}

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
