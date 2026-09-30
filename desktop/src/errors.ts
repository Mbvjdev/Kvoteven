import type { L10nKey } from "./i18n";

/**
 * Error-code catalog. The native backend returns errors as a fixed object
 * `{ code: string }` whose `code` is always one of these sanitized strings
 * (see `AppError` in the Rust backend). Anything outside this catalog is
 * treated as `unknown` and rendered with a generic message. Raw process/HTTP
 * errors must never reach the DOM.
 */
export const KNOWN_ERROR_CODES = [
  "demo_mode_rejected",
  "invalid_key",
  "keyring_unavailable",
  "keyring_write_failed",
  "keyring_delete_failed",
  "not_configured",
  "deepseek_request_failed",
  "deepseek_bad_response",
  "codex_unavailable",
  "codex_request_failed",
  "codex_bad_response",
  "hermes_unavailable",
  "hermes_request_failed",
  "hermes_bad_response",
  "timeout",
  "busy",
  "internal",
  "unknown",
] as const;

const CODE_PATTERN = /^[a-z0-9_.-]{1,64}$/i;

function isBridgeError(err: unknown): err is { code: unknown } {
  return typeof err === "object" && err !== null && "code" in (err as object);
}

/**
 * Reduce any thrown value to a safe catalog code. Unknown or malformed codes
 * (including anything that could smuggle HTML or a raw error body) become
 * `unknown`.
 */
export function sanitizeErrorCode(err: unknown): string {
  if (isBridgeError(err)) {
    const code = err.code;
    if (typeof code === "string" && CODE_PATTERN.test(code)) {
      return (KNOWN_ERROR_CODES as readonly string[]).includes(code) ? code : "unknown";
    }
  }
  return "unknown";
}

/** Map a catalog code to its localized message key. */
export function errorMessageKey(code: string): L10nKey {
  switch (code) {
    case "demo_mode_rejected":
      return "errorDemoRejected";
    case "invalid_key":
      return "errorInvalidKey";
    case "keyring_unavailable":
      return "keyringUnavailable";
    case "keyring_write_failed":
      return "errorKeyringWriteFailed";
    case "keyring_delete_failed":
      return "errorKeyringDeleteFailed";
    case "not_configured":
      return "errorNotConfigured";
    case "deepseek_request_failed":
      return "errorDeepseekRequestFailed";
    case "deepseek_bad_response":
      return "errorDeepseekBadResponse";
    case "codex_unavailable":
      return "codexUnavailable";
    case "codex_request_failed":
      return "errorCodexRequestFailed";
    case "codex_bad_response":
      return "errorCodexBadResponse";
    case "hermes_unavailable":
      return "hermesUnavailable";
    case "hermes_request_failed":
      return "errorHermesRequestFailed";
    case "hermes_bad_response":
      return "errorHermesBadResponse";
    case "timeout":
      return "errorTimeout";
    case "busy":
      return "errorBusy";
    case "internal":
      return "errorInternal";
    default:
      return "errorGeneric";
  }
}
