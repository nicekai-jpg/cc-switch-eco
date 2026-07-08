import { useTranslation } from "react-i18next";
import { ClaudeFormFields } from "./ClaudeFormFields";
import { CodexFormFields } from "./CodexFormFields";
import { GeminiFormFields } from "./GeminiFormFields";
import { OpenCodeFormFields } from "./OpenCodeFormFields";
import { OmoFormFields } from "./OmoFormFields";
import { OpenClawFormFields } from "./OpenClawFormFields";
import { HermesFormFields } from "./HermesFormFields";
import CodexConfigEditor from "./CodexConfigEditor";
import GeminiConfigEditor from "./GeminiConfigEditor";
import { CommonConfigEditor } from "./CommonConfigEditor";
import JsonEditor from "@/components/JsonEditor";
import { Label } from "@/components/ui/label";
import { FormField, FormItem, FormMessage } from "@/components/ui/form";
import { hasApiKeyField } from "@/utils/providerConfigUtils";
import type { UseFormReturn } from "react-hook-form";
import type { ProviderFormData } from "@/lib/schemas/provider";
import type { AppId } from "@/lib/api";
import { type ClaudeModelEnvField } from "./hooks/useModelState";
import type {
  ProviderCategory,
  ClaudeApiFormat,
  CodexApiFormat,
  ClaudeApiKeyField,
  CodexChatReasoning,
  CodexCatalogModel,
  EndpointCandidate,
  OpenCodeModel,
  OpenClawModel,
} from "@/types";
import type { HermesModel } from "@/config/hermesProviderPresets";
import type { TemplateValueConfig } from "@/config/baseProviderPreset";
import type { HermesApiMode } from "@/config/hermesProviderPresets";

export interface AppSpecificFormFieldsProps {
  appId: AppId;
  category?: ProviderCategory;
  providerId?: string;
  isEditMode: boolean;
  isAnyOmoCategory: boolean;

  // Claude specific handlers
  apiKey: string;
  onClaudeApiKeyChange: (val: string) => void;
  baseUrl: string;
  onClaudeBaseUrlChange: (val: string) => void;
  onClaudeModelChange: (field: ClaudeModelEnvField, value: string) => void;

  shouldShowApiKey: (config: string, isEdit: boolean) => boolean;
  shouldShowApiKeyLink: boolean;
  websiteUrl: string;
  isPartner?: boolean;
  partnerPromotionKey?: string;
  shouldShowSpeedTest: boolean;
  speedTestEndpoints: EndpointCandidate[];
  isEndpointModalOpen: boolean;
  onEndpointModalToggle: (val: boolean) => void;
  onCustomEndpointsChange?: (val: string[]) => void;
  autoSelect: boolean;
  onAutoSelectChange: (val: boolean) => void;
  isFullUrl: boolean;
  onFullUrlChange: (val: boolean) => void;
  settingsConfig: string;

  // Claude specific presets / OAuth
  isCopilotPreset: boolean;
  isCodexOauthPreset: boolean;
  usesOAuth: boolean;
  isCopilotAuthenticated: boolean;
  selectedGitHubAccountId: string | null;
  onGitHubAccountSelect: (val: string | null) => void;
  isCodexOauthAuthenticated: boolean;
  selectedCodexAccountId: string | null;
  onCodexAccountSelect: (val: string | null) => void;
  codexFastMode: boolean;
  onCodexFastModeChange: (val: boolean) => void;
  templateValueEntries: Array<[string, TemplateValueConfig]>;
  templateValues: Record<string, TemplateValueConfig>;
  templatePresetName: string;
  onTemplateValueChange: (key: string, val: string) => void;
  shouldShowModelSelector: boolean;
  claudeModel: string;
  defaultHaikuModel: string;
  defaultHaikuModelName: string;
  defaultSonnetModel: string;
  defaultSonnetModelName: string;
  defaultOpusModel: string;
  defaultOpusModelName: string;
  defaultFableModel: string;
  defaultFableModelName: string;
  apiFormat: ClaudeApiFormat;
  onApiFormatChange: (val: ClaudeApiFormat) => void;
  apiKeyField: ClaudeApiKeyField;
  onApiKeyFieldChange: (val: ClaudeApiKeyField) => void;

  // Codex specific
  codexApiKey: string;
  onCodexApiKeyChange: (val: string) => void;
  codexBaseUrl: string;
  onCodexBaseUrlChange: (val: string) => void;
  codexApiFormat: CodexApiFormat;
  onCodexApiFormatChange: (val: CodexApiFormat) => void;
  codexChatReasoning: CodexChatReasoning;
  onCodexChatReasoningChange: (val: CodexChatReasoning) => void;
  codexCatalogModels: CodexCatalogModel[];
  onCodexCatalogModelsChange: (val: CodexCatalogModel[]) => void;

  // Gemini specific
  geminiApiKey: string;
  onGeminiApiKeyChange: (val: string) => void;
  geminiBaseUrl: string;
  onGeminiBaseUrlChange: (val: string) => void;
  geminiModel: string;
  onGeminiModelChange: (val: string) => void;

  // OpenCode specific
  opencodeForm: {
    opencodeNpm: string;
    handleOpencodeNpmChange: (val: string) => void;
    opencodeApiKey: string;
    handleOpencodeApiKeyChange: (val: string) => void;
    opencodeBaseUrl: string;
    handleOpencodeBaseUrlChange: (val: string) => void;
    opencodeModels: Record<string, OpenCodeModel>;
    handleOpencodeModelsChange: (val: Record<string, OpenCodeModel>) => void;
    opencodeExtraOptions: Record<string, string>;
    handleOpencodeExtraOptionsChange: (val: Record<string, string>) => void;
  };

  // OMO specific
  omoModelOptions: Array<{ value: string; label: string }>;
  omoModelVariantsMap: Record<string, string[]>;
  omoPresetMetaMap: Record<string, { options?: Record<string, unknown>; limit?: { context?: number; output?: number } }>;
  omoDraft: {
    omoAgents: Record<string, Record<string, unknown>>;
    setOmoAgents: (val: Record<string, Record<string, unknown>>) => void;
    omoCategories?: Record<string, Record<string, unknown>>;
    setOmoCategories?: (val: Record<string, Record<string, unknown>>) => void;
    omoOtherFieldsStr: string;
    setOmoOtherFieldsStr: (val: string) => void;
  };

  // OpenClaw specific
  openclawForm: {
    openclawBaseUrl: string;
    handleOpenclawBaseUrlChange: (val: string) => void;
    openclawApiKey: string;
    handleOpenclawApiKeyChange: (val: string) => void;
    openclawApi: string;
    handleOpenclawApiChange: (val: string) => void;
    openclawModels: OpenClawModel[];
    handleOpenclawModelsChange: (val: OpenClawModel[]) => void;
    openclawUserAgent: boolean;
    handleOpenclawUserAgentChange: (val: boolean) => void;
  };

  // Hermes specific
  hermesForm: {
    hermesBaseUrl: string;
    handleHermesBaseUrlChange: (val: string) => void;
    hermesApiKey: string;
    handleHermesApiKeyChange: (val: string) => void;
    hermesApiMode: HermesApiMode;
    handleHermesApiModeChange: (val: HermesApiMode) => void;
    hermesModels: HermesModel[];
    handleHermesModelsChange: (val: HermesModel[]) => void;
    hermesRateLimitDelay: number | undefined;
    handleHermesRateLimitDelayChange: (val: number | undefined) => void;
  };
}

/**
 * 表单字段渲染策略映射表
 *
 * 替代 switch(appId) 分发，新增 app 只需在此映射表添加一条。
 * 每个策略函数接收完整的 props，返回对应的表单字段子组件。
 */
type FormFieldsRenderer = (props: AppSpecificFormFieldsProps) => React.ReactNode;

const formFieldsRenderers: Partial<Record<AppId, FormFieldsRenderer>> = {
  claude: (props) => (
    <ClaudeFormFields
      providerId={props.providerId}
      shouldShowApiKey={
        (props.category !== "cloud_provider" ||
          hasApiKeyField(props.settingsConfig, "claude")) &&
        props.shouldShowApiKey(props.settingsConfig, props.isEditMode)
      }
      apiKey={props.apiKey}
      onApiKeyChange={props.onClaudeApiKeyChange}
      category={props.category!}
      shouldShowApiKeyLink={props.shouldShowApiKeyLink}
      websiteUrl={props.websiteUrl}
      isPartner={props.isPartner}
      partnerPromotionKey={props.partnerPromotionKey}
      isCopilotPreset={props.isCopilotPreset}
      isCodexOauthPreset={props.isCodexOauthPreset}
      usesOAuth={props.usesOAuth}
      isCopilotAuthenticated={props.isCopilotAuthenticated}
      selectedGitHubAccountId={props.selectedGitHubAccountId}
      onGitHubAccountSelect={props.onGitHubAccountSelect}
      isCodexOauthAuthenticated={props.isCodexOauthAuthenticated}
      selectedCodexAccountId={props.selectedCodexAccountId}
      onCodexAccountSelect={props.onCodexAccountSelect}
      codexFastMode={props.codexFastMode}
      onCodexFastModeChange={props.onCodexFastModeChange}
      templateValueEntries={props.templateValueEntries}
      templateValues={props.templateValues}
      templatePresetName={props.templatePresetName}
      onTemplateValueChange={props.onTemplateValueChange}
      shouldShowSpeedTest={props.shouldShowSpeedTest}
      baseUrl={props.baseUrl}
      onBaseUrlChange={props.onClaudeBaseUrlChange}
      isEndpointModalOpen={props.isEndpointModalOpen}
      onEndpointModalToggle={props.onEndpointModalToggle}
      onCustomEndpointsChange={props.onCustomEndpointsChange}
      autoSelect={props.autoSelect}
      onAutoSelectChange={props.onAutoSelectChange}
      showEndpointTools
      shouldShowModelSelector={props.category !== "official"}
      claudeModel={props.claudeModel}
      defaultHaikuModel={props.defaultHaikuModel}
      defaultHaikuModelName={props.defaultHaikuModelName}
      defaultSonnetModel={props.defaultSonnetModel}
      defaultSonnetModelName={props.defaultSonnetModelName}
      defaultOpusModel={props.defaultOpusModel}
      defaultOpusModelName={props.defaultOpusModelName}
      defaultFableModel={props.defaultFableModel}
      defaultFableModelName={props.defaultFableModelName}
      onModelChange={props.onClaudeModelChange}
      speedTestEndpoints={props.speedTestEndpoints}
      apiFormat={props.apiFormat}
      onApiFormatChange={props.onApiFormatChange}
      apiKeyField={props.apiKeyField}
      onApiKeyFieldChange={props.onApiKeyFieldChange}
      isFullUrl={props.isFullUrl}
      onFullUrlChange={props.onFullUrlChange}
      subagentModel=""
      localProxyHeadersOverride=""
      onLocalProxyHeadersOverrideChange={() => {}}
      localProxyBodyOverride=""
      onLocalProxyBodyOverrideChange={() => {}}
    />
  ),

  codex: (props) => (
    <CodexFormFields
      providerId={props.providerId}
      codexApiKey={props.codexApiKey}
      onApiKeyChange={props.onCodexApiKeyChange}
      category={props.category!}
      shouldShowApiKeyLink={props.shouldShowApiKeyLink}
      websiteUrl={props.websiteUrl}
      isPartner={props.isPartner}
      partnerPromotionKey={props.partnerPromotionKey}
      shouldShowSpeedTest={props.shouldShowSpeedTest}
      codexBaseUrl={props.codexBaseUrl}
      onBaseUrlChange={props.onCodexBaseUrlChange}
      isFullUrl={props.isFullUrl}
      onFullUrlChange={props.onFullUrlChange}
      isEndpointModalOpen={props.isEndpointModalOpen}
      onEndpointModalToggle={props.onEndpointModalToggle}
      onCustomEndpointsChange={props.onCustomEndpointsChange}
      autoSelect={props.autoSelect}
      onAutoSelectChange={props.onAutoSelectChange}
      apiFormat={props.codexApiFormat}
      onApiFormatChange={props.onCodexApiFormatChange}
      codexChatReasoning={props.codexChatReasoning}
      onCodexChatReasoningChange={props.onCodexChatReasoningChange}
      catalogModels={props.codexCatalogModels}
      onCatalogModelsChange={props.onCodexCatalogModelsChange}
      speedTestEndpoints={props.speedTestEndpoints}
      localProxyHeadersOverride=""
      onLocalProxyHeadersOverrideChange={() => {}}
      localProxyBodyOverride=""
      onLocalProxyBodyOverrideChange={() => {}}
    />
  ),

  gemini: (props) => (
    <GeminiFormFields
      providerId={props.providerId}
      shouldShowApiKey={props.shouldShowApiKey(props.settingsConfig, props.isEditMode)}
      apiKey={props.geminiApiKey}
      onApiKeyChange={props.onGeminiApiKeyChange}
      category={props.category!}
      shouldShowApiKeyLink={props.shouldShowApiKeyLink}
      websiteUrl={props.websiteUrl}
      isPartner={props.isPartner}
      partnerPromotionKey={props.partnerPromotionKey}
      shouldShowSpeedTest={props.shouldShowSpeedTest}
      baseUrl={props.geminiBaseUrl}
      onBaseUrlChange={props.onGeminiBaseUrlChange}
      isEndpointModalOpen={props.isEndpointModalOpen}
      onEndpointModalToggle={props.onEndpointModalToggle}
      onCustomEndpointsChange={props.onCustomEndpointsChange!}
      autoSelect={props.autoSelect}
      onAutoSelectChange={props.onAutoSelectChange}
      shouldShowModelField={true}
      model={props.geminiModel}
      onModelChange={props.onGeminiModelChange}
      speedTestEndpoints={props.speedTestEndpoints}
    />
  ),

  opencode: (props) => {
    if (props.isAnyOmoCategory) {
      return (
        <OmoFormFields
          modelOptions={props.omoModelOptions}
          modelVariantsMap={props.omoModelVariantsMap}
          presetMetaMap={props.omoPresetMetaMap}
          agents={props.omoDraft.omoAgents}
          onAgentsChange={props.omoDraft.setOmoAgents}
          categories={props.category === "omo" ? props.omoDraft.omoCategories : undefined}
          onCategoriesChange={props.category === "omo" ? props.omoDraft.setOmoCategories : undefined}
          otherFieldsStr={props.omoDraft.omoOtherFieldsStr}
          onOtherFieldsStrChange={props.omoDraft.setOmoOtherFieldsStr}
          isSlim={props.category === "omo-slim"}
        />
      );
    }
    return (
      <OpenCodeFormFields
        npm={props.opencodeForm.opencodeNpm}
        onNpmChange={props.opencodeForm.handleOpencodeNpmChange}
        apiKey={props.opencodeForm.opencodeApiKey}
        onApiKeyChange={props.opencodeForm.handleOpencodeApiKeyChange}
        category={props.category!}
        shouldShowApiKeyLink={props.shouldShowApiKeyLink}
        websiteUrl={props.websiteUrl}
        isPartner={props.isPartner}
        partnerPromotionKey={props.partnerPromotionKey}
        baseUrl={props.opencodeForm.opencodeBaseUrl}
        onBaseUrlChange={props.opencodeForm.handleOpencodeBaseUrlChange}
        models={props.opencodeForm.opencodeModels}
        onModelsChange={props.opencodeForm.handleOpencodeModelsChange}
        extraOptions={props.opencodeForm.opencodeExtraOptions}
        onExtraOptionsChange={props.opencodeForm.handleOpencodeExtraOptionsChange}
      />
    );
  },

  openclaw: (props) => (
    <OpenClawFormFields
      baseUrl={props.openclawForm.openclawBaseUrl}
      onBaseUrlChange={props.openclawForm.handleOpenclawBaseUrlChange}
      apiKey={props.openclawForm.openclawApiKey}
      onApiKeyChange={props.openclawForm.handleOpenclawApiKeyChange}
      category={props.category!}
      shouldShowApiKeyLink={props.shouldShowApiKeyLink}
      websiteUrl={props.websiteUrl}
      isPartner={props.isPartner}
      partnerPromotionKey={props.partnerPromotionKey}
      api={props.openclawForm.openclawApi}
      onApiChange={props.openclawForm.handleOpenclawApiChange}
      models={props.openclawForm.openclawModels}
      onModelsChange={props.openclawForm.handleOpenclawModelsChange}
      userAgent={props.openclawForm.openclawUserAgent}
      onUserAgentChange={props.openclawForm.handleOpenclawUserAgentChange}
    />
  ),

  hermes: (props) => (
    <HermesFormFields
      baseUrl={props.hermesForm.hermesBaseUrl}
      onBaseUrlChange={props.hermesForm.handleHermesBaseUrlChange}
      apiKey={props.hermesForm.hermesApiKey}
      onApiKeyChange={props.hermesForm.handleHermesApiKeyChange}
      category={props.category!}
      shouldShowApiKeyLink={props.shouldShowApiKeyLink}
      websiteUrl={props.websiteUrl}
      isPartner={props.isPartner}
      partnerPromotionKey={props.partnerPromotionKey}
      apiMode={props.hermesForm.hermesApiMode}
      onApiModeChange={props.hermesForm.handleHermesApiModeChange}
      models={props.hermesForm.hermesModels}
      onModelsChange={props.hermesForm.handleHermesModelsChange}
      rateLimitDelay={props.hermesForm.hermesRateLimitDelay}
      onRateLimitDelayChange={props.hermesForm.handleHermesRateLimitDelayChange}
    />
  ),
};

export function AppSpecificFormFields(props: AppSpecificFormFieldsProps) {
  const renderer = formFieldsRenderers[props.appId];
  return renderer ? renderer(props) : null;
}

export interface AppSpecificConfigEditorProps {
  appId: AppId;
  category?: ProviderCategory;
  form: UseFormReturn<ProviderFormData>;
  settingsConfigErrorField: React.ReactNode;

  // Codex specific
  codexAuth: string;
  codexConfig: string;
  setCodexAuth: (val: string) => void;
  handleCodexConfigChange: (val: string) => void;
  useCodexCommonConfigFlag: boolean;
  handleCodexCommonConfigToggle: (val: boolean) => void;
  codexCommonConfigSnippet: string;
  handleCodexCommonConfigSnippetChange: (val: string) => boolean;
  clearCodexCommonConfigError: () => void;
  codexCommonConfigError: string | null;
  codexAuthError: string | null;
  codexConfigError: string | null;
  handleCodexExtract: () => void;
  isCodexExtracting: boolean;

  // Gemini specific
  geminiEnv: string;
  geminiConfig: string;
  handleGeminiEnvChange: (val: string) => void;
  handleGeminiConfigChange: (val: string) => void;
  useGeminiCommonConfigFlag: boolean;
  handleGeminiCommonConfigToggle: (val: boolean) => void;
  geminiCommonConfigSnippet: string;
  handleGeminiCommonConfigSnippetChange: (val: string) => boolean;
  clearGeminiCommonConfigError: () => void;
  geminiCommonConfigError: string | null;
  envError: string | null;
  geminiConfigError: string | null;
  handleGeminiExtract: () => void;
  isGeminiExtracting: boolean;

  // OMO specific
  omoDraft: {
    mergedOmoJsonPreview: string;
  };

  // Claude / default common config
  useCommonConfig: boolean;
  handleCommonConfigToggle: (val: boolean) => void;
  commonConfigSnippet: string;
  handleCommonConfigSnippetChange: (val: string) => void;
  commonConfigError: string | null;
  setIsCommonConfigModalOpen: (val: boolean) => void;
  isCommonConfigModalOpen: boolean;
  handleClaudeExtract: () => void;
  isClaudeExtracting: boolean;
}

export function AppSpecificConfigEditor(props: AppSpecificConfigEditorProps) {
  const { appId, category, form, settingsConfigErrorField } = props;
  const { t } = useTranslation();

  if (appId === "codex") {
    return (
      <>
        <CodexConfigEditor
          authValue={props.codexAuth}
          configValue={props.codexConfig}
          onAuthChange={props.setCodexAuth}
          onConfigChange={props.handleCodexConfigChange}
          useCommonConfig={props.useCodexCommonConfigFlag}
          onCommonConfigToggle={props.handleCodexCommonConfigToggle}
          commonConfigSnippet={props.codexCommonConfigSnippet}
          onCommonConfigSnippetChange={props.handleCodexCommonConfigSnippetChange}
          onCommonConfigErrorClear={props.clearCodexCommonConfigError}
          commonConfigError={props.codexCommonConfigError ?? ""}
          authError={props.codexAuthError ?? ""}
          configError={props.codexConfigError ?? ""}
          onExtract={props.handleCodexExtract}
          isExtracting={props.isCodexExtracting}
        />
        {settingsConfigErrorField}
      </>
    );
  }

  if (appId === "gemini") {
    return (
      <>
        <GeminiConfigEditor
          envValue={props.geminiEnv}
          configValue={props.geminiConfig}
          onEnvChange={props.handleGeminiEnvChange}
          onConfigChange={props.handleGeminiConfigChange}
          useCommonConfig={props.useGeminiCommonConfigFlag}
          onCommonConfigToggle={props.handleGeminiCommonConfigToggle}
          commonConfigSnippet={props.geminiCommonConfigSnippet}
          onCommonConfigSnippetChange={props.handleGeminiCommonConfigSnippetChange}
          onCommonConfigErrorClear={props.clearGeminiCommonConfigError}
          commonConfigError={props.geminiCommonConfigError ?? ""}
          envError={props.envError ?? ""}
          configError={props.geminiConfigError ?? ""}
          onExtract={props.handleGeminiExtract}
          isExtracting={props.isGeminiExtracting}
        />
        {settingsConfigErrorField}
      </>
    );
  }

  if (appId === "opencode" && (category === "omo" || category === "omo-slim")) {
    return (
      <div className="space-y-2">
        <Label>{t("provider.configJson")}</Label>
        <JsonEditor
          value={props.omoDraft.mergedOmoJsonPreview}
          onChange={() => {}}
          rows={14}
          showValidation={false}
          language="json"
        />
      </div>
    );
  }

  if (appId === "opencode" && category !== "omo" && category !== "omo-slim") {
    return (
      <>
        <div className="space-y-2">
          <Label htmlFor="settingsConfig">{t("provider.configJson")}</Label>
          <JsonEditor
            value={form.getValues("settingsConfig")}
            onChange={(config) => form.setValue("settingsConfig", config)}
            placeholder={`{
  "npm": "@ai-sdk/openai-compatible",
  "options": {
    "baseURL": "https://your-api-endpoint.com",
    "apiKey": "your-api-key-here"
  },
  "models": {}
}`}
            rows={14}
            showValidation={true}
            language="json"
          />
        </div>
        {settingsConfigErrorField}
      </>
    );
  }

  if (appId === "openclaw" || appId === "hermes") {
    return (
      <>
        <div className="space-y-2">
          <Label htmlFor="settingsConfig">{t("provider.configJson")}</Label>
          <JsonEditor
            value={form.getValues("settingsConfig")}
            onChange={(config) => form.setValue("settingsConfig", config)}
            placeholder={
              appId === "hermes"
                ? `{
  "name": "my-provider",
  "base_url": "https://api.example.com/v1",
  "api_key": ""
}`
                : `{
  "baseUrl": "https://api.example.com/v1",
  "apiKey": "your-api-key-here",
  "api": "openai-completions",
  "models": []
}`
            }
            rows={14}
            showValidation={true}
            language="json"
          />
        </div>
        <FormField
          control={form.control}
          name="settingsConfig"
          render={() => (
            <FormItem className="space-y-0">
              <FormMessage />
            </FormItem>
          )}
        />
      </>
    );
  }

  // Claude / default
  return (
    <>
      <CommonConfigEditor
        value={form.getValues("settingsConfig")}
        onChange={(value) => form.setValue("settingsConfig", value)}
        useCommonConfig={props.useCommonConfig}
        onCommonConfigToggle={props.handleCommonConfigToggle}
        commonConfigSnippet={props.commonConfigSnippet}
        onCommonConfigSnippetChange={props.handleCommonConfigSnippetChange}
        commonConfigError={props.commonConfigError ?? ""}
        onEditClick={() => props.setIsCommonConfigModalOpen(true)}
        isModalOpen={props.isCommonConfigModalOpen}
        onModalClose={() => props.setIsCommonConfigModalOpen(false)}
        onExtract={props.handleClaudeExtract}
        isExtracting={props.isClaudeExtracting}
      />
      {settingsConfigErrorField}
    </>
  );
}
