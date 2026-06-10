export type ProviderProtocol = "anthropic" | "openai-compatible" | "google";
export type AuthStyle = "api_key" | "auth_token" | "oauth_external";
export type ProviderField = "name" | "api_key" | "base_url" | "model_names" | "model_mapping" | "env_overrides";
export type ProviderBucket = "official" | "coding-plan" | "compatible";

export type CatalogModel = {
  modelId: string;
  upstreamModelId?: string;
  displayName: string;
  role?: "default" | "haiku" | "sonnet" | "opus";
};

export type VendorPreset = {
  key: string;
  name: string;
  descriptionZh: string;
  protocol: ProviderProtocol;
  authStyle: AuthStyle;
  baseUrl: string;
  defaultEnvOverrides: Record<string, string>;
  defaultModels: CatalogModel[];
  defaultRoleModels?: Record<string, string>;
  fields: ProviderField[];
  bucket: ProviderBucket;
  iconKey: string;
  sdkProxyOnly?: boolean;
  meta?: {
    apiKeyUrl?: string;
    docsUrl?: string;
    billingModel: "pay_as_you_go" | "coding_plan" | "token_plan" | "oauth" | "gateway";
    notes?: string[];
  };
};

const ANTHROPIC_DEFAULT_MODELS: CatalogModel[] = [
  { modelId: "sonnet", displayName: "Sonnet 4.6", role: "sonnet" },
  { modelId: "opus", displayName: "Opus 4.7", role: "opus" },
  { modelId: "haiku", displayName: "Haiku 4.5", role: "haiku" },
];

export const VENDOR_PRESETS: VendorPreset[] = [
  {
    key: "anthropic-official",
    name: "Anthropic",
    descriptionZh: "Claude 官方账号 OAuth",
    protocol: "anthropic",
    authStyle: "oauth_external",
    baseUrl: "https://api.anthropic.com",
    defaultEnvOverrides: {},
    defaultModels: ANTHROPIC_DEFAULT_MODELS,
    fields: ["model_names"],
    bucket: "official",
    iconKey: "anthropic",
    meta: {
      apiKeyUrl: "https://claude.ai/oauth/authorize",
      docsUrl: "https://platform.claude.com/docs/en/api/overview",
      billingModel: "oauth",
      notes: [
        "授权地址：claude.ai/oauth/authorize",
        "Token URL: https://console.anthropic.com/v1/oauth/token",
        "Scopes: org:create_api_key, user:profile, user:inference",
      ],
    },
  },
  {
    key: "openai-official",
    name: "OpenAI",
    descriptionZh: "ChatGPT / Codex 官方账号 OAuth",
    protocol: "openai-compatible",
    authStyle: "oauth_external",
    baseUrl: "https://chatgpt.com/backend-api/codex",
    defaultEnvOverrides: {},
    defaultModels: [
      { modelId: "gpt-5.5", displayName: "GPT-5.5", role: "default" },
      { modelId: "gpt-5.4", displayName: "GPT-5.4" },
      { modelId: "gpt-5.4-mini", displayName: "GPT-5.4 Mini", role: "haiku" },
      { modelId: "gpt-5.3-codex", displayName: "GPT-5.3 Codex" },
      { modelId: "gpt-5.3-codex-spark", displayName: "GPT-5.3 Codex Spark" },
    ],
    fields: ["model_names"],
    bucket: "official",
    iconKey: "openai",
    meta: {
      apiKeyUrl: "https://auth.openai.com/oauth/authorize",
      docsUrl: "https://platform.openai.com/docs",
      billingModel: "oauth",
      notes: [
        "授权地址：auth.openai.com/oauth/authorize",
        "Token URL: https://auth.openai.com/oauth/token",
        "Runtime URL: https://chatgpt.com/backend-api/codex/responses",
      ],
    },
  },
  {
    key: "gemini",
    name: "Google Gemini",
    descriptionZh: "Google AI Studio Gemini API Key",
    protocol: "google",
    authStyle: "api_key",
    baseUrl: "https://generativelanguage.googleapis.com/v1beta",
    defaultEnvOverrides: {},
    defaultModels: [
      { modelId: "gemini-2.5-pro", displayName: "Gemini 2.5 Pro", role: "default" },
      { modelId: "gemini-2.5-flash", displayName: "Gemini 2.5 Flash", role: "haiku" },
      { modelId: "gemini-2.0-flash", displayName: "Gemini 2.0 Flash" },
    ],
    fields: ["api_key", "model_names"],
    bucket: "official",
    iconKey: "google",
    meta: {
      apiKeyUrl: "https://aistudio.google.com/api-keys",
      docsUrl: "https://ai.google.dev/gemini-api/docs",
      billingModel: "pay_as_you_go",
    },
  },
  {
    key: "google-gemini-cli",
    name: "Gemini",
    descriptionZh: "Google 账号 OAuth 登录",
    protocol: "google",
    authStyle: "oauth_external",
    baseUrl: "https://generativelanguage.googleapis.com/v1beta",
    defaultEnvOverrides: {},
    defaultModels: [
      { modelId: "gemini-2.5-pro", displayName: "Gemini 2.5 Pro", role: "default" },
      { modelId: "gemini-2.5-flash", displayName: "Gemini 2.5 Flash", role: "haiku" },
    ],
    fields: ["model_names"],
    bucket: "official",
    iconKey: "google",
    meta: {
      apiKeyUrl: "https://accounts.google.com/o/oauth2/v2/auth",
      docsUrl: "https://ai.google.dev/gemini-api/docs",
      billingModel: "oauth",
      notes: [
        "OAuth URL: https://accounts.google.com/o/oauth2/v2/auth",
        "Token URL: https://oauth2.googleapis.com/token",
        "Redirect: http://localhost:8085/oauth2callback",
        "Scopes: cloud-platform, userinfo.email",
      ],
    },
  },
  {
    key: "anthropic-thirdparty",
    name: "Anthropic Third-party API",
    descriptionZh: "Anthropic 兼容第三方 API",
    protocol: "anthropic",
    authStyle: "api_key",
    baseUrl: "",
    defaultEnvOverrides: { ANTHROPIC_API_KEY: "" },
    defaultModels: ANTHROPIC_DEFAULT_MODELS,
    fields: ["name", "api_key", "base_url", "model_mapping", "env_overrides"],
    bucket: "compatible",
    iconKey: "anthropic",
    meta: { billingModel: "gateway" },
  },
  {
    key: "openai-compatible",
    name: "OpenAI-Compatible API",
    descriptionZh: "OpenAI 兼容第三方 API",
    protocol: "openai-compatible",
    authStyle: "api_key",
    baseUrl: "",
    defaultEnvOverrides: {},
    defaultModels: [],
    fields: ["name", "api_key", "base_url", "model_names"],
    bucket: "compatible",
    iconKey: "openai",
    meta: { billingModel: "gateway" },
  },
  {
    key: "glm-cn",
    name: "GLM (CN)",
    descriptionZh: "智谱 GLM 编程套餐，中国区",
    protocol: "anthropic",
    authStyle: "auth_token",
    baseUrl: "https://open.bigmodel.cn/api/anthropic",
    defaultEnvOverrides: { API_TIMEOUT_MS: "3000000", ANTHROPIC_DEFAULT_HAIKU_MODEL: "glm-4.5-air", ANTHROPIC_DEFAULT_SONNET_MODEL: "glm-5-turbo", ANTHROPIC_DEFAULT_OPUS_MODEL: "glm-5.1" },
    defaultModels: [
      { modelId: "sonnet", upstreamModelId: "sonnet", displayName: "GLM-5-Turbo", role: "sonnet" },
      { modelId: "opus", upstreamModelId: "opus", displayName: "GLM-5.1", role: "opus" },
      { modelId: "haiku", upstreamModelId: "haiku", displayName: "GLM-4.5-Air", role: "haiku" },
    ],
    fields: ["api_key"],
    bucket: "coding-plan",
    iconKey: "zhipu",
    sdkProxyOnly: true,
    meta: { apiKeyUrl: "https://bigmodel.cn/usercenter/proj-mgmt/apikeys", docsUrl: "https://docs.bigmodel.cn/cn/coding-plan/tool/claude", billingModel: "coding_plan", notes: ["高峰时段（14:00-18:00 UTC+8）消耗 3 倍积分"] },
  },
  {
    key: "glm-global",
    name: "GLM (Global)",
    descriptionZh: "智谱 GLM 编程套餐，国际区",
    protocol: "anthropic",
    authStyle: "auth_token",
    baseUrl: "https://api.z.ai/api/anthropic",
    defaultEnvOverrides: { API_TIMEOUT_MS: "3000000", ANTHROPIC_DEFAULT_HAIKU_MODEL: "glm-4.5-air", ANTHROPIC_DEFAULT_SONNET_MODEL: "glm-5-turbo", ANTHROPIC_DEFAULT_OPUS_MODEL: "glm-5.1" },
    defaultModels: [
      { modelId: "sonnet", upstreamModelId: "sonnet", displayName: "GLM-5-Turbo", role: "sonnet" },
      { modelId: "opus", upstreamModelId: "opus", displayName: "GLM-5.1", role: "opus" },
      { modelId: "haiku", upstreamModelId: "haiku", displayName: "GLM-4.5-Air", role: "haiku" },
    ],
    fields: ["api_key"],
    bucket: "coding-plan",
    iconKey: "zhipu",
    sdkProxyOnly: true,
    meta: { apiKeyUrl: "https://z.ai/manage-apikey/apikey-list", docsUrl: "https://docs.z.ai/devpack/tool/claude", billingModel: "coding_plan", notes: ["高峰时段（14:00-18:00 UTC+8）消耗 3 倍积分"] },
  },
  {
    key: "kimi",
    name: "Kimi Coding Plan",
    descriptionZh: "Kimi 编程计划 API",
    protocol: "anthropic",
    authStyle: "api_key",
    baseUrl: "https://api.kimi.com/coding/",
    defaultEnvOverrides: { ENABLE_TOOL_SEARCH: "false" },
    defaultModels: [{ modelId: "sonnet", displayName: "Kimi K2.5", role: "default" }],
    fields: ["api_key"],
    bucket: "coding-plan",
    iconKey: "kimi",
    sdkProxyOnly: true,
    meta: { apiKeyUrl: "https://www.kimi.com/code/console", docsUrl: "https://www.kimi.com/code/docs/more/third-party-agents.html", billingModel: "pay_as_you_go" },
  },
  {
    key: "minimax-cn",
    name: "MiniMax (CN)",
    descriptionZh: "MiniMax 编程套餐，中国区",
    protocol: "anthropic",
    authStyle: "auth_token",
    baseUrl: "https://api.minimaxi.com/anthropic",
    defaultEnvOverrides: { API_TIMEOUT_MS: "3000000", CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC: "1" },
    defaultModels: [{ modelId: "sonnet", upstreamModelId: "MiniMax-M2.7", displayName: "MiniMax-M2.7", role: "default" }],
    defaultRoleModels: { default: "MiniMax-M2.7", sonnet: "MiniMax-M2.7", opus: "MiniMax-M2.7", haiku: "MiniMax-M2.7" },
    fields: ["api_key"],
    bucket: "coding-plan",
    iconKey: "minimax",
    sdkProxyOnly: true,
    meta: { apiKeyUrl: "https://platform.minimaxi.com/user-center/payment/token-plan", docsUrl: "https://platform.minimaxi.com/docs/token-plan/claude-code", billingModel: "token_plan" },
  },
  {
    key: "minimax-global",
    name: "MiniMax (Global)",
    descriptionZh: "MiniMax 编程套餐，国际区",
    protocol: "anthropic",
    authStyle: "auth_token",
    baseUrl: "https://api.minimax.io/anthropic",
    defaultEnvOverrides: { API_TIMEOUT_MS: "3000000", CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC: "1" },
    defaultModels: [{ modelId: "sonnet", upstreamModelId: "MiniMax-M2.7", displayName: "MiniMax-M2.7", role: "default" }],
    defaultRoleModels: { default: "MiniMax-M2.7", sonnet: "MiniMax-M2.7", opus: "MiniMax-M2.7", haiku: "MiniMax-M2.7" },
    fields: ["api_key"],
    bucket: "coding-plan",
    iconKey: "minimax",
    sdkProxyOnly: true,
    meta: { apiKeyUrl: "https://platform.minimax.io/user-center/payment/token-plan", docsUrl: "https://platform.minimax.io/docs/token-plan/opencode", billingModel: "token_plan" },
  },
  {
    key: "volcengine",
    name: "Volcengine Ark",
    descriptionZh: "火山方舟 Coding Plan",
    protocol: "anthropic",
    authStyle: "auth_token",
    baseUrl: "https://ark.cn-beijing.volces.com/api/coding",
    defaultEnvOverrides: {},
    defaultModels: [
      { modelId: "doubao-seed-2.0-code", displayName: "Doubao Seed 2.0 Code", role: "default" },
      { modelId: "doubao-seed-2.0-pro", displayName: "Doubao Seed 2.0 Pro" },
      { modelId: "doubao-seed-2.0-lite", displayName: "Doubao Seed 2.0 Lite" },
      { modelId: "doubao-seed-code", displayName: "Doubao Seed Code" },
      { modelId: "minimax-m2.5", displayName: "MiniMax M2.5" },
      { modelId: "glm-4.7", displayName: "GLM-4.7" },
      { modelId: "deepseek-v3.2", displayName: "DeepSeek V3.2" },
      { modelId: "kimi-k2.5", displayName: "Kimi K2.5" },
      { modelId: "ark-code-latest", displayName: "ark-code-latest (Console-managed / Auto)" },
    ],
    fields: ["api_key", "model_names"],
    bucket: "coding-plan",
    iconKey: "volcengine",
    sdkProxyOnly: true,
    meta: { apiKeyUrl: "https://console.volcengine.com/ark/region:ark+cn-beijing/openManagement", docsUrl: "https://www.volcengine.com/docs/82379/1928262", billingModel: "coding_plan", notes: ["需先在控制台激活 Endpoint", "API Key 为临时凭证"] },
  },
  {
    key: "xiaomi-mimo",
    name: "Xiaomi MiMo",
    descriptionZh: "小米 MiMo 按量付费",
    protocol: "anthropic",
    authStyle: "auth_token",
    baseUrl: "https://api.xiaomimimo.com/anthropic",
    defaultEnvOverrides: {},
    defaultModels: [
      { modelId: "sonnet", upstreamModelId: "mimo-v2.5-pro", displayName: "MiMo-V2.5-Pro", role: "default" },
      { modelId: "mimo-v2.5-pro-ultraspeed", upstreamModelId: "mimo-v2.5-pro-ultraspeed", displayName: "MiMo-V2.5-Pro-UltraSpeed" },
    ],
    defaultRoleModels: { default: "mimo-v2.5-pro", sonnet: "mimo-v2.5-pro", opus: "mimo-v2.5-pro", haiku: "mimo-v2.5-pro" },
    fields: ["api_key", "model_names"],
    bucket: "coding-plan",
    iconKey: "xiaomi-mimo",
    sdkProxyOnly: true,
    meta: { apiKeyUrl: "https://platform.xiaomimimo.com/#/console/api-keys", docsUrl: "https://platform.xiaomimimo.com/#/docs/integration/claudecode", billingModel: "pay_as_you_go" },
  },
  {
    key: "xiaomi-mimo-token-plan",
    name: "Xiaomi MiMo Token Plan",
    descriptionZh: "小米 MiMo Token Plan 订阅套餐",
    protocol: "anthropic",
    authStyle: "auth_token",
    baseUrl: "https://token-plan-cn.xiaomimimo.com/anthropic",
    defaultEnvOverrides: {},
    defaultModels: [
      { modelId: "sonnet", upstreamModelId: "mimo-v2.5-pro", displayName: "MiMo-V2.5-Pro", role: "default" },
    ],
    defaultRoleModels: { default: "mimo-v2.5-pro", sonnet: "mimo-v2.5-pro", opus: "mimo-v2.5-pro", haiku: "mimo-v2.5-pro" },
    fields: ["api_key", "model_names"],
    bucket: "coding-plan",
    iconKey: "xiaomi-mimo",
    sdkProxyOnly: true,
    meta: { apiKeyUrl: "https://platform.xiaomimimo.com/#/console/plan-manage", docsUrl: "https://platform.xiaomimimo.com/#/docs/integration/claudecode", billingModel: "token_plan" },
  },
  {
    key: "bailian",
    name: "Aliyun Bailian",
    descriptionZh: "阿里云百炼 Coding Plan - 通义千问、GLM、Kimi、MiniMax",
    protocol: "anthropic",
    authStyle: "auth_token",
    baseUrl: "https://coding.dashscope.aliyuncs.com/apps/anthropic",
    defaultEnvOverrides: {},
    defaultModels: [
      { modelId: "qwen3.6-plus", displayName: "Qwen 3.6 Plus", role: "default" },
      { modelId: "qwen3.5-plus", displayName: "Qwen 3.5 Plus" },
      { modelId: "qwen3-max-2026-01-23", displayName: "Qwen 3 Max (2026-01-23)" },
      { modelId: "qwen3-coder-next", displayName: "Qwen 3 Coder Next" },
      { modelId: "qwen3-coder-plus", displayName: "Qwen 3 Coder Plus" },
      { modelId: "kimi-k2.5", displayName: "Kimi K2.5" },
      { modelId: "glm-5", displayName: "GLM-5" },
      { modelId: "glm-4.7", displayName: "GLM-4.7" },
      { modelId: "MiniMax-M2.5", displayName: "MiniMax-M2.5" },
    ],
    fields: ["api_key"],
    bucket: "coding-plan",
    iconKey: "bailian",
    sdkProxyOnly: true,
    meta: { apiKeyUrl: "https://bailian.console.aliyun.com", docsUrl: "https://help.aliyun.com/zh/model-studio/coding-plan", billingModel: "coding_plan", notes: ["必须使用 Coding Plan 专用 Key（以 sk-sp- 开头）", "普通 DashScope Key 无法使用"] },
  },
  {
    key: "bailian-token-plan-cn",
    name: "Aliyun Bailian Token Plan",
    descriptionZh: "阿里云百炼 Token Plan 团队版",
    protocol: "anthropic",
    authStyle: "auth_token",
    baseUrl: "https://token-plan.cn-beijing.maas.aliyuncs.com/apps/anthropic",
    defaultEnvOverrides: {},
    defaultModels: [
      { modelId: "qwen3.6-plus", displayName: "Qwen 3.6 Plus", role: "default" },
      { modelId: "glm-5", displayName: "GLM-5" },
      { modelId: "MiniMax-M2.5", displayName: "MiniMax-M2.5" },
    ],
    defaultRoleModels: { default: "qwen3.6-plus", sonnet: "qwen3.6-plus", opus: "qwen3.6-plus", haiku: "qwen3.6-plus" },
    fields: ["api_key"],
    bucket: "coding-plan",
    iconKey: "bailian",
    sdkProxyOnly: true,
    meta: { apiKeyUrl: "https://bailian.console.aliyun.com", docsUrl: "https://help.aliyun.com/zh/model-studio/token-plan", billingModel: "token_plan" },
  },
  {
    key: "deepseek",
    name: "DeepSeek",
    descriptionZh: "DeepSeek Anthropic 兼容 API",
    protocol: "anthropic",
    authStyle: "auth_token",
    baseUrl: "https://api.deepseek.com/anthropic",
    defaultEnvOverrides: { CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC: "1", CLAUDE_CODE_DISABLE_NONSTREAMING_FALLBACK: "1" },
    defaultModels: [
      { modelId: "deepseek-v4-pro[1m]", upstreamModelId: "deepseek-v4-pro[1m]", displayName: "DeepSeek V4 Pro (1M)", role: "opus" },
      { modelId: "deepseek-v4-pro", upstreamModelId: "deepseek-v4-pro", displayName: "DeepSeek V4 Pro", role: "default" },
      { modelId: "deepseek-v4-flash", upstreamModelId: "deepseek-v4-flash", displayName: "DeepSeek V4 Flash", role: "haiku" },
    ],
    defaultRoleModels: { default: "deepseek-v4-pro[1m]", opus: "deepseek-v4-pro[1m]", sonnet: "deepseek-v4-pro[1m]", haiku: "deepseek-v4-flash" },
    fields: ["api_key"],
    bucket: "coding-plan",
    iconKey: "deepseek",
    sdkProxyOnly: true,
    meta: { apiKeyUrl: "https://platform.deepseek.com/api_keys", docsUrl: "https://api-docs.deepseek.com", billingModel: "pay_as_you_go" },
  },
];

export function presetByKey(key: string) {
  return VENDOR_PRESETS.find((preset) => preset.key === key);
}

export function defaultModelIds(preset: VendorPreset) {
  return preset.defaultModels.map((model) => model.upstreamModelId || model.modelId);
}

export function accessTypeLabel(preset: VendorPreset) {
  if (preset.authStyle === "oauth_external") return "授权登录";
  if (preset.meta?.billingModel === "coding_plan" || preset.meta?.billingModel === "token_plan") return "套餐 Token";
  if (preset.bucket === "compatible") return "中转网关";
  return "API Key";
}

export function protocolLabel(protocol: ProviderProtocol | string) {
  if (protocol === "anthropic") return "Anthropic";
  if (protocol === "google") return "Google Gemini";
  return "OpenAI-compatible";
}
