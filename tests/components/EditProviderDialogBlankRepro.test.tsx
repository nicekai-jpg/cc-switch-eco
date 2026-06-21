/**
 * Integration-style repro for the user's report:
 *   "Custom Claude providers (e.g. 讯飞 copy) open to a blank edit page,
 *    AND the back button is missing."
 *
 * The "no back button" symptom means FullScreenPanel itself did not render,
 * so EditProviderDialog must have returned null OR thrown high enough that
 * the App-level ErrorBoundary swallowed the dialog.
 *
 * These tests instantiate EditProviderDialog with the real ProviderForm and
 * verify both the header (back button) and the form content render.
 */
import { render, screen, waitFor } from "@testing-library/react";
import { describe, it, vi, expect } from "vitest";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { EditProviderDialog } from "@/components/providers/EditProviderDialog";
import type { Provider } from "@/types";

const createTestQueryClient = () =>
  new QueryClient({
    defaultOptions: { queries: { retry: false } },
  });

const xunfeiProvider: Provider = {
  id: "d6e908fc-747b-4c13-b619-70e44b7cdb49",
  name: "讯飞 copy",
  category: "custom",
  websiteUrl: "",
  notes: "",
  meta: { commonConfigEnabled: false, endpointAutoSelect: true, apiFormat: "anthropic" },
  settingsConfig: {
    env: {
      ANTHROPIC_AUTH_TOKEN: "fake-token",
      ANTHROPIC_BASE_URL:
        "https://maas-coding-api.cn-huabei-1.xf-yun.com/anthropic",
      ANTHROPIC_DEFAULT_HAIKU_MODEL: "xopkimik26",
      ANTHROPIC_DEFAULT_OPUS_MODEL: "xopglm51",
      ANTHROPIC_DEFAULT_SONNET_MODEL: "xopglm51",
      ANTHROPIC_MODEL: "xopglm51",
    },
  },
};

const renderDialog = (provider: Provider) => {
  const queryClient = createTestQueryClient();
  return render(
    <QueryClientProvider client={queryClient}>
      <EditProviderDialog
        open
        provider={provider}
        appId="claude"
        onOpenChange={vi.fn()}
        onSubmit={vi.fn()}
      />
    </QueryClientProvider>,
  );
};

describe("EditProviderDialog — custom Claude providers must not produce a blank panel", () => {
  it("renders the back button (FullScreenPanel header) for a custom 讯飞 provider", async () => {
    renderDialog(xunfeiProvider);

    await waitFor(() => {
      const backButton = document.body.querySelector(".lucide-arrow-left");
      expect(backButton).not.toBeNull();
    });
  });

  it("renders editable form content (not blank) for a custom 讯飞 provider", async () => {
    renderDialog(xunfeiProvider);

    await waitFor(() => {
      const baseUrlInput = document.body.querySelector("#baseUrl");
      expect(baseUrlInput).not.toBeNull();
    });
  });

  it("renders the back button even when settingsConfig is degenerate (empty env)", async () => {
    const degenerate: Provider = {
      ...xunfeiProvider,
      id: "degenerate",
      name: "Degenerate Custom",
      settingsConfig: { env: {} },
    };

    renderDialog(degenerate);

    await waitFor(() => {
      const backButton = document.body.querySelector(".lucide-arrow-left");
      expect(backButton).not.toBeNull();
    });
  });

  it("renders the form content for a degenerate custom config (not blank)", async () => {
    const degenerate: Provider = {
      ...xunfeiProvider,
      id: "degenerate",
      name: "Degenerate Custom",
      settingsConfig: { env: {} },
    };

    renderDialog(degenerate);

    await waitFor(() => {
      const baseUrlInput = document.body.querySelector("#baseUrl");
      expect(baseUrlInput).not.toBeNull();
    });
  });

  it("does NOT crash when settingsConfig is undefined/null at the type-narrow boundary", async () => {
    const broken = {
      ...xunfeiProvider,
      id: "broken-1",
      name: "Broken Null Config",
      settingsConfig: null as unknown as Record<string, unknown>,
    };

    renderDialog(broken as Provider);

    await waitFor(() => {
      const backButton = document.body.querySelector(".lucide-arrow-left");
      expect(backButton).not.toBeNull();
    });
  });

  it("does NOT crash when settingsConfig is a string (legacy DB rows)", async () => {
    const stringConfig = {
      ...xunfeiProvider,
      id: "broken-2",
      name: "Broken String Config",
      settingsConfig: '{"env":{"ANTHROPIC_BASE_URL":"https://x"}}' as unknown as Record<
        string,
        unknown
      >,
    };

    renderDialog(stringConfig as Provider);

    await waitFor(() => {
      const backButton = document.body.querySelector(".lucide-arrow-left");
      expect(backButton).not.toBeNull();
    });
  });

  it("does NOT crash when category is an unknown value", async () => {
    const weirdCategory = {
      ...xunfeiProvider,
      id: "broken-3",
      name: "Broken Category",
      category: "bogus-category" as unknown as Provider["category"],
    };

    renderDialog(weirdCategory as Provider);

    await waitFor(() => {
      const backButton = document.body.querySelector(".lucide-arrow-left");
      expect(backButton).not.toBeNull();
    });
  });

  it("renders form fields when provider.category is undefined (legacy DB rows)", async () => {
    const noCategory: Provider = {
      ...xunfeiProvider,
      id: "no-cat",
      name: "Xunfei No Category",
      category: undefined,
    };

    renderDialog(noCategory);

    await waitFor(() => {
      const baseUrlInput = document.body.querySelector("#baseUrl");
      expect(baseUrlInput).not.toBeNull();
    });
    const backButton = document.body.querySelector(".lucide-arrow-left");
    expect(backButton).not.toBeNull();
  });
});
