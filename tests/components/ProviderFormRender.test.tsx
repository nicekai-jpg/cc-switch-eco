import { render } from "@testing-library/react";
import { describe, it, vi, expect } from "vitest";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { ProviderForm } from "@/components/providers/forms/ProviderForm";
import type { AppId } from "@/lib/api";

const createTestQueryClient = () =>
  new QueryClient({
    defaultOptions: {
      queries: {
        retry: false,
      },
    },
  });

describe("ProviderForm Render test", () => {
  const apps: AppId[] = ["claude", "codex", "gemini", "opencode", "openclaw", "hermes"];

  it("renders ProviderForm for Claude app with Xunfei config without crashing", () => {
    const queryClient = createTestQueryClient();
    const settingsConfig = {
      "agents": { "defaults": { "thinkingDefault": "high" } },
      "defaultMode": "bypassPermissions",
      "effort": "max",
      "enable_thinking": true,
      "env": {
        "ANTHROPIC_AUTH_TOKEN": "bcc5272fed5e604a1530b1a52151ecf1:ZDBlNTFjYmE3MWI4YTFiNTg5OWM2ZmY1",
        "ANTHROPIC_BASE_URL": "https://maas-coding-api.cn-huabei-1.xf-yun.com/anthropic",
        "ANTHROPIC_DEFAULT_HAIKU_MODEL": "xopkimik26",
        "ANTHROPIC_DEFAULT_OPUS_MODEL": "xopglm51",
        "ANTHROPIC_DEFAULT_SONNET_MODEL": "xopglm51",
        "ANTHROPIC_MODEL": "xopglm51",
        "API_TIMEOUT_MS": "600000",
        "DISABLE_AUTOUPDATER": "1"
      },
      "language": "中文",
      "max_tokens": 32768,
      "model": "xopglm51",
      "permissions": {
        "allow": ["Bash(*)", "Read(*)", "Write(*)", "Edit(*)", "Glob(*)", "Grep(*)", "WebFetch(*)", "WebSearch(*)", "Agent(*)", "NotebookEdit(*)", "AskUserQuestion", "EnterPlanMode", "ExitPlanMode", "EnterWorktree", "ExitWorktree", "CronCreate", "CronDelete", "CronList", "ScheduleWakeup", "Skill(*)"],
        "deny": []
      },
      "reasoning_effort": "high",
      "session": { "defaults": { "thinking": { "enabled": true, "level": "high" } } },
      "statusLine": {
        "command": "bash -c 'cols=${COLUMNS:-}; case \"$cols\" in \"\"|*[!0-9]*) cols=$(stty size </dev/tty 2>/dev/null | awk '\''{print $2}'\'');; esac; case \"$cols\" in \"\"|*[!0-9]*) cols=120;; esac; export COLUMNS=$(( cols > 4 ? cols - 4 : 1 )); plugin_dir=$(ls -d \"${CLAUDE_CONFIG_DIR:-$HOME/.claude}\"/plugins/cache/*/claude-hud/*/ 2>/dev/null | awk -F/ '\''{ print $(NF-1) \"\\t\" $(0) }'\'' | grep -E '\''^[0-9]+\\.[0-9]+\\.[0-9]+[[:space:]]'\'' | sort -t. -k1,1n -k2,2n -k3,3n -k4,4n | tail -1 | cut -f2-); exec \"/Users/limingkai/.bun/bin/bun\" --env-file /dev/null \"${plugin_dir}src/index.ts\"'",
        "type": "command"
      },
      "stream": true,
      "temperature": 0.3,
      "theme": "dark",
      "thinking": { "type": "enabled" },
      "thinkingDisplay": "full"
    };

    const initialData = {
      name: "讯飞 copy",
      websiteUrl: "",
      notes: "",
      settingsConfig: settingsConfig,
      category: "custom" as const,
      meta: { "commonConfigEnabled": false, "endpointAutoSelect": true, "apiFormat": "anthropic" },
      icon: "",
      iconColor: "",
    };

    const { container } = render(
      <QueryClientProvider client={queryClient}>
        <ProviderForm
          appId="claude"
          providerId="d6e908fc-747b-4c13-b619-70e44b7cdb49"
          submitLabel="Save"
          onSubmit={vi.fn()}
          onCancel={vi.fn()}
          initialData={initialData}
          showButtons={false}
        />
      </QueryClientProvider>
    );

    expect(container).toBeDefined();
  });

  it("renders ProviderForm for OpenClaw app with Minimax config without crashing", () => {
    const queryClient = createTestQueryClient();
    const settingsConfig = {
      "baseUrl": "https://api.minimaxi.com/anthropic",
      "api": "anthropic-messages",
      "models": [
        {
          "id": "MiniMax-M2.7",
          "name": "MiniMax M2.7",
          "cost": { "input": 0.3, "output": 1.2, "cacheWrite": 0.375, "cacheRead": 0.06 },
          "contextWindow": 204800,
          "reasoning": true,
          "maxTokens": 131072,
          "input": ["text"]
        }
      ],
      "authHeader": true
    };

    const initialData = {
      name: "MiniMax M2.7",
      websiteUrl: "",
      notes: "",
      settingsConfig: settingsConfig,
      category: "custom" as const,
      meta: { "liveConfigManaged": true },
      icon: "",
      iconColor: "",
    };

    const { container } = render(
      <QueryClientProvider client={queryClient}>
        <ProviderForm
          appId="openclaw"
          providerId="minimax"
          submitLabel="Save"
          onSubmit={vi.fn()}
          onCancel={vi.fn()}
          initialData={initialData}
          showButtons={false}
        />
      </QueryClientProvider>
    );

    expect(container).toBeDefined();
  });

  it("renders ProviderForm for OpenCode app with Omo config without crashing", () => {
    const queryClient = createTestQueryClient();
    const settingsConfig = {
      "agents": {
        "sisyphus": {
          "model": "kimi-for-coding/k2p5",
          "fallback_models": [{ "model": "alibaba-coding-plan-cn/kimi-k2.5" }, { "model": "openai/gpt-5.4", "variant": "medium" }]
        }
      },
      "categories": {
        "ultrabrain": { "model": "openai/gpt-5.4", "variant": "xhigh" }
      },
      "otherFields": {
        "$schema": "https://raw.githubusercontent.com/code-yeongyu/oh-my-openagent/dev/assets/oh-my-opencode.schema.json"
      }
    };

    const initialData = {
      name: "OMO Imported",
      websiteUrl: "",
      notes: "",
      settingsConfig: settingsConfig,
      category: "omo" as const,
      meta: {},
      icon: "",
      iconColor: "",
    };

    const { container } = render(
      <QueryClientProvider client={queryClient}>
        <ProviderForm
          appId="opencode"
          providerId="omo-1c81bfb9-de57-4aef-9d3b-c97c471d6174"
          submitLabel="Save"
          onSubmit={vi.fn()}
          onCancel={vi.fn()}
          initialData={initialData}
          showButtons={false}
        />
      </QueryClientProvider>
    );

    expect(container).toBeDefined();
  });

  apps.forEach((appId) => {
    it(`renders ProviderForm for app: ${appId} in edit mode without crashing`, () => {
      const queryClient = createTestQueryClient();
      const initialData = {
        name: "Test " + appId,
        websiteUrl: "https://example.com",
        notes: "Some notes",
        settingsConfig: {},
        category: "custom" as const,
        meta: {},
        icon: "",
        iconColor: "",
      };

      const { container } = render(
        <QueryClientProvider client={queryClient}>
          <ProviderForm
            appId={appId}
            providerId="test-provider-id"
            submitLabel="Save"
            onSubmit={vi.fn()}
            onCancel={vi.fn()}
            initialData={initialData}
            showButtons={false}
          />
        </QueryClientProvider>
      );

      expect(container).toBeDefined();
    });
  });
});
