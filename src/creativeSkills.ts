export type CreativeSkillId = "workDecompose" | "knowledgeCard" | "authorDistill";

export type CreativeSkillWorkflow = {
  inputs: string[];
  outputs: string[];
  qualityGates: string[];
  rollback: string;
};

export type CreativeSkill = {
  id: CreativeSkillId;
  title: string;
  status: string;
  prompt: string;
  workflow: CreativeSkillWorkflow;
};

const BUILTIN_SKILL_PROTOCOL = "Wridian 内置技能资源";

export const CREATIVE_SKILLS: CreativeSkill[] = [
  {
    id: "workDecompose",
    title: "作品拆解",
    status: "分析作品结构、人物、冲突、节奏和可复用写法。",
    prompt: `${BUILTIN_SKILL_PROTOCOL}：作品拆解。`,
    workflow: {
      inputs: ["作品源文件或片段", "媒介类型", "拆解目标和覆盖范围"],
      outputs: ["02 拆解案例目录", "分析笔记", "综合报告与候选分级"],
      qualityGates: ["覆盖声明完整", "关键判断有证据锚点", "A/B/C 候选分级清楚"],
      rollback: "新增报告和笔记只写入 02 拆解报告，错误产物可归档或移到回收站。",
    },
  },
  {
    id: "knowledgeCard",
    title: "知识卡提炼",
    status: "把材料提炼成可复用的知识卡和写作方法。",
    prompt: `${BUILTIN_SKILL_PROTOCOL}：知识卡提炼。`,
    workflow: {
      inputs: ["合格拆解报告", "A 级候选", "已有 03-07 知识卡"],
      outputs: ["S 级知识卡", "A/B 候选处理记录", "调用记录更新"],
      qualityGates: ["输入/处理逻辑/输出/场景/边界齐全", "编译来源可定位", "不把剧情摘要写成知识卡"],
      rollback: "新增或更新卡片通过 fileOperations 写入，可按相对路径追加修正或移回归档。",
    },
  },
  {
    id: "authorDistill",
    title: "大神蒸馏",
    status: "从多部作品中提炼作者稳定的创作基因。",
    prompt: `${BUILTIN_SKILL_PROTOCOL}：大神蒸馏。`,
    workflow: {
      inputs: ["同作者多部合格拆解", "目标用途", "已有作者 skill"],
      outputs: ["08 大神蒸馏作者 skill", "自包含案例证据", "版本记录"],
      qualityGates: ["跨作品复现", "最终 skill 无本地绝对路径", "有失效域和可执行工作流"],
      rollback: "作者 skill 写入 08 大神蒸馏目录，覆盖前保留版本记录并可从归档恢复。",
    },
  },
];

export const DEFAULT_CREATIVE_SKILL_STATE: Record<CreativeSkillId, boolean> = {
  workDecompose: true,
  knowledgeCard: true,
  authorDistill: true,
};
