import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { ModelAccountsStatus, ModelProviderStatus, TestModelProviderResponse } from "../appTypes";
import {
  accessTypeLabel,
  defaultModelIds,
  presetByKey,
  protocolLabel,
  type VendorPreset,
} from "./providerCatalog";

type ProviderFormData = {
  providerId: string;
  presetKey: string;
  providerName: string;
  providerType: string;
  protocol: string;
  authStyle: string;
  baseUrl: string;
  apiKey: string;
  models: string[];
  extraEnv: Record<string, string>;
};

type GoogleGeminiOauthResponse = {
  email?: string | null;
  status: ModelAccountsStatus;
};

type ProviderOauthResponse = {
  email?: string | null;
  status: ModelAccountsStatus;
};

type AnthropicOauthStartResponse = {
  sessionId: string;
  authUrl: string;
};

type OpenAiOauthStartResponse = {
  sessionId: string;
  authUrl: string;
  userCode: string;
};

const ADD_SERVICE_SECTIONS = [
  {
    title: "授权登录",
    keys: ["anthropic-official", "openai-official", "google-gemini-cli"],
  },
  {
    title: "国内服务",
    keys: ["deepseek", "glm-cn", "kimi", "minimax-cn", "volcengine", "xiaomi-mimo-token-plan", "bailian", "bailian-token-plan-cn"],
  },
  {
    title: "第三方API",
    keys: ["anthropic-thirdparty", "openai-compatible"],
  },
];

export function ModelSettingsDialog({
  onChanged,
  onClose,
}: {
  onChanged: () => void;
  onClose: () => void;
}) {
  const [status, setStatus] = useState<ModelAccountsStatus>({ configuredModels: [], providers: [] });
  const [connectPreset, setConnectPreset] = useState<VendorPreset | null>(null);
  const [editingProvider, setEditingProvider] = useState<ModelProviderStatus | null>(null);
  const [busyProviderId, setBusyProviderId] = useState("");
  const [message, setMessage] = useState("");

  useEffect(() => {
    void loadStatus();
  }, []);

  const configuredProviderIds = new Set(status.providers.map((provider) => provider.id));

  const loadStatus = async () => {
    try {
      const next = await invoke<ModelAccountsStatus>("wridian_get_model_accounts");
      setStatus(next);
      setMessage("");
    } catch (error) {
      setMessage(error instanceof Error ? error.message : "请在 Wridian 桌面端配置模型账户。");
    }
  };

  const saveProvider = async (data: ProviderFormData) => {
    setBusyProviderId(data.presetKey);
    setMessage("");
    try {
      const next = await invoke<ModelAccountsStatus>("wridian_save_model_provider", {
        input: {
          presetKey: data.presetKey,
          providerId: data.providerId,
          providerName: data.providerName,
          providerType: data.providerType,
          protocol: data.protocol,
          authStyle: data.authStyle,
          baseUrl: data.baseUrl,
          apiKey: data.apiKey,
          models: data.models,
          extraEnv: data.extraEnv,
        },
      });
      setStatus(next);
      setConnectPreset(null);
      setEditingProvider(null);
      setMessage("已保存。");
      onChanged();
    } catch (error) {
      setMessage(error instanceof Error ? error.message : String(error));
    } finally {
      setBusyProviderId("");
    }
  };

  const deleteProvider = async (provider: ModelProviderStatus) => {
    setBusyProviderId(provider.id);
    setMessage("");
    try {
      const next = await invoke<ModelAccountsStatus>("wridian_delete_model_provider", {
        input: { providerId: provider.id },
      });
      setStatus(next);
      setMessage("已取消配置。");
      onChanged();
    } catch (error) {
      setMessage(error instanceof Error ? error.message : String(error));
    } finally {
      setBusyProviderId("");
    }
  };

  const openExternal = async (url?: string) => {
    if (!url) return;
    try {
      const { openUrl } = await import("@tauri-apps/plugin-opener");
      await openUrl(url);
    } catch (error) {
      setMessage(error instanceof Error ? error.message : String(error));
    }
  };

  return (
    <div className="modal-backdrop" onMouseDown={onClose} role="presentation">
      <section className="settings-dialog model-settings-dialog" role="dialog" aria-modal="true" aria-label="模型账户" onMouseDown={(event) => event.stopPropagation()}>
        <div className="drawer-header">
          <div>
            <div className="drawer-title">模型账户</div>
          </div>
          <button type="button" className="icon-button" onClick={onClose} aria-label="关闭">
            ×
          </button>
        </div>

        <div className="provider-manager-grid">
          <section className="provider-manager-main">
            <div className="provider-section-head">
              <div>
                <h3>已连接服务</h3>
              </div>
            </div>
            <div className="provider-card-grid connected">
              {status.providers.map((provider) => {
                const preset = presetByKey(provider.presetKey || provider.id);
                return (
                  <ProviderCard
                    key={provider.id}
                    name={provider.providerName}
                    description={preset?.descriptionZh || protocolLabel(provider.protocol)}
                    action={<button type="button" className="mini-action danger" onClick={() => void deleteProvider(provider)} disabled={busyProviderId === provider.id}>断开</button>}
                  />
                );
              })}
            </div>
          </section>

          <section className="provider-add-panel">
            <div className="provider-section-head compact">
              <div>
                <h3>添加服务</h3>
              </div>
            </div>
            <div className="provider-section-list">
              {ADD_SERVICE_SECTIONS.map((section) => {
                const presets = section.keys
                  .filter((key) => !configuredProviderIds.has(key))
                  .map((key) => presetByKey(key))
                  .filter((preset): preset is VendorPreset => Boolean(preset));
                if (!presets.length) return null;
                return (
                  <div className="provider-add-section" key={section.title}>
                    <div className="provider-add-section-title">{section.title}</div>
                    <div className="provider-preset-list">
                      {presets.map((preset) => (
                        <article
                          className="provider-preset-row"
                          key={preset.key}
                        >
                          <span>
                            <strong>{preset.name}</strong>
                            <small>{preset.descriptionZh}</small>
                          </span>
                          <button
                            type="button"
                            className="mini-action"
                            onClick={() => { setConnectPreset(preset); setEditingProvider(null); }}
                          >
                            连接
                          </button>
                        </article>
                      ))}
                    </div>
                  </div>
                );
              })}
            </div>
          </section>
        </div>

        {message ? <div className="settings-message">{message}</div> : null}

        {connectPreset ? (
          <PresetConnectDialog
            preset={connectPreset}
            provider={editingProvider}
            busy={busyProviderId === connectPreset.key}
            onClose={() => { setConnectPreset(null); setEditingProvider(null); }}
            onExternal={openExternal}
            onOauthLogin={async (preset) => {
              setBusyProviderId(preset.key);
              setMessage("");
              try {
                let response: ProviderOauthResponse;
                if (preset.key === "anthropic-official") {
                  const start = await invoke<AnthropicOauthStartResponse>("wridian_anthropic_oauth_start");
                  const code = window.prompt("Anthropic 授权完成后，把网页显示的 code 粘贴到这里。");
                  if (!code?.trim()) {
                    setMessage("已取消 Anthropic OAuth 登录。");
                    return;
                  }
                  response = await invoke<ProviderOauthResponse>("wridian_anthropic_oauth_complete", {
                    input: { sessionId: start.sessionId, code },
                  });
                } else if (preset.key === "openai-official") {
                  const start = await invoke<OpenAiOauthStartResponse>("wridian_openai_oauth_start");
                  const confirmed = window.confirm(
                    `浏览器已打开 OpenAI Codex 登录页。\n\n验证码：${start.userCode}\n\n请在网页中输入验证码并完成登录，然后点击“确定”。\n如果浏览器没有打开，请手动访问：${start.authUrl}`,
                  );
                  if (!confirmed) {
                    setMessage("已取消 OpenAI OAuth 登录。");
                    return;
                  }
                  response = await invoke<ProviderOauthResponse>("wridian_openai_oauth_complete", {
                    input: { sessionId: start.sessionId },
                  });
                } else {
                  response = await invoke<GoogleGeminiOauthResponse>("wridian_google_gemini_oauth_login");
                }
                setStatus(response.status);
                setConnectPreset(null);
                setEditingProvider(null);
                setMessage(response.email ? `${preset.name} 已登录：${response.email}` : `${preset.name} 已登录。`);
                onChanged();
              } catch (error) {
                setMessage(error instanceof Error ? error.message : String(error));
              } finally {
                setBusyProviderId("");
              }
            }}
            onSave={saveProvider}
          />
        ) : null}
      </section>
    </div>
  );
}

function ProviderCard({
  action,
  description,
  name,
}: {
  action: React.ReactNode;
  description: string;
  name: string;
}) {
  return (
    <article className="provider-card">
      <div className="provider-card-header">
        <div className="provider-card-title">
          <strong>{name}</strong>
          <small>{description}</small>
        </div>
        <div className="provider-card-actions">{action}</div>
      </div>
    </article>
  );
}

function PresetConnectDialog({
  busy,
  onClose,
  onExternal,
  onOauthLogin,
  onSave,
  preset,
  provider,
}: {
  busy: boolean;
  onClose: () => void;
  onExternal: (url?: string) => Promise<void>;
  onOauthLogin: (preset: VendorPreset) => Promise<void>;
  onSave: (data: ProviderFormData) => Promise<void>;
  preset: VendorPreset;
  provider: ModelProviderStatus | null;
}) {
  const [name, setName] = useState(provider?.providerName || preset.name);
  const [baseUrl, setBaseUrl] = useState(provider?.baseUrl || preset.baseUrl);
  const [apiKey, setApiKey] = useState("");
  const [modelsText, setModelsText] = useState((provider?.models.length ? provider.models : defaultModelIds(preset)).join("\n"));
  const [showAdvanced, setShowAdvanced] = useState(false);
  const [error, setError] = useState("");
  const [testMessage, setTestMessage] = useState("");
  const [testing, setTesting] = useState(false);

  const formData = (models: string[]): ProviderFormData => ({
    providerId: provider?.id || preset.key,
    presetKey: preset.key,
    providerName: name.trim() || preset.name,
    providerType: preset.key,
    protocol: preset.protocol,
    authStyle: preset.authStyle,
    baseUrl: baseUrl.trim() || preset.baseUrl,
    apiKey,
    models,
    extraEnv: preset.defaultEnvOverrides,
  });

  const validate = () => {
    setError("");
    setTestMessage("");
    const models = parseModels(modelsText);
    if (!models.length) {
      setError("至少需要一个模型。");
      return null;
    }
    if (preset.fields.includes("base_url") && !baseUrl.trim()) {
      setError("请填写 Base URL。");
      return null;
    }
    if (preset.authStyle === "oauth_external" && !provider?.maskedKey) {
      setError("请先点击浏览器 OAuth 登录。");
      return null;
    }
    if (preset.fields.includes("api_key") && !apiKey.trim() && !provider?.maskedKey) {
      setError(preset.authStyle === "oauth_external" ? "请先点击浏览器 OAuth 登录。" : "请填写 API Key 或 Token。");
      return null;
    }
    return formData(models);
  };

  const submit = async () => {
    const data = validate();
    if (!data) return;
    await onSave(data);
  };

  const testConnection = async () => {
    const data = validate();
    if (!data) return;
    setTesting(true);
    try {
      const response = await invoke<TestModelProviderResponse>("wridian_test_model_provider_config", {
        input: {
          providerId: data.providerId,
          providerName: data.providerName,
          protocol: data.protocol,
          authStyle: data.authStyle,
          baseUrl: data.baseUrl,
          apiKey: data.apiKey,
          models: data.models,
          extraEnv: data.extraEnv,
        },
      });
      setTestMessage(response.message || "测试通过。");
    } catch (testError) {
      setError(testError instanceof Error ? testError.message : String(testError));
    } finally {
      setTesting(false);
    }
  };

  return (
    <div className="provider-connect-backdrop" role="presentation">
      <form className="provider-connect-dialog" onSubmit={(event) => { event.preventDefault(); void submit(); }}>
        <div className="provider-connect-header">
          <div>
            <div className="provider-connect-title">{provider ? "编辑服务" : "连接服务"} · {preset.name}</div>
            <div className="provider-connect-subtitle">{preset.descriptionZh}</div>
          </div>
          <button type="button" className="icon-button" onClick={onClose} aria-label="关闭">×</button>
        </div>

        <div className="provider-connect-meta">
          <span>{protocolLabel(preset.protocol)}</span>
          <span>{accessTypeLabel(preset)}</span>
          {preset.sdkProxyOnly ? <span>Claude Code 兼容</span> : null}
        </div>

        {preset.authStyle === "oauth_external" ? (
          <div className="provider-oauth-box">
            <strong>{preset.name} OAuth</strong>
            <p>{oauthDescription(preset.key)}</p>
            <button type="button" className="secondary-action" onClick={() => void onOauthLogin(preset)} disabled={busy}>
              浏览器 OAuth 登录
            </button>
          </div>
        ) : null}

        <div className="settings-form model-settings-form">
          {preset.fields.includes("name") ? (
            <label>
              <span>名称</span>
              <input value={name} onChange={(event) => setName(event.currentTarget.value)} />
            </label>
          ) : null}
          {(preset.fields.includes("base_url") || preset.baseUrl) ? (
            <label>
              <span>Base URL</span>
              <input value={baseUrl} onChange={(event) => setBaseUrl(event.currentTarget.value)} placeholder={preset.baseUrl || "https://api.example.com/v1"} />
            </label>
          ) : null}
          {preset.fields.includes("api_key") ? (
            <label>
              <span>{preset.authStyle === "auth_token" ? "Auth Token" : preset.authStyle === "oauth_external" ? "OAuth access token" : "API Key"}</span>
              <input
                value={apiKey}
                onChange={(event) => setApiKey(event.currentTarget.value)}
                placeholder={preset.authStyle === "oauth_external" ? "OAuth 登录成功后自动保存" : provider?.maskedKey ? `已保存：${provider.maskedKey}` : "留空不会保存新凭据"}
                type="password"
                disabled={preset.authStyle === "oauth_external"}
              />
            </label>
          ) : null}
          <label>
            <span>模型列表</span>
            <textarea value={modelsText} onChange={(event) => setModelsText(event.currentTarget.value)} rows={5} />
          </label>
        </div>

        <button type="button" className="provider-advanced-toggle" onClick={() => setShowAdvanced((value) => !value)}>
          {showAdvanced ? "收起连接参数" : "查看连接参数"}
        </button>
        {showAdvanced ? (
          <div className="provider-catalog-details">
            <div><span>preset</span><strong>{preset.key}</strong></div>
            <div><span>authStyle</span><strong>{preset.authStyle}</strong></div>
            <div><span>env</span><strong>{Object.keys(preset.defaultEnvOverrides).length ? JSON.stringify(preset.defaultEnvOverrides) : "{}"}</strong></div>
            {preset.meta?.docsUrl ? <div><span>docs</span><button type="button" onClick={() => void onExternal(preset.meta?.docsUrl)}>打开文档</button></div> : null}
            {preset.meta?.apiKeyUrl ? <div><span>key/login</span><button type="button" onClick={() => void onExternal(preset.meta?.apiKeyUrl)}>打开控制台</button></div> : null}
            {preset.meta?.notes?.map((note) => <p key={note}>{note}</p>)}
          </div>
        ) : null}

        {error ? <div className="settings-message error">{error}</div> : null}
        {testMessage ? <div className="settings-message">{testMessage}</div> : null}

        <div className="settings-actions">
          <button type="button" className="secondary-action" onClick={() => void testConnection()} disabled={busy || testing}>
            {testing ? "测试中..." : "测试"}
          </button>
          <button type="button" className="secondary-action" onClick={onClose} disabled={busy}>取消</button>
          <button type="submit" className="primary-action" disabled={busy}>{busy ? "保存中..." : "保存服务"}</button>
        </div>
      </form>
    </div>
  );
}

function parseModels(value: string) {
  return value
    .split(/\r?\n|,/)
    .map((model) => model.trim())
    .filter(Boolean);
}

function oauthDescription(presetKey: string) {
  if (presetKey === "anthropic-official") {
    return "使用 Anthropic 官方账号授权。浏览器授权后，把网页显示的 code 粘贴回 Wridian。";
  }
  if (presetKey === "openai-official") {
    return "使用 OpenAI / ChatGPT 官方账号授权。浏览器中输入验证码后，Wridian 会自动保存登录状态。";
  }
  return "使用 Google 账号授权 Gemini CLI / Code Assist。本机会监听回调并自动保存登录状态。";
}
