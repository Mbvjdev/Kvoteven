import { describe, it, expect } from "vitest";
import { sanitizeErrorCode, errorMessageKey, KNOWN_ERROR_CODES } from "./errors";

describe("sanitizeErrorCode", () => {
  it("passes through known, well-formed backend codes", () => {
    expect(sanitizeErrorCode({ code: "timeout" })).toBe("timeout");
    expect(sanitizeErrorCode({ code: "keyring_unavailable" })).toBe("keyring_unavailable");
    expect(sanitizeErrorCode({ code: "not_configured" })).toBe("not_configured");
    expect(sanitizeErrorCode({ code: "invalid_key" })).toBe("invalid_key");
    expect(sanitizeErrorCode({ code: "demo_mode_rejected" })).toBe("demo_mode_rejected");
    expect(sanitizeErrorCode({ code: "codex_bad_response" })).toBe("codex_bad_response");
  });

  it("maps unknown, legacy or malformed codes to the generic fallback", () => {
    expect(sanitizeErrorCode({ code: "some_internal_detail" })).toBe("unknown");
    expect(sanitizeErrorCode({ code: "network" })).toBe("unknown"); // legacy code no longer in the catalog
    expect(sanitizeErrorCode({ code: "deepseek_not_configured" })).toBe("unknown"); // legacy name
    expect(sanitizeErrorCode({ code: "<script>alert(1)</script>" })).toBe("unknown");
    expect(sanitizeErrorCode({ code: "" })).toBe("unknown");
  });

  it("handles non-object errors and missing codes", () => {
    expect(sanitizeErrorCode(new Error("boom"))).toBe("unknown");
    expect(sanitizeErrorCode("boom")).toBe("unknown");
    expect(sanitizeErrorCode(undefined)).toBe("unknown");
    expect(sanitizeErrorCode({})).toBe("unknown");
  });

  it("never exposes a raw error as a displayable code", () => {
    const malicious = { code: "timeout", detail: "secret-token-123" };
    expect(sanitizeErrorCode(malicious)).toBe("timeout");
  });
});

describe("errorMessageKey", () => {
  it("maps every catalog code to a localized key", () => {
    for (const code of KNOWN_ERROR_CODES) {
      expect(errorMessageKey(code), `missing message for ${code}`).toBeTruthy();
    }
  });

  it("maps the 17 native backend codes to distinct, non-generic messages", () => {
    const backendCodes = KNOWN_ERROR_CODES.filter((c) => c !== "unknown");
    expect(backendCodes).toHaveLength(17);
    for (const code of backendCodes) {
      const key = errorMessageKey(code);
      expect(key, `code ${code} must not fall back to generic`).not.toBe("errorGeneric");
    }
  });

  it("maps keyring codes to distinct messages", () => {
    expect(errorMessageKey("keyring_unavailable")).toBe("keyringUnavailable");
    expect(errorMessageKey("keyring_write_failed")).toBe("errorKeyringWriteFailed");
    expect(errorMessageKey("keyring_delete_failed")).toBe("errorKeyringDeleteFailed");
  });

  it("falls back to the generic message for unknown codes", () => {
    expect(errorMessageKey("unknown")).toBe("errorGeneric");
  });
});
