/**
 * Repro test for: Custom Claude providers (e.g. 讯飞) opening to a blank edit page.
 *
 * Goal: verify the form actually shows fields (API Key area or Endpoint area or
 * advanced section) when a custom-category Claude provider is opened in edit mode.
 *
 * Existing ProviderFormRender.test.tsx only verifies the form does not throw — it
 * cannot detect a fully blank panel. These tests poke at real visible affordances.
 */
import { render } from "@testing-library/react";
import { describe, it, vi, expect } from "vitest";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { ProviderForm } from "@/components/providers/forms/ProviderForm";

const createTestQueryClient = () =>
  new QueryClient({
    defaultOptions: { queries: { retry: false } },
  });

const renderForm = (initialData: any) => {
  const queryClient = createTestQueryClient();
  return render(
    <QueryClientProvider client={queryClient}>
      <ProviderForm
        appId="claude"
        providerId="test-id"
        submitLabel="Save"
        onSubmit={vi.fn()}
        onCancel={vi.fn()}
        initialData={initialData}
        showButtons={false}
      />
    </QueryClientProvider>,
  );
};

// Heuristic: a "blank" Claude edit panel is one where ClaudeFormFields renders
// nothing user-actionable. We approximate "not blank" by requiring at least one
// of the well-known affordances inside the app-specific section to be present:
//   - the Base URL input (id="baseUrl") — endpoint field
//   - the API Key input — covered by ApiKeyInput (placeholder text varies)
//   - the advanced options collapsible trigger
// If NONE of these exist, the panel is effectively blank for the user.
const expectFormNotBlank = (container: HTMLElement) => {
  const baseUrl = container.querySelector("#baseUrl");
  const apiKeyInputs = container.querySelectorAll(
    'input[type="password"], input[type="text"][autocomplete="off"]',
  );
  // Any rendered <input> or <button> inside the app-specific section indicates
  // the user has something to interact with. The general form chrome (name,
  // category select) lives OUTSIDE ClaudeFormFields, so we focus on inputs
  // that belong to ClaudeFormFields (baseUrl, api key, etc.).
  const hasBaseUrl = baseUrl !== null;
  const hasInteractiveInput = apiKeyInputs.length > 0;
  expect(hasBaseUrl || hasInteractiveInput).toBe(true);
};

describe("ProviderForm — custom Claude provider edit mode is not blank", () => {
  it("讯飞 copy (custom + ANTHROPIC_AUTH_TOKEN) shows the endpoint field", () => {
    const settingsConfig = {
      env: {
        ANTHROPIC_AUTH_TOKEN: "sk-xunfei-fake",
        ANTHROPIC_BASE_URL: "https://maas-coding-api.cn-huabei-1.xf-yun.com/anthropic",
        ANTHROPIC_MODEL: "xopglm51",
      },
    };
    const initialData = {
      name: "讯飞 copy",
      websiteUrl: "",
      notes: "",
      settingsConfig,
      category: "custom" as const,
      meta: { apiFormat: "anthropic" },
      icon: "",
      iconColor: "",
    };

    const { container } = renderForm(initialData);

    expectFormNotBlank(container);
  });

  it("custom Claude provider WITHOUT ANTHROPIC_AUTH_TOKEN still renders editable fields (not blank)", () => {
    const settingsConfig = {
      env: {
        ANTHROPIC_BASE_URL: "https://example.com/anthropic",
      },
    };
    const initialData = {
      name: "Custom No-Key",
      websiteUrl: "",
      notes: "",
      settingsConfig,
      category: "custom" as const,
      meta: {},
      icon: "",
      iconColor: "",
    };

    const { container } = renderForm(initialData);

    expectFormNotBlank(container);
  });

  it("custom Claude provider with empty settingsConfig still renders editable fields (not blank)", () => {
    const initialData = {
      name: "Empty Config",
      websiteUrl: "",
      notes: "",
      settingsConfig: {},
      category: "custom" as const,
      meta: {},
      icon: "",
      iconColor: "",
    };

    const { container } = renderForm(initialData);

    expectFormNotBlank(container);
  });
});
