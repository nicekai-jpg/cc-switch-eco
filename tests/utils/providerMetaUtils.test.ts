import { describe, expect, it } from "vitest";
import type { ProviderMeta } from "@/types";
import { mergeProviderMeta } from "@/utils/providerMetaUtils";

const buildEndpoint = (url: string) => ({
  url,
  addedAt: 1,
});

describe("mergeProviderMeta", () => {
  it("returns undefined when no initial meta and no endpoints", () => {
    expect(mergeProviderMeta(undefined, null)).toBeUndefined();
    expect(mergeProviderMeta(undefined, undefined)).toBeUndefined();
  });

  it("creates meta when endpoints are provided for new provider", () => {
    const result = mergeProviderMeta(undefined, {
      "https://example.com": buildEndpoint("https://example.com"),
    });

    expect(result).toEqual({
      custom_endpoints: {
        "https://example.com": buildEndpoint("https://example.com"),
      },
    });
  });

  it("overrides custom endpoints but preserves other fields", () => {
    const initial: ProviderMeta = {
      usage_script: {
        enabled: true,
        language: "javascript",
        code: "console.log(1);",
      },
      custom_endpoints: {
        "https://old.com": buildEndpoint("https://old.com"),
      },
    };

    const result = mergeProviderMeta(initial, {
      "https://new.com": buildEndpoint("https://new.com"),
    });

    expect(result).toEqual({
      usage_script: initial.usage_script,
      custom_endpoints: {
        "https://new.com": buildEndpoint("https://new.com"),
      },
    });
  });

  it("preserves custom endpoints when customEndpoints is null (no modification)", () => {
    const initial: ProviderMeta = {
      usage_script: {
        enabled: true,
        language: "javascript",
        code: "console.log(1);",
      },
      custom_endpoints: {
        "https://example.com": buildEndpoint("https://example.com"),
      },
    };

    // null 表示"不修改端点"，保留原有端点
    const result = mergeProviderMeta(initial, null);

    expect(result).toEqual(initial);
  });

  it("removes custom endpoints when explicitly cleared with empty object", () => {
    const initial: ProviderMeta = {
      usage_script: {
        enabled: true,
        language: "javascript",
        code: "console.log(1);",
      },
      custom_endpoints: {
        "https://example.com": buildEndpoint("https://example.com"),
      },
    };

    // 空对象 {} 表示"明确清空端点"
    const result = mergeProviderMeta(initial, {});

    expect(result).toEqual({
      usage_script: initial.usage_script,
    });
  });

  it("returns empty object when removing last field via explicit clear", () => {
    const initial: ProviderMeta = {
      custom_endpoints: {
        "https://example.com": buildEndpoint("https://example.com"),
      },
    };

    // 空对象明确清空，只剩空 meta
    const result = mergeProviderMeta(initial, {});

    expect(result).toEqual({});
  });

  it("preserves custom endpoints when customEndpoints is undefined", () => {
    const initial: ProviderMeta = {
      custom_endpoints: {
        "https://example.com": buildEndpoint("https://example.com"),
      },
    };

    // undefined 也表示"不修改端点"
    const result = mergeProviderMeta(initial, undefined);

    expect(result).toEqual(initial);
  });
});
