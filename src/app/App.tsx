import { type FormEvent, useEffect, useMemo, useState } from "react";
import {
  Activity,
  AlertTriangle,
  BookOpenCheck,
  Bug,
  CheckCircle2,
  CircleGauge,
  Code2,
  Copy,
  CreditCard,
  Eye,
  EyeOff,
  Home,
  KeyRound,
  Loader2,
  PackageCheck,
  Plus,
  RefreshCw,
  SearchCheck,
  Trash2,
  ShieldCheck,
  Stethoscope,
  XCircle,
  Wrench,
} from "lucide-react";
import {
  AccountStatus,
  ApiKeySummary,
  AvailableGroup,
  DiagnosticReport,
  InstallerScanResult,
  InvokeResult,
  NetworkScanResult,
  NetworkServerIp,
  PublicSettings,
  ScanCheck,
  ScanFinding,
  ScanProgressEvent,
  QuickScanResult,
  SystemEnvironmentScanResult,
  SystemProfile,
  listenTauriEvent,
  tauriApi,
} from "../lib/tauri";
import { apiKeyStatusText, severityLabel } from "../lib/format";

type PageId =
  | "dashboard"
  | "account"
  | "apiKeys"
  | "environment"
  | "clients"
  | "repairs"
  | "installers"
  | "professional";

type Icon = typeof Home;

const navItems: Array<{ id: PageId; label: string; icon: Icon }> = [
  { id: "dashboard", label: "首页", icon: Home },
  { id: "account", label: "我的账户", icon: CreditCard },
  { id: "apiKeys", label: "API Key", icon: KeyRound },
  { id: "environment", label: "环境检测", icon: Stethoscope },
  { id: "clients", label: "客户端配置", icon: Code2 },
  { id: "repairs", label: "修复中心", icon: Wrench },
  { id: "installers", label: "环境安装", icon: PackageCheck },
  { id: "professional", label: "专业模式", icon: Bug },
];

const appVersion = "v0.1.0";
const pendingText = "等待接口适配/未登录";

type EnvironmentTabId = "fullScan" | "system" | "network" | "installers";

const environmentTabs: Array<{ id: EnvironmentTabId; label: string; icon: Icon }> = [
  { id: "fullScan", label: "全盘扫描", icon: SearchCheck },
  { id: "system", label: "系统环境", icon: Stethoscope },
  { id: "network", label: "网络检测", icon: Activity },
  { id: "installers", label: "软件安装", icon: PackageCheck },
];

type LiveScanStep = {
  id: string;
  title: string;
  state: "pending" | "running" | "pass" | "warn" | "fail" | "skipped";
  message: string;
  durationMs?: number;
  evidence?: unknown;
};

type LiveScanState = {
  active: boolean;
  progress: number;
  completed: number;
  total: number;
  message: string;
  currentStepId?: string;
  startedAt?: string;
  finishedAt?: string;
  steps: LiveScanStep[];
};

type FullScanRow = LiveScanStep & {
  finding?: ScanFinding;
};

type RepairSource = "全盘扫描" | "系统环境" | "网络检测" | "软件安装";

type RepairItem = {
  id: string;
  source: RepairSource;
  title: string;
  severity: ScanFinding["severity"];
  message: string;
  nextStep: string;
  fixSuggestion?: string;
  evidence?: string;
};

const defaultScanSteps: LiveScanStep[] = [
  { id: "dns", title: "DNS 解析", state: "pending", message: "等待检查域名是否能解析。" },
  { id: "tcp", title: "TCP 连接", state: "pending", message: "等待检查端口是否能连通。" },
  { id: "tls", title: "TLS 证书", state: "pending", message: "等待检查 HTTPS 握手和证书链。" },
  { id: "http", title: "HTTP 响应", state: "pending", message: "等待检查服务是否返回响应。" },
  { id: "models", title: "模型接口", state: "pending", message: "等待检查 /v1/models；未填 Key 时会跳过。" },
];

const initialLiveScan: LiveScanState = {
  active: false,
  progress: 0,
  completed: 0,
  total: defaultScanSteps.length,
  message: "尚未开始体检。",
  steps: defaultScanSteps,
};

export function App() {
  const [activePage, setActivePage] = useState<PageId>("dashboard");
  const [account, setAccount] = useState<InvokeResult<AccountStatus> | null>(null);
  const [system, setSystem] = useState<InvokeResult<SystemProfile> | null>(null);
  const [apiKeys, setApiKeys] = useState<InvokeResult<ApiKeySummary[]> | null>(null);
  const [settings, setSettings] = useState<InvokeResult<PublicSettings> | null>(null);
  const [scan, setScan] = useState<InvokeResult<QuickScanResult> | null>(null);
  const [systemScan, setSystemScan] = useState<InvokeResult<SystemEnvironmentScanResult> | null>(null);
  const [installerScan, setInstallerScan] = useState<InvokeResult<InstallerScanResult> | null>(null);
  const [networkScan, setNetworkScan] = useState<InvokeResult<NetworkScanResult> | null>(null);
  const [availableGroups, setAvailableGroups] = useState<InvokeResult<AvailableGroup[]> | null>(null);
  const [liveScan, setLiveScan] = useState<LiveScanState>(initialLiveScan);
  const [scanRequest, setScanRequest] = useState({ baseUrl: "https://www.msutools.cn", apiKey: "" });
  const [isScanning, setIsScanning] = useState(false);
  const [isSystemScanning, setIsSystemScanning] = useState(false);
  const [isInstallerScanning, setIsInstallerScanning] = useState(false);
  const [isNetworkScanning, setIsNetworkScanning] = useState(false);

  async function refreshAccountData() {
    const [nextAccount, nextKeys, nextGroups] = await Promise.all([
      tauriApi.getAccountStatus(),
      tauriApi.listApiKeys(),
      tauriApi.listAvailableGroups(),
    ]);
    setAccount(nextAccount);
    setApiKeys(nextKeys);
    setAvailableGroups(nextGroups);
  }

  async function runSystemEnvironmentScan() {
    setIsSystemScanning(true);
    try {
      const result = await tauriApi.runSystemEnvironmentScan();
      setSystemScan(result);
      if (result.state === "ok") {
        setSystem({ state: "ok", data: result.data.profile });
      }
    } finally {
      setIsSystemScanning(false);
    }
  }

  async function runInstallerScan() {
    setIsInstallerScanning(true);
    try {
      setInstallerScan(await tauriApi.runInstallerScan());
    } finally {
      setIsInstallerScanning(false);
    }
  }

  async function runNetworkScan() {
    setIsNetworkScanning(true);
    try {
      setNetworkScan(await tauriApi.runNetworkScan({
        baseUrl: scanRequest.baseUrl,
        apiKey: scanRequest.apiKey || undefined,
        timeoutMs: 8000,
      }));
    } finally {
      setIsNetworkScanning(false);
    }
  }

  useEffect(() => {
    void tauriApi.getPublicSettings().then(setSettings);
    void refreshAccountData();
    void tauriApi.getSystemProfile().then(setSystem);
    void runSystemEnvironmentScan();
    void runInstallerScan();
    void runNetworkScan();
  }, []);

  useEffect(() => {
    let unsubscribe: (() => void) | null = null;
    let disposed = false;

    void listenTauriEvent<ScanProgressEvent>("quick-scan-progress", (event) => {
      setLiveScan((current) => applyScanProgressEvent(current, event));
    })
      .then((nextUnsubscribe) => {
        if (disposed) {
          nextUnsubscribe();
          return;
        }
        unsubscribe = nextUnsubscribe;
      })
      .catch(() => undefined);

    return () => {
      disposed = true;
      unsubscribe?.();
    };
  }, []);

  const accountSummary = useMemo(() => {
    if (!account || account.state !== "ok") return pendingText;
    if (account.data.loginState !== "logged_in") return "未登录";
    return account.data.displayName ?? account.data.emailMasked ?? "已登录";
  }, [account]);

  async function runQuickScan() {
    setIsScanning(true);
    setScan(null);
    setLiveScan({
      ...initialLiveScan,
      active: true,
      startedAt: new Date().toISOString(),
      message: "正在准备体检。",
      steps: resetScanSteps(),
    });
    try {
      const result = await tauriApi.runQuickScanStreamed({
        baseUrl: scanRequest.baseUrl,
        apiKey: scanRequest.apiKey || undefined,
        timeoutMs: 8000,
      });
      setScan(result);
      if (result.state !== "ok") {
        setLiveScan((current) => ({
          ...current,
          active: false,
          message: result.message,
          finishedAt: new Date().toISOString(),
        }));
      }
    } finally {
      setIsScanning(false);
    }
  }

  async function stopQuickScan() {
    if (!isScanning) return;
    setLiveScan((current) => ({
      ...current,
      message: "正在停止扫描，当前检查完成后会停下。",
    }));
    const result = await tauriApi.stopQuickScan();
    if (result.state !== "ok") {
      setLiveScan((current) => ({
        ...current,
        message: result.message,
      }));
    }
  }

  return (
    <div className="app-shell">
      <aside className="sidebar">
        <div className="brand">
          <div className="brand-mark">
            <ShieldCheck size={20} />
          </div>
          <div>
            <strong>AI Scan</strong>
            <span>msutools 客户端</span>
          </div>
        </div>

        <nav className="nav-list" aria-label="主导航">
          {navItems.map((item) => {
            const IconComponent = item.icon;
            return (
              <button
                className={item.id === activePage ? "nav-item active" : "nav-item"}
                key={item.id}
                type="button"
                onClick={() => setActivePage(item.id)}
              >
                <IconComponent size={18} />
                <span>{item.label}</span>
              </button>
            );
          })}
        </nav>
      </aside>

      <main className="workspace">
        <header className="topbar">
          <div>
            <span className="eyebrow">系统工具</span>
            <h1>{navItems.find((item) => item.id === activePage)?.label}</h1>
          </div>
          <div className="account-chip">
            <CircleGauge size={17} />
            <span>{accountSummary}</span>
          </div>
        </header>

        <div className="workspace-content">
          {activePage === "dashboard" && (
            <Dashboard
              isScanning={isScanning}
              onStartScan={() => {
                setActivePage("environment");
                void runQuickScan();
              }}
            />
          )}
          {activePage === "account" && (
            <AccountPage
              account={account}
              settings={settings}
              onAccountChanged={refreshAccountData}
            />
          )}
          {activePage === "apiKeys" && (
            <ApiKeyPage
              account={account}
              apiKeys={apiKeys}
              availableGroups={availableGroups}
              onKeysChanged={refreshAccountData}
            />
          )}
          {activePage === "environment" && (
            <EnvironmentPage
              system={system}
              systemScan={systemScan}
              installerScan={installerScan}
              networkScan={networkScan}
              scan={scan}
              liveScan={liveScan}
              scanRequest={scanRequest}
              isScanning={isScanning}
              isSystemScanning={isSystemScanning}
              isInstallerScanning={isInstallerScanning}
              isNetworkScanning={isNetworkScanning}
              onScanRequestChange={setScanRequest}
              onRunScan={runQuickScan}
              onStopScan={stopQuickScan}
              onRunSystemScan={runSystemEnvironmentScan}
              onRunInstallerScan={runInstallerScan}
              onRunNetworkScan={runNetworkScan}
            />
          )}
          {activePage === "clients" && (
            <ClientConfigPage
              system={system}
              scanRequest={scanRequest}
            />
          )}
          {activePage === "repairs" && (
            <RepairPage
              scan={scan}
              systemScan={systemScan}
              networkScan={networkScan}
              installerScan={installerScan}
              isScanning={isScanning || isSystemScanning || isNetworkScanning || isInstallerScanning}
              onRunFullScan={runQuickScan}
              onRunSystemScan={runSystemEnvironmentScan}
              onRunNetworkScan={runNetworkScan}
              onRunInstallerScan={runInstallerScan}
            />
          )}
          {activePage === "installers" && (
            <InstallerPage
              installerScan={installerScan}
              isInstallerScanning={isInstallerScanning}
              onRunInstallerScan={runInstallerScan}
            />
          )}
          {activePage === "professional" && (
            <ProfessionalPage
              account={account}
              system={system}
              apiKeys={apiKeys}
              scan={scan}
              systemScan={systemScan}
              networkScan={networkScan}
              installerScan={installerScan}
              scanRequest={scanRequest}
            />
          )}
        </div>
      </main>
    </div>
  );
}

function Dashboard(props: {
  isScanning: boolean;
  onStartScan: () => void;
}) {
  return (
    <section className="home-page">
      <div className="home-banner">
        <div>
          <div className="home-title-row">
            <h2>AI-SCAN</h2>
            <span className="version-badge">{appVersion}</span>
          </div>
          <strong>AI环境一键诊断</strong>
          <p>一键扫描系统环境、网络环境、AI客户端和配置</p>
        </div>
        <button className="primary-action home-scan-button" type="button" disabled={props.isScanning} onClick={props.onStartScan}>
          {props.isScanning ? <Loader2 size={20} className="spin" /> : <SearchCheck size={20} />}
          <span>{props.isScanning ? "正在扫描" : "一键扫描"}</span>
        </button>
      </div>
    </section>
  );
}

function AccountPage({
  account,
  settings,
  onAccountChanged,
}: {
  account: InvokeResult<AccountStatus> | null;
  settings: InvokeResult<PublicSettings> | null;
  onAccountChanged: () => Promise<void>;
}) {
  const connected = account?.state === "ok" && account.data.loginState === "logged_in";
  const paymentUrl =
    settings?.state === "ok" && settings.data.purchaseSubscriptionUrl
      ? settings.data.purchaseSubscriptionUrl
      : settings?.state === "ok" && settings.data.balanceLowNotifyRechargeUrl
        ? settings.data.balanceLowNotifyRechargeUrl
        : "https://www.msutools.cn";
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");
  const [totpCode, setTotpCode] = useState("");
  const [totpTempToken, setTotpTempToken] = useState<string | null>(null);
  const [formMessage, setFormMessage] = useState<string | null>(null);
  const [isBusy, setIsBusy] = useState(false);

  async function submitLogin(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setIsBusy(true);
    setFormMessage(null);
    try {
      const result = await tauriApi.login({ email, password });
      if (result.state !== "ok") {
        setFormMessage(result.message);
        return;
      }

      if (result.data.requires2fa) {
        setTotpTempToken(result.data.tempToken ?? "");
        setFormMessage(`需要输入 6 位两步验证码${result.data.userEmailMasked ? `：${result.data.userEmailMasked}` : ""}`);
        return;
      }

      setPassword("");
      setTotpCode("");
      setTotpTempToken(null);
      setFormMessage("登录成功，账户信息已刷新。");
      await onAccountChanged();
    } finally {
      setIsBusy(false);
    }
  }

  async function submit2fa(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!totpTempToken) {
      setFormMessage("两步验证会话已失效，请重新登录。");
      return;
    }

    setIsBusy(true);
    setFormMessage(null);
    try {
      const result = await tauriApi.login2fa({
        tempToken: totpTempToken,
        totpCode,
      });
      if (result.state !== "ok") {
        setFormMessage(result.message);
        return;
      }
      setPassword("");
      setTotpCode("");
      setTotpTempToken(null);
      setFormMessage("验证成功，账户信息已刷新。");
      await onAccountChanged();
    } finally {
      setIsBusy(false);
    }
  }

  async function logout() {
    setIsBusy(true);
    setFormMessage(null);
    try {
      const result = await tauriApi.logout();
      if (result.state !== "ok") {
        setFormMessage(result.message);
        return;
      }
      setEmail("");
      setPassword("");
      setTotpCode("");
      setTotpTempToken(null);
      setFormMessage("已退出登录，本机保存的会话已清除。");
      await onAccountChanged();
    } finally {
      setIsBusy(false);
    }
  }

  return (
    <section className="two-column">
      <div className="panel">
        <div className="panel-heading">
          <h2>账户概览</h2>
          <span className={connected ? "badge success" : "badge muted"}>{connected ? "已登录" : pendingText}</span>
        </div>
        <dl className="detail-list">
          <div>
            <dt>用户</dt>
            <dd>{connected ? account.data.displayName ?? account.data.emailMasked ?? "已登录用户" : pendingText}</dd>
          </div>
          <div>
            <dt>余额</dt>
            <dd>{connected ? account.data.balanceText ?? "接口未返回余额" : pendingText}</dd>
          </div>
          <div>
            <dt>套餐/额度</dt>
            <dd>{connected ? account.data.quotaText ?? "接口未返回额度" : pendingText}</dd>
          </div>
        </dl>
        {connected && (
          <div className="button-row">
            <button className="secondary-action" type="button" onClick={onAccountChanged} disabled={isBusy}>
              <RefreshCw size={16} />
              <span>刷新账户</span>
            </button>
            <button className="danger-action" type="button" onClick={logout} disabled={isBusy}>
              退出登录
            </button>
          </div>
        )}
      </div>
      <div className="panel">
        <div className="panel-heading">
          <h2>{connected ? "充值入口" : totpTempToken ? "两步验证" : "登录 msutools"}</h2>
          <span className="badge info">{settings?.state === "ok" ? "接口已识别" : "待接入"}</span>
        </div>
        {connected ? (
          <>
            <p className="muted-text">充值入口来自 `/api/v1/settings/public`。如果站点未配置购买订阅 URL，则打开 msutools 官网。</p>
            <div className="button-row">
              <a className="secondary-link" href={paymentUrl} target="_blank" rel="noreferrer">打开充值</a>
            </div>
          </>
        ) : totpTempToken ? (
          <form className="stack-form" onSubmit={submit2fa}>
            <label>
              <span>6 位验证码</span>
              <input
                autoComplete="one-time-code"
                inputMode="numeric"
                maxLength={6}
                value={totpCode}
                onChange={(event) => setTotpCode(event.target.value.replace(/\D/g, ""))}
                placeholder="123456"
              />
            </label>
            <div className="button-row">
              <button className="primary-action compact" type="submit" disabled={isBusy || totpCode.length !== 6}>
                {isBusy ? <Loader2 size={17} className="spin" /> : <ShieldCheck size={17} />}
                <span>完成验证</span>
              </button>
              <button className="secondary-action" type="button" onClick={() => setTotpTempToken(null)} disabled={isBusy}>
                返回登录
              </button>
            </div>
          </form>
        ) : (
          <form className="stack-form" onSubmit={submitLogin}>
            <label>
              <span>邮箱</span>
              <input
                autoComplete="email"
                type="email"
                value={email}
                onChange={(event) => setEmail(event.target.value)}
                placeholder="you@example.com"
                required
              />
            </label>
            <label>
              <span>密码</span>
              <input
                autoComplete="current-password"
                type="password"
                value={password}
                onChange={(event) => setPassword(event.target.value)}
                placeholder="输入 msutools 密码"
                required
              />
            </label>
            {settings?.state === "ok" && settings.data.turnstileEnabled && (
              <Notice kind="warning" text="当前站点启用了 Turnstile。桌面端还未内嵌挑战控件，遇到验证错误时请先在网页端登录或联系管理员开通桌面登录白名单。" />
            )}
            <div className="button-row">
              <button className="primary-action compact" type="submit" disabled={isBusy || !email || !password}>
                {isBusy ? <Loader2 size={17} className="spin" /> : <ShieldCheck size={17} />}
                <span>登录</span>
              </button>
              <a className="secondary-link" href="https://www.msutools.cn/login" target="_blank" rel="noreferrer">网页登录</a>
            </div>
          </form>
        )}
        {formMessage && <p className="form-message">{formMessage}</p>}
      </div>
    </section>
  );
}

function ApiKeyPage({
  account,
  apiKeys,
  availableGroups,
  onKeysChanged,
}: {
  account: InvokeResult<AccountStatus> | null;
  apiKeys: InvokeResult<ApiKeySummary[]> | null;
  availableGroups: InvokeResult<AvailableGroup[]> | null;
  onKeysChanged: () => Promise<void>;
}) {
  const keys = apiKeys?.state === "ok" ? apiKeys.data : [];
  const groups = availableGroups?.state === "ok" ? availableGroups.data : [];
  const loggedIn = account?.state === "ok" && account.data.loginState === "logged_in";
  const [name, setName] = useState("默认 API Key");
  const [groupId, setGroupId] = useState("");
  const [quota, setQuota] = useState("");
  const [expiresInDays, setExpiresInDays] = useState("");
  const [createdKey, setCreatedKey] = useState<string | null>(null);
  const [message, setMessage] = useState<string | null>(null);
  const [busyAction, setBusyAction] = useState<string | null>(null);
  const [deleteCandidate, setDeleteCandidate] = useState<ApiKeySummary | null>(null);

  useEffect(() => {
    if (!groupId && groups.length > 0) {
      setGroupId(String(groups[0].id));
    }
  }, [groupId, groups]);

  async function createKey(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setBusyAction("create");
    setMessage(null);
    setCreatedKey(null);
    try {
      const result = await tauriApi.createApiKey({
        name,
        groupId: groupId ? Number(groupId) : undefined,
        quota: quota ? Number(quota) : undefined,
        expiresInDays: expiresInDays ? Number(expiresInDays) : undefined,
      });
      if (result.state !== "ok") {
        setMessage(result.message);
        return;
      }
      setCreatedKey(result.data.plaintextKeyOnce ?? null);
      setMessage(result.data.plaintextKeyOnce ? "已创建。完整 Key 只显示这一次，请妥善保存。" : "已创建，后端未返回完整 Key。");
      await onKeysChanged();
    } finally {
      setBusyAction(null);
    }
  }

  async function deleteKey(key: ApiKeySummary) {
    setBusyAction(`delete-${key.id}`);
    setMessage(null);
    try {
      const result = await tauriApi.deleteApiKey(key.id);
      if (result.state !== "ok") {
        setMessage(result.message);
        return;
      }
      setMessage("API Key 已删除。");
      setDeleteCandidate(null);
      await onKeysChanged();
    } finally {
      setBusyAction(null);
    }
  }

  return (
    <section className="page-grid api-key-page">
      <div className="two-column">
        <div className="panel">
          <div className="panel-heading">
            <h2>API Key 管理</h2>
            <span className={keys.length > 0 ? "badge success" : "badge muted"}>
              {keys.length > 0 ? `${keys.length} 个 Key` : loggedIn ? "暂无 Key" : "未登录"}
            </span>
          </div>
          {!loggedIn ? (
            <EmptyState title="请先登录" description="登录后会从 `/api/v1/keys` 读取真实 API Key 列表；当前不展示假 Key。" />
          ) : keys.length === 0 ? (
            <EmptyState title="当前账户没有 API Key" description="可以在右侧创建一个新的 API Key。创建后完整 Key 只会显示一次。" />
          ) : (
            <div className="key-table-shell" role="table" aria-label="API Key 列表">
              <div className="table-row table-head" role="row">
                <span>名称 / 额度</span>
                <span>Key</span>
                <span>分组</span>
                <span>状态</span>
                <span>操作</span>
              </div>
              <div className="table-body" role="rowgroup">
                {keys.map((key) => (
                  <div className="table-row" role="row" key={key.id}>
                    <span>
                      <strong>{key.name}</strong>
                      <small>{key.quotaText ?? "未设置额度"} · {key.usageText ?? "未返回用量"}</small>
                    </span>
                    <code>{key.maskedKey}</code>
                    <span>{groupNameForKey(key, groups)}</span>
                    <span>{apiKeyStatusText(key.status)}</span>
                    <button
                      className="icon-button danger"
                      type="button"
                      title="删除 API Key，需要二次确认"
                      onClick={() => setDeleteCandidate(key)}
                      disabled={busyAction === `delete-${key.id}`}
                    >
                      {busyAction === `delete-${key.id}` ? <Loader2 size={16} className="spin" /> : <Trash2 size={16} />}
                    </button>
                  </div>
                ))}
              </div>
            </div>
          )}
        </div>

        <div className="panel">
          <div className="panel-heading">
            <h2>新建 Key</h2>
            <span className="badge info">真实接口</span>
          </div>
          <form className="stack-form" onSubmit={createKey}>
            <label>
              <span>名称</span>
              <input value={name} onChange={(event) => setName(event.target.value)} required maxLength={60} />
            </label>
            <label>
              <span>可用分组</span>
              <select
                value={groupId}
                onChange={(event) => setGroupId(event.target.value)}
                disabled={!loggedIn || groups.length === 0}
                required={groups.length > 0}
              >
                {groups.length === 0 ? (
                  <option value="">未读取到可用分组</option>
                ) : (
                  groups.map((group) => (
                    <option value={group.id} key={group.id}>
                      {group.name} · {group.platform ?? "通用"} · {group.status ?? "unknown"}
                    </option>
                  ))
                )}
              </select>
              <small>Key 必须绑定可用分组，否则网关可能返回 “API Key is not assigned to any group”。</small>
            </label>
            <label>
              <span>额度 USD，可留空</span>
              <input
                inputMode="decimal"
                value={quota}
                onChange={(event) => setQuota(event.target.value.replace(/[^\d.]/g, ""))}
                placeholder="留空表示后端默认"
              />
            </label>
            <label>
              <span>有效天数，可留空</span>
              <input
                inputMode="numeric"
                value={expiresInDays}
                onChange={(event) => setExpiresInDays(event.target.value.replace(/\D/g, ""))}
                placeholder="留空表示不过期"
              />
            </label>
            <button className="primary-action compact" type="submit" disabled={!loggedIn || busyAction === "create" || !name.trim() || groups.length === 0}>
              {busyAction === "create" ? <Loader2 size={17} className="spin" /> : <Plus size={17} />}
              <span>创建 API Key</span>
            </button>
          </form>
          {availableGroups?.state !== "ok" && loggedIn && (
            <Notice kind="warning" text={availableGroups?.message ?? "正在读取可用分组。读取失败时不会创建无分组 Key。"} />
          )}
          {createdKey && (
            <div className="secret-once">
              <div>
                <EyeOff size={16} />
                <strong>完整 Key 只显示一次</strong>
              </div>
              <code>{createdKey}</code>
            </div>
          )}
          {message && <p className="form-message">{message}</p>}
        </div>
      </div>
      {deleteCandidate && (
        <div className="confirm-overlay" role="dialog" aria-modal="true" aria-label="确认删除 API Key">
          <div className="confirm-dialog">
            <div className="panel-heading">
              <h2>确认删除 API Key</h2>
              <span className="badge error">高风险操作</span>
            </div>
            <p>
              删除后客户端将无法继续使用这个 Key：<strong>{deleteCandidate.name}</strong>
              <br />
              <code>{deleteCandidate.maskedKey}</code>
            </p>
            <div className="button-row">
              <button className="danger-action compact" type="button" onClick={() => deleteKey(deleteCandidate)} disabled={busyAction === `delete-${deleteCandidate.id}`}>
                {busyAction === `delete-${deleteCandidate.id}` ? <Loader2 size={17} className="spin" /> : <Trash2 size={17} />}
                <span>确认删除</span>
              </button>
              <button className="secondary-action compact" type="button" onClick={() => setDeleteCandidate(null)} disabled={busyAction === `delete-${deleteCandidate.id}`}>
                取消
              </button>
            </div>
          </div>
        </div>
      )}
    </section>
  );
}

function EnvironmentPage(props: {
  system: InvokeResult<SystemProfile> | null;
  systemScan: InvokeResult<SystemEnvironmentScanResult> | null;
  installerScan: InvokeResult<InstallerScanResult> | null;
  networkScan: InvokeResult<NetworkScanResult> | null;
  scan: InvokeResult<QuickScanResult> | null;
  liveScan: LiveScanState;
  scanRequest: { baseUrl: string; apiKey: string };
  isScanning: boolean;
  isSystemScanning: boolean;
  isInstallerScanning: boolean;
  isNetworkScanning: boolean;
  onScanRequestChange: (value: { baseUrl: string; apiKey: string }) => void;
  onRunScan: () => void;
  onStopScan: () => void;
  onRunSystemScan: () => void;
  onRunInstallerScan: () => void;
  onRunNetworkScan: () => void;
}) {
  const [activeTab, setActiveTab] = useState<EnvironmentTabId>("fullScan");
  const [selectedScanIds, setSelectedScanIds] = useState<Set<string>>(new Set());
  const [repairMessage, setRepairMessage] = useState<string | null>(null);
  const [expandedProbeIds, setExpandedProbeIds] = useState<Set<string>>(new Set());
  const [networkCopyText, setNetworkCopyText] = useState<string | null>(null);
  const networkResult = props.networkScan?.state === "ok" ? props.networkScan.data : null;
  const networkFindings = networkResult?.findings ?? [];
  const systemChecks = props.systemScan?.state === "ok" ? props.systemScan.data.checks : [];
  const systemFindings = props.systemScan?.state === "ok" ? props.systemScan.data.findings : [];
  const installerItems = props.installerScan?.state === "ok" ? props.installerScan.data.items : [];
  const installerFindings = props.installerScan?.state === "ok" ? props.installerScan.data.findings : [];
  const fullScanRows = useMemo(
    () => buildFullScanRows(props.liveScan, props.scan),
    [props.liveScan, props.scan],
  );
  const selectableScanIds = useMemo(() => fullScanRows.map((row) => row.id), [fullScanRows]);
  const actionableRows = fullScanRows.filter(isActionableRow);
  const selectedActionableRows = actionableRows.filter((row) => selectedScanIds.has(row.id));
  const repairTargetRows = selectedActionableRows.length > 0 ? selectedActionableRows : actionableRows;
  const allRowsSelected =
    selectableScanIds.length > 0 && selectableScanIds.every((id) => selectedScanIds.has(id));
  const partialRowsSelected = selectedScanIds.size > 0 && !allRowsSelected;

  useEffect(() => {
    setSelectedScanIds((current) => {
      const allowed = new Set(selectableScanIds);
      const next = new Set([...current].filter((id) => allowed.has(id)));
      return next.size === current.size ? current : next;
    });
  }, [selectableScanIds.join("|")]);

  function toggleAllScanRows() {
    setSelectedScanIds((current) => {
      if (selectableScanIds.length > 0 && selectableScanIds.every((id) => current.has(id))) {
        return new Set();
      }
      return new Set(selectableScanIds);
    });
  }

  function toggleScanRow(id: string) {
    setSelectedScanIds((current) => {
      const next = new Set(current);
      if (next.has(id)) {
        next.delete(id);
      } else {
        next.add(id);
      }
      return next;
    });
  }

  function showRepairPlan(rows: FullScanRow[]) {
    if (rows.length === 0) {
      setRepairMessage("当前没有可修复的未通过项。");
      return;
    }
    setRepairMessage(
      rows
        .map((row, index) => `${index + 1}. ${row.title}：${repairTextForRow(row)}`)
        .join("\n"),
    );
  }

  function toggleProbeDetails(id: string) {
    setExpandedProbeIds((current) => {
      const next = new Set(current);
      if (next.has(id)) {
        next.delete(id);
      } else {
        next.add(id);
      }
      return next;
    });
  }

  async function copyNetworkDiagnostics() {
    const text = networkResult?.diagnosticText;
    if (!text) return;
    try {
      await navigator.clipboard.writeText(text);
      setNetworkCopyText("诊断详情已复制。");
    } catch {
      setNetworkCopyText("当前环境不允许自动复制，请展开详情后手动复制。");
    }
  }

  return (
    <section className="environment-page">
      <div className="env-tabs" role="tablist" aria-label="环境检测分类">
        {environmentTabs.map((tab) => {
          const IconComponent = tab.icon;
          return (
            <button
              className={activeTab === tab.id ? "env-tab active" : "env-tab"}
              type="button"
              role="tab"
              aria-selected={activeTab === tab.id}
              key={tab.id}
              onClick={() => setActiveTab(tab.id)}
            >
              <IconComponent size={16} />
              <span>{tab.label}</span>
            </button>
          );
        })}
      </div>

      {activeTab === "fullScan" && (
        <div className="env-tab-panel full-scan-panel" role="tabpanel">
          <section className="panel full-scan-command">
            <div className="full-scan-command-row">
              <div className="full-scan-title">
                <h2>全盘扫描</h2>
                <span className={props.liveScan.active ? "badge info" : props.scan?.state === "ok" ? statusBadgeClass(props.scan.data.status) : "badge muted"}>
                  {props.liveScan.active ? "正在扫描" : props.scan?.state === "ok" ? scanValue(props.scan) : "等待开始"}
                </span>
              </div>
              <div className="full-scan-actions">
                <button className="primary-action compact" type="button" disabled={props.isScanning} onClick={props.onRunScan}>
                  {props.isScanning ? <Loader2 size={17} className="spin" /> : <SearchCheck size={17} />}
                  <span>{props.isScanning ? "扫描中" : props.scan ? "重新扫描" : "开始扫描"}</span>
                </button>
                <button className="danger-action compact" type="button" disabled={!props.isScanning} onClick={props.onStopScan}>
                  <XCircle size={17} />
                  <span>停止</span>
                </button>
                <button
                  className="secondary-action compact"
                  type="button"
                  disabled={repairTargetRows.length === 0}
                  onClick={() => showRepairPlan(repairTargetRows)}
                >
                  <Wrench size={17} />
                  <span>处理建议</span>
                </button>
              </div>
            </div>

            <div className="full-scan-progress-line">
              <div className="scan-progress-header">
                <div>
                  <strong>{props.liveScan.message}</strong>
                  <span>
                    完成 {props.liveScan.completed}/{props.liveScan.total}
                    {props.liveScan.currentStepId ? ` · 当前：${currentStepTitle(props.liveScan)}` : ""}
                  </span>
                </div>
                <b>{Math.round(props.liveScan.progress)}%</b>
              </div>
              <div className="scan-progress-track" aria-label="扫描进度">
                <div style={{ width: `${props.liveScan.progress}%` }} />
              </div>
            </div>

            <div className="full-scan-config-row">
              <label>
                <span>API 地址</span>
                <input
                  value={props.scanRequest.baseUrl}
                  onChange={(event) =>
                    props.onScanRequestChange({ ...props.scanRequest, baseUrl: event.target.value })
                  }
                  placeholder="https://www.msutools.cn"
                />
              </label>
              <label>
                <span>API Key</span>
                <input
                  type="password"
                  value={props.scanRequest.apiKey}
                  onChange={(event) =>
                    props.onScanRequestChange({ ...props.scanRequest, apiKey: event.target.value })
                  }
                  placeholder="可留空，填入后检查模型接口"
                  autoComplete="off"
                />
              </label>
              <div className="full-scan-counters">
                <span>通过 {fullScanRows.filter((row) => row.state === "pass").length}</span>
                <span>关注 {fullScanRows.filter((row) => row.state === "warn" || row.state === "skipped").length}</span>
                <span>未过 {fullScanRows.filter((row) => row.state === "fail").length}</span>
              </div>
            </div>
          </section>

          <section className="panel super-scan-panel">
            <div className="super-list-heading">
              <div>
                <h2>超级列表</h2>
                <span>每个扫描项逐项过关，支持多选后批量处理。</span>
              </div>
              <span className="badge muted">已选 {selectedScanIds.size}</span>
            </div>

            <div className="super-scan-list" role="table" aria-label="全盘扫描项">
              <div className="super-scan-head" role="row">
                <label className={partialRowsSelected ? "scan-check-cell mixed" : "scan-check-cell"}>
                  <input
                    type="checkbox"
                    checked={allRowsSelected}
                    aria-checked={partialRowsSelected ? "mixed" : allRowsSelected}
                    onChange={toggleAllScanRows}
                  />
                </label>
                <span>状态</span>
                <span>扫描项</span>
                <span>结果</span>
                <span>用时</span>
                <span>操作</span>
              </div>

              <div className="super-scan-body">
                {fullScanRows.map((row) => (
                  <article
                    className={`super-scan-row ${row.state}${selectedScanIds.has(row.id) ? " selected" : ""}`}
                    key={row.id}
                    role="row"
                  >
                    <label className="scan-check-cell">
                      <input
                        type="checkbox"
                        checked={selectedScanIds.has(row.id)}
                        onChange={() => toggleScanRow(row.id)}
                        aria-label={`选择 ${row.title}`}
                      />
                    </label>
                    <div className="scan-row-status">
                      {stepIcon(row.state)}
                      <span>{stepStateText(row.state)}</span>
                    </div>
                    <div className="scan-row-title">
                      <strong>{row.title}</strong>
                      <small>{row.id}</small>
                    </div>
                    <p>{row.message}</p>
                    <small>{typeof row.durationMs === "number" && row.durationMs > 0 ? `${row.durationMs} ms` : "-"}</small>
                    {isActionableRow(row) ? (
                      <button
                        className="secondary-action tiny"
                        type="button"
                        onClick={() => showRepairPlan([row])}
                      >
                        <Wrench size={14} />
                        <span>建议</span>
                      </button>
                    ) : (
                      <span className="scan-row-no-action">-</span>
                    )}
                  </article>
                ))}
              </div>
            </div>

            {repairMessage && (
              <div className="scan-repair-advice">
                <strong>修复建议</strong>
                <pre>{repairMessage}</pre>
              </div>
            )}
          </section>
        </div>
      )}

      {activeTab === "system" && (
        <div className="env-tab-panel system-panel" role="tabpanel">
          <div className="panel">
            <div className="panel-heading">
              <h2>本机环境</h2>
              <span className={props.systemScan?.state === "ok" ? statusBadgeClass(props.systemScan.data.status) : "badge muted"}>
                {systemScanText(props.systemScan)}
              </span>
            </div>
            <dl className="detail-list">
              <div>
                <dt>操作系统</dt>
                <dd>{props.system?.state === "ok" ? `${props.system.data.os}${props.system.data.osVersion ? ` ${props.system.data.osVersion}` : ""}` : pendingText}</dd>
              </div>
              <div>
                <dt>架构</dt>
                <dd>{props.system?.state === "ok" ? props.system.data.architecture : pendingText}</dd>
              </div>
              <div>
                <dt>Shell</dt>
                <dd>{props.system?.state === "ok" ? props.system.data.shell ?? "接口未返回" : pendingText}</dd>
              </div>
            </dl>
            <div className="button-row">
              <button className="primary-action compact" type="button" onClick={props.onRunSystemScan} disabled={props.isSystemScanning}>
                {props.isSystemScanning ? <Loader2 size={17} className="spin" /> : <RefreshCw size={17} />}
                <span>{props.isSystemScanning ? "检测中" : "重新检测"}</span>
              </button>
            </div>
          </div>
          <div className="panel system-check-panel">
            <div className="panel-heading">
              <h2>检测结果</h2>
              <span className="badge muted">{systemChecks.length > 0 ? `${systemChecks.length} 项` : "加载中"}</span>
            </div>
            {systemChecks.length > 0 ? (
              <div className="system-check-list">
                {systemChecks.map((check) => (
                  <article className={`scan-step ${scanCheckState(check)}`} key={check.id}>
                    <div className="scan-step-icon">{stepIcon(scanCheckState(check))}</div>
                    <div className="scan-step-body">
                      <div>
                        <strong>{check.title}</strong>
                        <span>{stepStateText(scanCheckState(check))}</span>
                      </div>
                      <p>{check.message}</p>
                    </div>
                  </article>
                ))}
              </div>
            ) : (
              <EmptyState title="正在读取系统环境" description="检测会读取系统概要、环境变量、PATH 和常用命令版本。" />
            )}
          </div>
          <div className="panel system-finding-panel">
            <div className="panel-heading">
              <h2>需要关注</h2>
              <span className="badge muted">{systemFindings.length > 0 ? `${systemFindings.length} 项` : "无问题"}</span>
            </div>
            {systemFindings.length > 0 ? (
              <div className="finding-list">
                {systemFindings.map((finding) => (
                  <article className={`finding ${finding.severity}`} key={finding.id}>
                    <span>{severityLabel(finding.severity)}</span>
                    <div>
                      <strong>{finding.title}</strong>
                      <p>{finding.message}</p>
                      <small>{finding.nextStep}</small>
                    </div>
                  </article>
                ))}
              </div>
            ) : (
              <EmptyState title="没有系统环境问题" description="当前系统环境检测没有返回需要处理的问题。" />
            )}
          </div>
        </div>
      )}

      {activeTab === "network" && (
        <div className="env-tab-panel network-panel" role="tabpanel">
          <section className="panel network-summary-panel">
            <div className="network-summary-head">
              <div>
                <h2>网络检测</h2>
                <p>检测本地出口、服务器解析、服务器端口和关键 HTTP/API 状态。</p>
              </div>
              <div className="network-actions">
                <button className="primary-action compact" type="button" onClick={props.onRunNetworkScan} disabled={props.isNetworkScanning}>
                  {props.isNetworkScanning ? <Loader2 size={17} className="spin" /> : <RefreshCw size={17} />}
                  <span>{props.isNetworkScanning ? "检测中" : "重新检测"}</span>
                </button>
                <button className="secondary-action compact" type="button" disabled={!networkResult?.diagnosticText} onClick={copyNetworkDiagnostics}>
                  <Copy size={17} />
                  <span>复制详情</span>
                </button>
              </div>
            </div>

            <div className="network-kpi-grid">
              <NetworkKpi
                title="本地出口 IP"
                value={networkResult?.exitIp ? networkResult.exitIp.ip : props.isNetworkScanning ? "检测中" : "未检测"}
                detail={networkResult?.exitIp ? ipLocationText(networkResult.exitIp) : "用于判断用户公网出口和归属地"}
              />
              <NetworkKpi
                title="服务器 IP"
                value={networkResult?.serverIps.length ? `${networkResult.serverIps.length} 个` : props.isNetworkScanning ? "检测中" : "未检测"}
                detail={networkResult?.host ?? props.scanRequest.baseUrl}
              />
              <NetworkKpi
                title="总体状态"
                value={networkResult ? networkStatusText(networkResult.status) : props.isNetworkScanning ? "检测中" : "等待检测"}
                detail={networkFindings.length > 0 ? `${networkFindings.length} 个问题/建议` : "可截图给客服定位问题"}
              />
            </div>

            {props.networkScan?.state !== "ok" && props.networkScan && (
              <Notice kind="warning" text={props.networkScan.message} />
            )}
            {networkCopyText && <p className="form-message compact-message">{networkCopyText}</p>}
          </section>

          <section className="panel network-server-panel">
            <div className="panel-heading">
              <h2>服务器与出口</h2>
              <span className={networkResult ? statusBadgeClass(networkResult.status) : "badge muted"}>
                {networkResult ? networkStatusText(networkResult.status) : "等待检测"}
              </span>
            </div>
            {networkResult ? (
              <div className="network-server-list">
                <article className="network-server-row">
                  <div>
                    <strong>本地出口</strong>
                    <span>{networkResult.exitIp ? ipLocationText(networkResult.exitIp) : "未获取出口 IP"}</span>
                  </div>
                  <small>{networkResult.exitIp?.isp ?? networkResult.exitIp?.org ?? "运营商未知"}</small>
                </article>
                {networkResult.serverIps.map((server) => (
                  <ServerIpRow server={server} key={server.address} />
                ))}
              </div>
            ) : (
              <EmptyState title="尚未检测网络" description="点击重新检测后会读取出口 IP、服务器 IP 和端口连通状态。" />
            )}
          </section>

          <section className="panel network-probe-panel">
            <div className="panel-heading">
              <h2>请求状态</h2>
              <span className="badge muted">{networkResult ? `${networkResult.probes.length} 项` : "等待检测"}</span>
            </div>
            {networkResult ? (
              <div className="network-probe-list">
                {networkResult.probes.map((probe) => (
                  <article className={`network-probe ${probe.status}`} key={probe.id}>
                    <div className="network-probe-main">
                      <div className="scan-step-icon">{stepIcon(scanCheckStateFromStatus(probe.status))}</div>
                      <div>
                        <strong>{probe.title}</strong>
                        <span>{probe.method} {probe.url}</span>
                      </div>
                      <b>{probe.statusCode ? `HTTP ${probe.statusCode}` : "无状态码"}</b>
                      <button className="secondary-action tiny" type="button" onClick={() => toggleProbeDetails(probe.id)}>
                        <Eye size={14} />
                        <span>{expandedProbeIds.has(probe.id) ? "收起" : "详情"}</span>
                      </button>
                    </div>
                    <p>{probe.message}</p>
                    <small>{probe.suggestion}</small>
                    {expandedProbeIds.has(probe.id) && (
                      <div className="network-probe-detail">
                        <dl>
                          <div>
                            <dt>说明</dt>
                            <dd>{probe.detail}</dd>
                          </div>
                          <div>
                            <dt>用时</dt>
                            <dd>{probe.durationMs} ms</dd>
                          </div>
                          <div>
                            <dt>响应头</dt>
                            <dd>{probe.responseHeaders.length > 0 ? probe.responseHeaders.map((header) => `${header.name}: ${header.value}`).join("；") : "无关键响应头"}</dd>
                          </div>
                          <div>
                            <dt>Body</dt>
                            <dd>{probe.bodyPreview ?? probe.error ?? "无响应正文"}</dd>
                          </div>
                        </dl>
                      </div>
                    )}
                  </article>
                ))}
              </div>
            ) : (
              <EmptyState title="等待请求检测" description="会检测首页、/health 和 /v1/models，保留 401/403/404/429/521 等状态详情。" />
            )}
          </section>

          <section className="panel network-finding-panel">
            <div className="panel-heading">
              <h2>错误详情</h2>
              <span className="badge muted">{networkFindings.length > 0 ? `${networkFindings.length} 项` : "无错误"}</span>
            </div>
            {networkFindings.length > 0 ? (
              <div className="finding-list">
                {networkFindings.map((finding) => (
                  <article className={`finding ${finding.severity}`} key={finding.id}>
                    <span>{severityLabel(finding.severity)}</span>
                    <div>
                      <strong>{finding.title}</strong>
                      <p>{finding.message}</p>
                      <small>{finding.nextStep}</small>
                    </div>
                  </article>
                ))}
              </div>
            ) : (
              <EmptyState title="没有网络错误" description="当前网络检测没有返回必须处理的问题。" />
            )}
            {networkResult?.diagnosticText && (
              <div className="network-diagnostic-box">
                <span>客服诊断文本</span>
                <pre>{networkResult.diagnosticText}</pre>
              </div>
            )}
          </section>
        </div>
      )}

      {activeTab === "installers" && (
        <div className="env-tab-panel installers-panel" role="tabpanel">
          <div className="panel">
            <div className="panel-heading">
              <h2>软件安装</h2>
              <span className={props.installerScan?.state === "ok" ? statusBadgeClass(props.installerScan.data.status) : "badge muted"}>
                {installerScanText(props.installerScan)}
              </span>
            </div>
            <div className="button-row installer-actions">
              <button className="primary-action compact" type="button" onClick={props.onRunInstallerScan} disabled={props.isInstallerScanning}>
                {props.isInstallerScanning ? <Loader2 size={17} className="spin" /> : <RefreshCw size={17} />}
                <span>{props.isInstallerScanning ? "检测中" : "重新检测"}</span>
              </button>
            </div>
            {installerItems.length > 0 ? (
              <div className="installer-grid installer-status-grid">
                {installerItems.map((item) => (
                  <article className={`installer-status-item ${item.status}`} key={item.id}>
                    <div>
                      <BookOpenCheck size={18} />
                      <strong>{item.name}</strong>
                      <span className={installerBadgeClass(item.status)}>{installerStatusText(item.status)}</span>
                    </div>
                    <p>{item.version ?? item.detail}</p>
                    <small>{item.status === "installed" || item.status === "unsupported" ? item.detail : item.installHint}</small>
                  </article>
                ))}
              </div>
            ) : (
              <EmptyState title="正在检测软件环境" description="检测会检查 Node.js、Git、Python、Docker、WebView2 和 VC++ Runtime。" />
            )}
          </div>
          <div className="panel installer-finding-panel">
            <div className="panel-heading">
              <h2>安装建议</h2>
              <span className="badge muted">{installerFindings.length > 0 ? `${installerFindings.length} 项` : "无需处理"}</span>
            </div>
            {installerFindings.length > 0 ? (
              <div className="finding-list">
                {installerFindings.map((finding) => (
                  <article className={`finding ${finding.severity}`} key={finding.id}>
                    <span>{severityLabel(finding.severity)}</span>
                    <div>
                      <strong>{finding.title}</strong>
                      <p>{finding.message}</p>
                      <small>{finding.nextStep}</small>
                    </div>
                  </article>
                ))}
              </div>
            ) : (
              <EmptyState title="运行库状态正常" description="基础运行库和常用工具没有返回必须处理的问题。" />
            )}
          </div>
        </div>
      )}
    </section>
  );
}

function ClientConfigPage({
  system,
  scanRequest,
}: {
  system: InvokeResult<SystemProfile> | null;
  scanRequest: { baseUrl: string; apiKey: string };
}) {
  const env = system?.state === "ok" ? system.data.environment ?? [] : [];
  const tools = system?.state === "ok" ? system.data.tools ?? [] : [];
  const detectedConfig = buildClientConfigItems(env, tools, scanRequest);
  const readyCount = detectedConfig.filter((item) => item.status === "ready").length;

  return (
    <section className="client-config-page">
      <div className="panel">
        <div className="panel-heading">
          <h2>客户端配置</h2>
          <span className={readyCount > 0 ? "badge success" : "badge warning"}>
            {readyCount > 0 ? `${readyCount} 项可用` : "需要配置"}
          </span>
        </div>
        <div className="client-config-list">
          {detectedConfig.map((item) => (
            <article className={`client-config-item ${item.status}`} key={item.id}>
              <div className="scan-step-icon">{stepIcon(item.status === "ready" ? "pass" : item.status === "attention" ? "warn" : "skipped")}</div>
              <div>
                <strong>{item.name}</strong>
                <p>{item.message}</p>
                <small>{item.detail}</small>
              </div>
              <span className={item.status === "ready" ? "mini-badge success" : item.status === "attention" ? "mini-badge warning" : "mini-badge muted"}>
                {item.status === "ready" ? "已识别" : item.status === "attention" ? "待配置" : "未检测"}
              </span>
            </article>
          ))}
        </div>
      </div>
      <div className="panel client-config-side">
        <div className="panel-heading">
          <h2>当前 API 目标</h2>
          <span className="badge info">只读优先</span>
        </div>
        <dl className="detail-list">
          <div>
            <dt>Base URL</dt>
            <dd>{scanRequest.baseUrl || "未填写"}</dd>
          </div>
          <div>
            <dt>API Key</dt>
            <dd>{scanRequest.apiKey ? "已填写，界面不会明文展示" : "未填写"}</dd>
          </div>
        </dl>
        <p className="muted-text">本页只读取环境变量和常用命令状态。自动写入客户端配置必须先实现备份、预览和确认流程。</p>
      </div>
    </section>
  );
}

function RepairPage({
  scan,
  systemScan,
  networkScan,
  installerScan,
  isScanning,
  onRunFullScan,
  onRunSystemScan,
  onRunNetworkScan,
  onRunInstallerScan,
}: {
  scan: InvokeResult<QuickScanResult> | null;
  systemScan: InvokeResult<SystemEnvironmentScanResult> | null;
  networkScan: InvokeResult<NetworkScanResult> | null;
  installerScan: InvokeResult<InstallerScanResult> | null;
  isScanning: boolean;
  onRunFullScan: () => void;
  onRunSystemScan: () => void;
  onRunNetworkScan: () => void;
  onRunInstallerScan: () => void;
}) {
  const repairItems = useMemo(
    () => buildRepairItems(scan, systemScan, networkScan, installerScan),
    [scan, systemScan, networkScan, installerScan],
  );
  const [selectedRepairIds, setSelectedRepairIds] = useState<Set<string>>(new Set());
  const [repairPlan, setRepairPlan] = useState<string | null>(null);
  const [copyMessage, setCopyMessage] = useState<string | null>(null);
  const allSelected = repairItems.length > 0 && repairItems.every((item) => selectedRepairIds.has(item.id));
  const selectedItems = repairItems.filter((item) => selectedRepairIds.has(item.id));
  const planItems = selectedItems.length > 0 ? selectedItems : repairItems;

  useEffect(() => {
    setSelectedRepairIds((current) => {
      const allowed = new Set(repairItems.map((item) => item.id));
      const next = new Set([...current].filter((id) => allowed.has(id)));
      return next.size === current.size ? current : next;
    });
  }, [repairItems.map((item) => item.id).join("|")]);

  function toggleRepairItem(id: string) {
    setSelectedRepairIds((current) => {
      const next = new Set(current);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });
  }

  function toggleAllRepairItems() {
    setSelectedRepairIds((current) => {
      if (repairItems.length > 0 && repairItems.every((item) => current.has(item.id))) {
        return new Set();
      }
      return new Set(repairItems.map((item) => item.id));
    });
  }

  function generateRepairPlan(items: RepairItem[]) {
    if (items.length === 0) {
      setRepairPlan("当前没有真实扫描返回的可处理问题。");
      return;
    }
    setRepairPlan(items.map((item, index) => repairPlanLine(item, index)).join("\n"));
  }

  async function copyRepairPlan() {
    if (!repairPlan) return;
    try {
      await navigator.clipboard.writeText(repairPlan);
      setCopyMessage("修复建议已复制。");
    } catch {
      setCopyMessage("当前环境不允许自动复制，请手动选择文本。");
    }
  }

  return (
    <section className="repair-page">
      <div className="panel repair-command-panel">
        <div className="panel-heading">
          <h2>修复中心</h2>
          <span className={repairItems.length > 0 ? "badge warning" : "badge success"}>
            {repairItems.length > 0 ? `${repairItems.length} 项待处理` : "暂无问题"}
          </span>
        </div>
        <div className="repair-actions">
          <button className="primary-action compact" type="button" onClick={() => generateRepairPlan(planItems)} disabled={repairItems.length === 0}>
            <Wrench size={17} />
            <span>生成修复建议</span>
          </button>
          <button className="secondary-action compact" type="button" onClick={onRunFullScan} disabled={isScanning}>
            <SearchCheck size={17} />
            <span>全盘复检</span>
          </button>
          <button className="secondary-action compact" type="button" onClick={onRunSystemScan} disabled={isScanning}>
            <Stethoscope size={17} />
            <span>系统复检</span>
          </button>
          <button className="secondary-action compact" type="button" onClick={onRunNetworkScan} disabled={isScanning}>
            <Activity size={17} />
            <span>网络复检</span>
          </button>
          <button className="secondary-action compact" type="button" onClick={onRunInstallerScan} disabled={isScanning}>
            <PackageCheck size={17} />
            <span>安装复检</span>
          </button>
        </div>
        <p className="muted-text">这里只基于真实扫描结果生成修复建议；自动改系统配置前必须有备份和确认流程。</p>
      </div>

      <div className="panel repair-list-panel">
        <div className="repair-table" role="table" aria-label="修复项列表">
          <div className="repair-row repair-head" role="row">
            <label className="scan-check-cell">
              <input type="checkbox" checked={allSelected} onChange={toggleAllRepairItems} />
            </label>
            <span>来源</span>
            <span>问题</span>
            <span>建议</span>
            <span>操作</span>
          </div>
          <div className="repair-body" role="rowgroup">
            {repairItems.length > 0 ? (
              repairItems.map((item) => (
                <article className={`repair-row ${item.severity}`} role="row" key={item.id}>
                  <label className="scan-check-cell">
                    <input
                      type="checkbox"
                      checked={selectedRepairIds.has(item.id)}
                      onChange={() => toggleRepairItem(item.id)}
                      aria-label={`选择 ${item.title}`}
                    />
                  </label>
                  <span className="repair-source">{item.source}</span>
                  <div>
                    <strong>{item.title}</strong>
                    <p>{item.message}</p>
                  </div>
                  <small>{item.fixSuggestion ?? item.nextStep}</small>
                  <button className="secondary-action tiny" type="button" onClick={() => generateRepairPlan([item])}>
                    <Wrench size={14} />
                    <span>建议</span>
                  </button>
                </article>
              ))
            ) : (
              <EmptyState title="没有待处理问题" description="当前全盘、系统、网络和安装扫描没有返回需要处理的 Finding。" />
            )}
          </div>
        </div>
      </div>

      {repairPlan && (
        <div className="panel repair-plan-panel">
          <div className="panel-heading">
            <h2>修复建议</h2>
            <button className="secondary-action tiny" type="button" onClick={copyRepairPlan}>
              <Copy size={14} />
              <span>复制</span>
            </button>
          </div>
          <pre>{repairPlan}</pre>
          {copyMessage && <p className="form-message compact-message">{copyMessage}</p>}
        </div>
      )}
    </section>
  );
}

function InstallerPage({
  installerScan,
  isInstallerScanning,
  onRunInstallerScan,
}: {
  installerScan: InvokeResult<InstallerScanResult> | null;
  isInstallerScanning: boolean;
  onRunInstallerScan: () => void;
}) {
  const installerItems = installerScan?.state === "ok" ? installerScan.data.items : [];
  const installerFindings = installerScan?.state === "ok" ? installerScan.data.findings : [];
  const missingCount = installerItems.filter((item) => item.status === "missing").length;
  const attentionCount = installerItems.filter((item) => item.status === "needs_attention").length;
  const installedCount = installerItems.filter((item) => item.status === "installed").length;

  return (
    <section className="installer-page">
      <div className="panel installer-summary-card">
        <div className="panel-heading">
          <h2>环境安装</h2>
          <span className={installerScan?.state === "ok" ? statusBadgeClass(installerScan.data.status) : "badge muted"}>
            {installerScanText(installerScan)}
          </span>
        </div>
        <div className="installer-summary-grid">
          <StatusTile title="已安装" value={`${installedCount}`} detail="可直接使用的运行库" />
          <StatusTile title="缺失" value={`${missingCount}`} detail="基础能力可能不可用" />
          <StatusTile title="需关注" value={`${attentionCount}`} detail="高级功能可能受影响" />
        </div>
        <div className="button-row">
          <button className="primary-action compact" type="button" onClick={onRunInstallerScan} disabled={isInstallerScanning}>
            {isInstallerScanning ? <Loader2 size={17} className="spin" /> : <RefreshCw size={17} />}
            <span>{isInstallerScanning ? "检测中" : "重新检测"}</span>
          </button>
        </div>
      </div>
      <div className="panel installer-main-list">
        <div className="panel-heading">
          <h2>运行库列表</h2>
          <span className="badge muted">{installerItems.length > 0 ? `${installerItems.length} 项` : "加载中"}</span>
        </div>
        {installerItems.length > 0 ? (
          <div className="installer-grid installer-status-grid">
            {installerItems.map((item) => (
              <article className={`installer-status-item ${item.status}`} key={item.id}>
                <div>
                  <BookOpenCheck size={18} />
                  <strong>{item.name}</strong>
                  <span className={installerBadgeClass(item.status)}>{installerStatusText(item.status)}</span>
                </div>
                <p>{item.version ?? item.detail}</p>
                <small>{item.status === "installed" || item.status === "unsupported" ? item.detail : item.installHint}</small>
              </article>
            ))}
          </div>
        ) : (
          <EmptyState title="正在读取安装状态" description="检测会检查 Node.js、Git、curl、Python、Docker、WebView2 和 VC++ Runtime。" />
        )}
      </div>
      <div className="panel installer-main-finding">
        <div className="panel-heading">
          <h2>安装建议</h2>
          <span className="badge muted">{installerFindings.length > 0 ? `${installerFindings.length} 项` : "无需处理"}</span>
        </div>
        {installerFindings.length > 0 ? (
          <div className="finding-list">
            {installerFindings.map((finding) => (
              <article className={`finding ${finding.severity}`} key={finding.id}>
                <span>{severityLabel(finding.severity)}</span>
                <div>
                  <strong>{finding.title}</strong>
                  <p>{finding.message}</p>
                  <small>{finding.nextStep}</small>
                </div>
              </article>
            ))}
          </div>
        ) : (
          <EmptyState title="当前没有安装建议" description="基础运行库检测没有返回必须处理的问题。" />
        )}
      </div>
    </section>
  );
}

function ProfessionalPage(props: {
  account: InvokeResult<AccountStatus> | null;
  system: InvokeResult<SystemProfile> | null;
  apiKeys: InvokeResult<ApiKeySummary[]> | null;
  scan: InvokeResult<QuickScanResult> | null;
  systemScan: InvokeResult<SystemEnvironmentScanResult> | null;
  networkScan: InvokeResult<NetworkScanResult> | null;
  installerScan: InvokeResult<InstallerScanResult> | null;
  scanRequest: { baseUrl: string; apiKey: string };
}) {
  const [report, setReport] = useState<InvokeResult<DiagnosticReport> | null>(null);
  const [isExporting, setIsExporting] = useState(false);

  async function exportReport() {
    setIsExporting(true);
    try {
      setReport(await tauriApi.exportDiagnosticReport({
        baseUrl: props.scanRequest.baseUrl,
        apiKey: props.scanRequest.apiKey || undefined,
        timeoutMs: 8000,
      }));
    } finally {
      setIsExporting(false);
    }
  }

  return (
    <section className="page-grid professional-page">
      <div className="panel">
        <div className="panel-heading">
          <h2>专业模式</h2>
          <span className="badge info">结构化证据</span>
        </div>
        <div className="button-row">
          <button className="primary-action compact" type="button" onClick={exportReport} disabled={isExporting}>
            {isExporting ? <Loader2 size={17} className="spin" /> : <BookOpenCheck size={17} />}
            <span>生成脱敏报告</span>
          </button>
        </div>
        <p className="muted-text">报告来自 Rust 命令，默认标记敏感值已脱敏，不包含原始 API Key、token、cookie 或密码。</p>
      </div>
      <div className="panel">
        <div className="log-grid">
          <PreBlock title="scan">{JSON.stringify(props.scan, null, 2)}</PreBlock>
          <PreBlock title="account">{JSON.stringify(props.account, null, 2)}</PreBlock>
          <PreBlock title="system">{JSON.stringify(props.system, null, 2)}</PreBlock>
          <PreBlock title="systemScan">{JSON.stringify(props.systemScan, null, 2)}</PreBlock>
          <PreBlock title="networkScan">{JSON.stringify(props.networkScan, null, 2)}</PreBlock>
          <PreBlock title="installerScan">{JSON.stringify(props.installerScan, null, 2)}</PreBlock>
          <PreBlock title="apiKeys">{JSON.stringify(props.apiKeys, null, 2)}</PreBlock>
          <PreBlock title="diagnosticReport">{JSON.stringify(report, null, 2)}</PreBlock>
        </div>
      </div>
    </section>
  );
}

function StatusTile({ title, value, detail }: { title: string; value: string; detail: string }) {
  return (
    <div className="status-tile">
      <span>{title}</span>
      <strong>{value}</strong>
      <small>{detail}</small>
    </div>
  );
}

function NetworkKpi({ title, value, detail }: { title: string; value: string; detail: string }) {
  return (
    <div className="network-kpi">
      <span>{title}</span>
      <strong>{value}</strong>
      <small>{detail}</small>
    </div>
  );
}

function ServerIpRow({ server }: { server: NetworkServerIp }) {
  const state = scanCheckStateFromStatus(server.status);
  return (
    <article className={`network-server-row ${server.status}`}>
      <div>
        <strong>{server.address}</strong>
        <span>{server.location ? ipLocationText(server.location) : "归属地未获取"}</span>
      </div>
      <small>{server.family} · {stepStateText(state)} · {server.message}</small>
    </article>
  );
}

function buildClientConfigItems(
  env: NonNullable<SystemProfile["environment"]>,
  tools: NonNullable<SystemProfile["tools"]>,
  scanRequest: { baseUrl: string; apiKey: string },
) {
  const envMap = new Map(env.map((item) => [item.name.toUpperCase(), item]));
  const hasBaseUrl = Boolean(
    scanRequest.baseUrl ||
      envMap.get("OPENAI_BASE_URL")?.value ||
      envMap.get("OPENAI_API_BASE")?.value,
  );
  const hasApiKey = Boolean(scanRequest.apiKey || envMap.get("OPENAI_API_KEY")?.value);
  const toolAvailable = (name: string) => tools.some((tool) => tool.name === name && tool.available);

  return [
    {
      id: "openai-env",
      name: "OpenAI 兼容环境变量",
      status: hasBaseUrl && hasApiKey ? "ready" : hasBaseUrl || hasApiKey ? "attention" : "missing",
      message: hasBaseUrl && hasApiKey ? "已识别 Base URL 和 API Key 来源。" : "未同时识别 Base URL 与 API Key。",
      detail: "检查 OPENAI_BASE_URL、OPENAI_API_BASE、OPENAI_API_KEY，以及当前全盘扫描输入。",
    },
    {
      id: "node-clients",
      name: "Cursor / Cline / Continue",
      status: toolAvailable("node") && toolAvailable("npm") ? "ready" : "attention",
      message: toolAvailable("node") && toolAvailable("npm") ? "Node.js 与 npm 可用，常见编辑器插件具备运行基础。" : "Node.js 或 npm 不可用，插件依赖可能安装失败。",
      detail: "这些客户端通常依赖 Node.js 生态，具体配置写入将在备份流程完成后开放。",
    },
    {
      id: "git-cli",
      name: "Codex / Claude Code CLI",
      status: toolAvailable("git") ? "ready" : "attention",
      message: toolAvailable("git") ? "Git 可用，CLI 工具安装和项目操作具备基础条件。" : "Git 不可用，CLI 工具安装和项目操作可能失败。",
      detail: "后续会读取 CLI 配置文件并生成可确认的写入预览。",
    },
    {
      id: "open-webui",
      name: "Open WebUI / Docker",
      status: toolAvailable("docker") ? "ready" : "missing",
      message: toolAvailable("docker") ? "Docker 可用，可以承载本地 Web UI 类工具。" : "Docker 不可用，本机部署 Open WebUI 会受限。",
      detail: "远程 Web UI 不要求本机 Docker；本地部署时才需要。",
    },
  ] as Array<{
    id: string;
    name: string;
    status: "ready" | "attention" | "missing";
    message: string;
    detail: string;
  }>;
}

function buildRepairItems(
  scan: InvokeResult<QuickScanResult> | null,
  systemScan: InvokeResult<SystemEnvironmentScanResult> | null,
  networkScan: InvokeResult<NetworkScanResult> | null,
  installerScan: InvokeResult<InstallerScanResult> | null,
): RepairItem[] {
  return [
    ...findingsToRepairItems("全盘扫描", scan?.state === "ok" ? scan.data.findings : []),
    ...findingsToRepairItems("系统环境", systemScan?.state === "ok" ? systemScan.data.findings : []),
    ...findingsToRepairItems("网络检测", networkScan?.state === "ok" ? networkScan.data.findings : []),
    ...findingsToRepairItems("软件安装", installerScan?.state === "ok" ? installerScan.data.findings : []),
  ];
}

function findingsToRepairItems(source: RepairSource, findings: ScanFinding[]): RepairItem[] {
  return findings.map((finding) => ({
    id: `${source}:${finding.id}`,
    source,
    title: finding.title,
    severity: finding.severity,
    message: finding.message,
    nextStep: finding.nextStep,
    fixSuggestion: finding.fixSuggestion,
    evidence: finding.evidence,
  }));
}

function repairPlanLine(item: RepairItem, index: number): string {
  const suggestion = item.fixSuggestion ?? item.nextStep;
  return `${index + 1}. [${item.source}] ${item.title}\n   问题：${item.message}\n   建议：${suggestion}`;
}

function groupNameForKey(key: ApiKeySummary, groups: AvailableGroup[]): string {
  if (!key.groupId) return "未绑定";
  const group = groups.find((item) => item.id === key.groupId);
  return group ? group.name : `#${key.groupId}`;
}

function EmptyState({ title, description }: { title: string; description: string }) {
  return (
    <section className="empty-state">
      <strong>{title}</strong>
      <p>{description}</p>
    </section>
  );
}

function Notice({ kind, text }: { kind: "warning" | "info"; text: string }) {
  return (
    <div className={`notice ${kind}`}>
      <AlertTriangle size={16} />
      <span>{text}</span>
    </div>
  );
}

function PreBlock({ title, children }: { title: string; children: string }) {
  return (
    <div className="pre-block">
      <span>{title}</span>
      <pre>{children}</pre>
    </div>
  );
}

function resetScanSteps(): LiveScanStep[] {
  return defaultScanSteps.map((step) => ({ ...step }));
}

function applyScanProgressEvent(current: LiveScanState, event: ScanProgressEvent): LiveScanState {
  const next: LiveScanState = {
    ...current,
    active: event.phase !== "finished" && event.phase !== "canceled" && event.phase !== "failed",
    progress: event.progress,
    completed: event.completed,
    total: event.total,
    currentStepId: event.currentStepId,
    message: event.message,
    finishedAt:
      event.phase === "finished" || event.phase === "canceled" || event.phase === "failed"
        ? event.emittedAt
        : current.finishedAt,
    steps: current.steps,
  };

  if (event.phase === "started") {
    return {
      ...next,
      active: true,
      progress: 0,
      completed: 0,
      total: event.total,
      startedAt: event.emittedAt,
      finishedAt: undefined,
      steps: resetScanSteps(),
    };
  }

  if (!event.check) return next;

  return {
    ...next,
    steps: current.steps.map((step) => {
      if (step.id !== event.check?.id) return step;
      if (event.phase === "step_started") {
        return {
          ...step,
          state: "running",
          message: "正在执行真实检查。",
          durationMs: undefined,
          evidence: undefined,
        };
      }

      if (event.phase === "step_finished") {
        return {
          ...step,
          state: scanCheckState(event.check),
          message: event.check.message,
          durationMs: event.check.durationMs,
          evidence: event.check.evidence,
        };
      }

      return step;
    }),
  };
}

function buildFullScanRows(
  liveScan: LiveScanState,
  scan: InvokeResult<QuickScanResult> | null,
): FullScanRow[] {
  const rows: FullScanRow[] = liveScan.steps.map((step) => ({ ...step }));
  if (scan?.state !== "ok") return rows;

  const findingsByCheck = new Map<string, ScanFinding>();
  for (const finding of scan.data.findings) {
    const checkId = scan.data.checks?.find((check) => finding.id.startsWith(`${check.id}_`))?.id;
    if (checkId && !findingsByCheck.has(checkId)) {
      findingsByCheck.set(checkId, finding);
    }
  }

  for (const check of scan.data.checks ?? []) {
    const row = rows.find((item) => item.id === check.id);
    if (row) {
      row.state = scanCheckState(check);
      row.message = check.message;
      row.durationMs = check.durationMs;
      row.evidence = check.evidence;
      row.finding = findingsByCheck.get(check.id);
    } else {
      rows.push({
        id: check.id,
        title: check.title,
        state: scanCheckState(check),
        message: check.message,
        durationMs: check.durationMs,
        evidence: check.evidence,
        finding: findingsByCheck.get(check.id),
      });
    }
  }

  return rows;
}

function isActionableRow(row: FullScanRow): boolean {
  return row.state === "fail" || row.state === "warn" || row.state === "skipped";
}

function repairTextForRow(row: FullScanRow): string {
  return row.finding?.fixSuggestion ?? row.finding?.nextStep ?? fallbackRepairText(row.id);
}

function fallbackRepairText(id: string): string {
  switch (id) {
    case "dns":
      return "请检查域名拼写、DNS、VPN 或系统代理配置。";
    case "tcp":
      return "请检查防火墙、代理、VPN，以及目标服务端口是否开放。";
    case "tls":
      return "请检查系统时间、证书信任、HTTPS 代理拦截或源站证书配置。";
    case "http":
      return "请确认 API 地址是否正确，以及该地址是否用于 OpenAI 兼容接口。";
    case "models":
      return "请确认 API Key 是否正确、账户是否可用，并确认服务暴露 GET /v1/models。";
    case "canceled":
      return "请重新运行全盘扫描以获得完整诊断结果。";
    default:
      return "请打开专业模式查看详细证据后处理。";
  }
}

function scanCheckState(check: ScanCheck): LiveScanStep["state"] {
  if (check.status === "pass") return "pass";
  if (check.status === "warn") return "warn";
  if (check.status === "fail") return "fail";
  return "skipped";
}

function scanCheckStateFromStatus(status: string): LiveScanStep["state"] {
  if (status === "pass") return "pass";
  if (status === "warn") return "warn";
  if (status === "fail") return "fail";
  if (status === "running") return "running";
  return "skipped";
}

function currentStepTitle(liveScan: LiveScanState): string {
  const step = liveScan.steps.find((item) => item.id === liveScan.currentStepId);
  return step?.title ?? liveScan.currentStepId ?? "";
}

function stepStateText(state: LiveScanStep["state"]): string {
  switch (state) {
    case "running":
      return "检查中";
    case "pass":
      return "通过";
    case "warn":
      return "需关注";
    case "fail":
      return "未通过";
    case "skipped":
      return "已跳过";
    default:
      return "等待";
  }
}

function stepIcon(state: LiveScanStep["state"]) {
  if (state === "running") return <Loader2 size={18} className="spin" />;
  if (state === "pass") return <CheckCircle2 size={18} />;
  if (state === "fail") return <XCircle size={18} />;
  if (state === "warn" || state === "skipped") return <AlertTriangle size={18} />;
  return <SearchCheck size={18} />;
}

function accountValue(result: InvokeResult<AccountStatus> | null): string {
  if (!result) return "检查中";
  if (result.state !== "ok") return pendingText;
  return result.data.loginState === "logged_in" ? "已登录" : "未登录";
}

function systemValue(result: InvokeResult<SystemProfile> | null): string {
  if (!result) return "检查中";
  if (result.state !== "ok") return "等待系统命令";
  return `${result.data.os} ${result.data.architecture}`;
}

function scanValue(result: InvokeResult<QuickScanResult> | null): string {
  if (!result) return "未体检";
  if (result.state !== "ok") return "等待扫描命令";
  if (result.data.status === "passed") return "可使用";
  if (result.data.status === "not_ready") return "未就绪";
  if (result.data.status === "needs_attention") return "需关注";
  return "需要处理";
}

function systemScanText(result: InvokeResult<SystemEnvironmentScanResult> | null): string {
  if (!result) return "检测中";
  if (result.state !== "ok") return result.message || "检测失败";
  if (result.data.status === "passed") return "全部通过";
  if (result.data.status === "needs_attention") return "需关注";
  if (result.data.status === "not_ready") return "未就绪";
  return "需要处理";
}

function installerScanText(result: InvokeResult<InstallerScanResult> | null): string {
  if (!result) return "检测中";
  if (result.state !== "ok") return result.message || "检测失败";
  if (result.data.status === "passed") return "全部可用";
  if (result.data.status === "needs_attention") return "需关注";
  if (result.data.status === "not_ready") return "未就绪";
  return "需要安装";
}

function networkStatusText(status: string): string {
  if (status === "passed") return "网络正常";
  if (status === "needs_attention") return "需关注";
  if (status === "not_ready") return "未就绪";
  return "存在错误";
}

function ipLocationText(ip?: {
  ip: string;
  country?: string;
  region?: string;
  city?: string;
  isp?: string;
  org?: string;
  asn?: string;
}): string {
  if (!ip) return "未知";
  const location = [ip.country, ip.region, ip.city].filter(Boolean).join(" / ");
  const owner = ip.isp ?? ip.org ?? ip.asn;
  if (location && owner) return `${ip.ip} · ${location} · ${owner}`;
  if (location) return `${ip.ip} · ${location}`;
  if (owner) return `${ip.ip} · ${owner}`;
  return ip.ip;
}

function installerStatusText(status: string): string {
  if (status === "installed") return "已安装";
  if (status === "missing") return "未安装";
  if (status === "needs_attention") return "需关注";
  if (status === "unsupported") return "不适用";
  return "未知";
}

function installerBadgeClass(status: string): string {
  if (status === "installed") return "mini-badge success";
  if (status === "missing") return "mini-badge error";
  if (status === "unsupported") return "mini-badge muted";
  return "mini-badge warning";
}

function statusBadgeClass(status: string): string {
  if (status === "passed") return "badge success";
  if (status === "failed") return "badge error";
  return "badge warning";
}
