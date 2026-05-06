export type CommandName =
  | "get_system_profile"
  | "run_system_environment_scan"
  | "run_installer_scan"
  | "run_network_scan"
  | "run_quick_scan"
  | "run_quick_scan_streamed"
  | "stop_quick_scan"
  | "get_public_settings"
  | "login"
  | "login_2fa"
  | "logout"
  | "get_account_status"
  | "list_api_keys"
  | "create_api_key"
  | "delete_api_key"
  | "export_diagnostic_report";

export type InvokeState = "ok" | "unavailable" | "not_implemented" | "error";

export type InvokeResult<T> =
  | { state: "ok"; data: T }
  | { state: "unavailable" | "not_implemented" | "error"; message: string };

export type SystemProfile = {
  os: string;
  osVersion?: string;
  architecture: string;
  shell?: string;
  tools?: Array<{ name: string; available: boolean; version?: string; error?: string }>;
  environment?: Array<{ name: string; value: string; redacted: boolean }>;
};

export type AccountStatus = {
  loginState: "logged_in" | "logged_out" | "unknown";
  displayName?: string;
  emailMasked?: string;
  balanceText?: string;
  quotaText?: string;
  message?: string;
};

export type ApiKeySummary = {
  id: string;
  name: string;
  maskedKey: string;
  keyMasked?: string;
  status: ApiKeyStatus;
  createdAt?: string;
  lastUsedAt?: string;
  expiresAt?: string;
  quotaText?: string;
  usageText?: string;
  enabled?: boolean;
};

export type ApiKeyStatus =
  | "active"
  | "inactive"
  | "quota_exhausted"
  | "expired"
  | "disabled"
  | "unknown"
  | string;

export type ScanSeverity = "info" | "warning" | "error" | "low" | "medium" | "high";
export type ScanFinding = {
  id: string;
  title: string;
  severity: ScanSeverity;
  message: string;
  nextStep: string;
  fixSuggestion?: string;
  evidence?: string;
};

export type QuickScanResult = {
  target?: string;
  status: "passed" | "needs_attention" | "failed" | "not_ready";
  findings: ScanFinding[];
  scannedAt: string;
  checks?: Array<{
    id: string;
    title: string;
    status: string;
    severity: string;
    message: string;
    evidence: unknown;
    durationMs: number;
  }>;
};

export type ScanCheck = NonNullable<QuickScanResult["checks"]>[number];

export type SystemEnvironmentScanResult = {
  scannedAt: string;
  status: QuickScanResult["status"];
  checks: ScanCheck[];
  findings: ScanFinding[];
  profile: SystemProfile;
};

export type InstallerScanResult = {
  scannedAt: string;
  status: QuickScanResult["status"];
  items: InstallerItem[];
  findings: ScanFinding[];
};

export type NetworkScanRequest = QuickScanRequest;

export type NetworkScanResult = {
  target?: string;
  host?: string;
  scannedAt: string;
  status: QuickScanResult["status"];
  exitIp?: NetworkIpInfo;
  serverIps: NetworkServerIp[];
  probes: NetworkHttpProbe[];
  checks: ScanCheck[];
  findings: ScanFinding[];
  diagnosticText: string;
};

export type NetworkIpInfo = {
  ip: string;
  country?: string;
  region?: string;
  city?: string;
  isp?: string;
  org?: string;
  asn?: string;
  timezone?: string;
  source: string;
};

export type NetworkServerIp = {
  ip: string;
  address: string;
  port: number;
  family: string;
  location?: NetworkIpInfo;
  status: string;
  message: string;
  durationMs: number;
};

export type NetworkHttpProbe = {
  id: string;
  title: string;
  method: string;
  url: string;
  status: string;
  severity: ScanSeverity;
  statusCode?: number;
  reason?: string;
  message: string;
  detail: string;
  suggestion: string;
  durationMs: number;
  responseHeaders: Array<{ name: string; value: string }>;
  bodyPreview?: string;
  error?: string;
};

export type InstallerItem = {
  id: string;
  name: string;
  category: "runtime" | "developer_tool" | "container" | "system_component";
  status: "installed" | "missing" | "needs_attention" | "unsupported";
  version?: string;
  detail: string;
  required: boolean;
  installHint: string;
};

export type ScanProgressPhase =
  | "started"
  | "step_started"
  | "step_finished"
  | "finished"
  | "canceled"
  | "failed";

export type ScanProgressEvent = {
  runId: string;
  phase: ScanProgressPhase;
  progress: number;
  completed: number;
  total: number;
  currentStepId?: string;
  currentStepTitle?: string;
  message: string;
  check?: ScanCheck;
  emittedAt: string;
};

export type QuickScanRequest = {
  baseUrl?: string;
  apiKey?: string;
  timeoutMs?: number;
};

export type PublicSettings = {
  siteName?: string;
  siteSubtitle?: string;
  apiBaseUrl?: string;
  docUrl?: string;
  contactInfo?: string;
  turnstileEnabled?: boolean;
  turnstileSiteKey?: string;
  paymentEnabled: boolean;
  purchaseSubscriptionEnabled?: boolean;
  purchaseSubscriptionUrl?: string;
  balanceLowNotifyRechargeUrl?: string;
  version?: string;
};

export type LoginRequest = {
  email: string;
  password: string;
  turnstileToken?: string;
};

export type LoginResult = {
  status: "authenticated" | "requires_2fa";
  account: AccountStatus;
  tokenExpiresAt?: string;
  requires2fa: boolean;
  tempToken?: string;
  userEmailMasked?: string;
};

export type Login2faRequest = {
  tempToken: string;
  totpCode: string;
};

export type CreateApiKeyRequest = {
  name: string;
  groupId?: number | null;
  quota?: number;
  expiresInDays?: number;
  rateLimit5h?: number;
  rateLimit1d?: number;
  rateLimit7d?: number;
};

export type CreatedApiKey = {
  key: ApiKeySummary;
  plaintextKeyOnce?: string;
};

export type ApiKeyList = {
  configured: boolean;
  keys: ApiKeySummary[];
  message: string;
};

export type DiagnosticReport = Record<string, unknown>;

const notReadyMessage = "等待接口适配/未登录";

function normalizeError(error: unknown): InvokeResult<never> {
  const message = error instanceof Error ? error.message : String(error);
  const lowered = message.toLowerCase();

  if (
    lowered.includes("unknown command") ||
    lowered.includes("not found") ||
    lowered.includes("not implemented") ||
    lowered.includes("command") && lowered.includes("missing")
  ) {
    return { state: "not_implemented", message: notReadyMessage };
  }

  return { state: "error", message };
}

export async function invokeCommand<T>(
  command: CommandName,
  args?: Record<string, unknown>,
): Promise<InvokeResult<T>> {
  if (typeof window === "undefined" || !("__TAURI_INTERNALS__" in window)) {
    return { state: "unavailable", message: notReadyMessage };
  }

  try {
    const mod = await import("@tauri-apps/api/core");
    const data = await mod.invoke<T>(command, args);
    return { state: "ok", data };
  } catch (error) {
    return normalizeError(error);
  }
}

export async function listenTauriEvent<T>(
  eventName: string,
  handler: (payload: T) => void,
): Promise<() => void> {
  if (typeof window === "undefined" || !("__TAURI_INTERNALS__" in window)) {
    return () => undefined;
  }

  const mod = await import("@tauri-apps/api/event");
  return mod.listen<T>(eventName, (event) => handler(event.payload));
}

export const tauriApi = {
  getSystemProfile: () => invokeCommand<SystemProfile>("get_system_profile"),
  runSystemEnvironmentScan: () => invokeCommand<SystemEnvironmentScanResult>("run_system_environment_scan"),
  runInstallerScan: () => invokeCommand<InstallerScanResult>("run_installer_scan"),
  runNetworkScan: (request?: NetworkScanRequest) =>
    invokeCommand<NetworkScanResult>("run_network_scan", { request: request ?? null }),
  runQuickScan: (request?: QuickScanRequest) => invokeCommand<QuickScanResult>("run_quick_scan", { request: request ?? null }),
  runQuickScanStreamed: (request?: QuickScanRequest) =>
    invokeCommand<QuickScanResult>("run_quick_scan_streamed", { request: request ?? null }),
  stopQuickScan: () => invokeCommand<void>("stop_quick_scan"),
  getPublicSettings: () => invokeCommand<PublicSettings>("get_public_settings"),
  login: (request: LoginRequest) => invokeCommand<LoginResult>("login", { request }),
  login2fa: (request: Login2faRequest) => invokeCommand<LoginResult>("login_2fa", { request }),
  logout: () => invokeCommand<void>("logout"),
  getAccountStatus: () => invokeCommand<AccountStatus>("get_account_status"),
  listApiKeys: async (): Promise<InvokeResult<ApiKeySummary[]>> => {
    const result = await invokeCommand<ApiKeyList>("list_api_keys");
    if (result.state !== "ok") return result;
    return { state: "ok", data: result.data.keys };
  },
  createApiKey: (request: CreateApiKeyRequest) => invokeCommand<CreatedApiKey>("create_api_key", { request }),
  deleteApiKey: (id: string) => invokeCommand<string>("delete_api_key", { request: { id } }),
  exportDiagnosticReport: (quickScan?: QuickScanRequest) =>
    invokeCommand<DiagnosticReport>("export_diagnostic_report", {
      request: {
        includeSystemProfile: true,
        quickScan: quickScan ?? {
          baseUrl: "https://www.msutools.cn",
          timeoutMs: 8000,
        },
      },
    }),
};
