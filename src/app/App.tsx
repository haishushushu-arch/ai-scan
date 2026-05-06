import { type FormEvent, useEffect, useMemo, useState } from "react";
import {
  Activity,
  AlertTriangle,
  BookOpenCheck,
  Bug,
  CheckCircle2,
  CircleGauge,
  Code2,
  CreditCard,
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
  DiagnosticReport,
  InvokeResult,
  PublicSettings,
  ScanCheck,
  ScanProgressEvent,
  QuickScanResult,
  SystemProfile,
  listenTauriEvent,
  tauriApi,
} from "../lib/tauri";
import { apiKeyStatusText, formatOptionalDate, severityLabel, statusText } from "../lib/format";

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
  { id: "dashboard", label: "首页体检", icon: Home },
  { id: "account", label: "我的账户", icon: CreditCard },
  { id: "apiKeys", label: "API Key", icon: KeyRound },
  { id: "environment", label: "环境体检", icon: Stethoscope },
  { id: "clients", label: "客户端配置", icon: Code2 },
  { id: "repairs", label: "修复中心", icon: Wrench },
  { id: "installers", label: "环境安装", icon: PackageCheck },
  { id: "professional", label: "专业模式", icon: Bug },
];

const pendingText = "等待接口适配/未登录";

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
  const [liveScan, setLiveScan] = useState<LiveScanState>(initialLiveScan);
  const [scanRequest, setScanRequest] = useState({ baseUrl: "https://www.msutools.cn", apiKey: "" });
  const [isScanning, setIsScanning] = useState(false);

  async function refreshAccountData() {
    const [nextAccount, nextKeys] = await Promise.all([
      tauriApi.getAccountStatus(),
      tauriApi.listApiKeys(),
    ]);
    setAccount(nextAccount);
    setApiKeys(nextKeys);
  }

  useEffect(() => {
    void tauriApi.getPublicSettings().then(setSettings);
    void refreshAccountData();
    void tauriApi.getSystemProfile().then(setSystem);
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

        {activePage === "dashboard" && (
          <Dashboard
            account={account}
            system={system}
            scan={scan}
            liveScan={liveScan}
            scanRequest={scanRequest}
            isScanning={isScanning}
            onScanRequestChange={setScanRequest}
            onRunScan={runQuickScan}
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
            onKeysChanged={refreshAccountData}
          />
        )}
        {activePage === "environment" && <EnvironmentPage system={system} />}
        {activePage === "clients" && <ClientConfigPage />}
        {activePage === "repairs" && <RepairPage />}
        {activePage === "installers" && <InstallerPage />}
        {activePage === "professional" && (
          <ProfessionalPage
            account={account}
            system={system}
            apiKeys={apiKeys}
            scan={scan}
            scanRequest={scanRequest}
          />
        )}
      </main>
    </div>
  );
}

function Dashboard(props: {
  account: InvokeResult<AccountStatus> | null;
  system: InvokeResult<SystemProfile> | null;
  scan: InvokeResult<QuickScanResult> | null;
  liveScan: LiveScanState;
  scanRequest: { baseUrl: string; apiKey: string };
  isScanning: boolean;
  onScanRequestChange: (value: { baseUrl: string; apiKey: string }) => void;
  onRunScan: () => void;
}) {
  return (
    <section className="page-grid">
      <div className="primary-panel">
        <div>
          <span className="eyebrow">一键体检</span>
          <h2>检查账号、API 和本机环境是否可用</h2>
          <p>扫描会按 DNS、TCP、TLS、HTTP 和模型接口逐项执行，进度只来自真实检查事件。</p>
        </div>
        <button className="primary-action" type="button" disabled={props.isScanning} onClick={props.onRunScan}>
          {props.isScanning ? <Loader2 size={20} className="spin" /> : <Activity size={20} />}
          <span>{props.isScanning ? "体检中" : "开始体检"}</span>
        </button>
      </div>

      <div className="panel">
        <div className="panel-heading">
          <h2>体检输入</h2>
          <span className="badge info">真实扫描参数</span>
        </div>
        <div className="scan-input-grid">
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
            <span>API Key，可留空</span>
            <input
              type="password"
              value={props.scanRequest.apiKey}
              onChange={(event) =>
                props.onScanRequestChange({ ...props.scanRequest, apiKey: event.target.value })
              }
              placeholder="sk-..."
              autoComplete="off"
            />
          </label>
        </div>
      </div>

      <div className="status-grid">
        <StatusTile title="当前账号状态" value={accountValue(props.account)} detail={invokeDetail(props.account)} />
        <StatusTile title="系统信息" value={systemValue(props.system)} detail={invokeDetail(props.system)} />
        <StatusTile title="API 服务状态" value={scanValue(props.scan)} detail={invokeDetail(props.scan)} />
      </div>

      <LiveScanPanel liveScan={props.liveScan} scan={props.scan} />
      <ResultPanel scan={props.scan} />
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
  onKeysChanged,
}: {
  account: InvokeResult<AccountStatus> | null;
  apiKeys: InvokeResult<ApiKeySummary[]> | null;
  onKeysChanged: () => Promise<void>;
}) {
  const keys = apiKeys?.state === "ok" ? apiKeys.data : [];
  const loggedIn = account?.state === "ok" && account.data.loginState === "logged_in";
  const [name, setName] = useState("默认 API Key");
  const [quota, setQuota] = useState("");
  const [expiresInDays, setExpiresInDays] = useState("");
  const [createdKey, setCreatedKey] = useState<string | null>(null);
  const [message, setMessage] = useState<string | null>(null);
  const [busyAction, setBusyAction] = useState<string | null>(null);

  async function createKey(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setBusyAction("create");
    setMessage(null);
    setCreatedKey(null);
    try {
      const result = await tauriApi.createApiKey({
        name,
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

  async function deleteKey(id: string) {
    setBusyAction(id);
    setMessage(null);
    try {
      const result = await tauriApi.deleteApiKey(id);
      if (result.state !== "ok") {
        setMessage(result.message);
        return;
      }
      setMessage("API Key 已删除。");
      await onKeysChanged();
    } finally {
      setBusyAction(null);
    }
  }

  return (
    <section className="page-grid">
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
            <div className="table key-table">
              {keys.map((key) => (
                <div className="table-row" key={key.id}>
                  <span>
                    <strong>{key.name}</strong>
                    <small>{key.quotaText ?? "未设置额度"} · {key.usageText ?? "未返回用量"}</small>
                  </span>
                  <code>{key.maskedKey}</code>
                  <span>{apiKeyStatusText(key.status)}</span>
                  <span>{formatOptionalDate(key.lastUsedAt)}</span>
                  <button
                    className="icon-button danger"
                    type="button"
                    title="删除 API Key"
                    onClick={() => deleteKey(key.id)}
                    disabled={busyAction === key.id}
                  >
                    {busyAction === key.id ? <Loader2 size={16} className="spin" /> : <Trash2 size={16} />}
                  </button>
                </div>
              ))}
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
            <button className="primary-action compact" type="submit" disabled={!loggedIn || busyAction === "create" || !name.trim()}>
              {busyAction === "create" ? <Loader2 size={17} className="spin" /> : <Plus size={17} />}
              <span>创建 API Key</span>
            </button>
          </form>
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
    </section>
  );
}

function EnvironmentPage({ system }: { system: InvokeResult<SystemProfile> | null }) {
  return (
    <section className="page-grid">
      <div className="panel">
        <div className="panel-heading">
          <h2>本机环境</h2>
          <span className="badge muted">{invokeDetail(system)}</span>
        </div>
        <dl className="detail-list">
          <div>
            <dt>操作系统</dt>
            <dd>{system?.state === "ok" ? system.data.os : pendingText}</dd>
          </div>
          <div>
            <dt>架构</dt>
            <dd>{system?.state === "ok" ? system.data.architecture : pendingText}</dd>
          </div>
          <div>
            <dt>Shell</dt>
            <dd>{system?.state === "ok" ? system.data.shell ?? "接口未返回" : pendingText}</dd>
          </div>
        </dl>
      </div>
      <ChecklistPanel
        title="待接入检测项"
        items={["DNS/TCP/TLS/HTTP", "系统代理", "环境变量", "Node/Python/Git/curl/Docker", "系统时间和证书"]}
      />
    </section>
  );
}

function ClientConfigPage() {
  return (
    <section className="page-grid">
      <ChecklistPanel
        title="客户端配置"
        items={["Cursor", "VS Code", "Cline", "Continue", "Cherry Studio", "Open WebUI", "Codex/Claude Code CLI"]}
      />
      <div className="panel">
        <div className="panel-heading">
          <h2>写入策略</h2>
          <span className="badge info">只读优先</span>
        </div>
        <p className="muted-text">发现配置文件后默认只读展示。任何写入都需要 Rust 侧生成预览、备份路径和确认流程。</p>
      </div>
    </section>
  );
}

function RepairPage() {
  return (
    <section className="page-grid">
      <ChecklistPanel title="修复中心" items={["可自动修复", "需要确认", "只能手动处理", "修复后复检"]} />
      <div className="panel">
        <div className="panel-heading">
          <h2>当前状态</h2>
          <span className="badge muted">等待扫描结果</span>
        </div>
        <p className="muted-text">修复动作必须来自真实 Finding 和 Repair Plan，不能边扫边改。</p>
      </div>
    </section>
  );
}

function InstallerPage() {
  return (
    <section className="panel">
      <div className="panel-heading">
        <h2>环境安装</h2>
        <span className="badge info">计划阶段</span>
      </div>
      <div className="installer-grid">
        {["Node.js LTS", "Git", "Python", "Docker Desktop", "WebView2", "VC++ Runtime", "证书/代理工具"].map((item) => (
          <div className="installer-item" key={item}>
            <BookOpenCheck size={18} />
            <span>{item}</span>
            <small>等待检测接口</small>
          </div>
        ))}
      </div>
    </section>
  );
}

function ProfessionalPage(props: {
  account: InvokeResult<AccountStatus> | null;
  system: InvokeResult<SystemProfile> | null;
  apiKeys: InvokeResult<ApiKeySummary[]> | null;
  scan: InvokeResult<QuickScanResult> | null;
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
    <section className="page-grid">
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

function LiveScanPanel({
  liveScan,
  scan,
}: {
  liveScan: LiveScanState;
  scan: InvokeResult<QuickScanResult> | null;
}) {
  const failed = liveScan.steps.filter((step) => step.state === "fail").length;
  const warned = liveScan.steps.filter((step) => step.state === "warn" || step.state === "skipped").length;
  const passed = liveScan.steps.filter((step) => step.state === "pass").length;
  const progressLabel = `${liveScan.completed}/${liveScan.total}`;

  return (
    <section className="panel scan-live-panel">
      <div className="panel-heading">
        <h2>实时体检进度</h2>
        <span className={liveScan.active ? "badge info" : "badge muted"}>
          {liveScan.active ? "正在体检" : scan?.state === "ok" ? "体检完成" : "等待开始"}
        </span>
      </div>

      <div className="scan-progress-header">
        <div>
          <strong>{liveScan.message}</strong>
          <span>
            已完成 {progressLabel}
            {liveScan.currentStepId ? ` · 当前：${currentStepTitle(liveScan)}` : ""}
          </span>
        </div>
        <b>{Math.round(liveScan.progress)}%</b>
      </div>
      <div className="scan-progress-track" aria-label="体检进度">
        <div style={{ width: `${liveScan.progress}%` }} />
      </div>

      <div className="scan-summary-strip">
        <span className="summary-pass">通过 {passed}</span>
        <span className="summary-warn">关注 {warned}</span>
        <span className="summary-fail">失败 {failed}</span>
      </div>

      <div className="scan-step-list">
        {liveScan.steps.map((step, index) => (
          <article className={`scan-step ${step.state}`} key={step.id}>
            <div className="scan-step-index">{index + 1}</div>
            <div className="scan-step-icon">{stepIcon(step.state)}</div>
            <div className="scan-step-body">
              <div>
                <strong>{step.title}</strong>
                <span>{stepStateText(step.state)}</span>
              </div>
              <p>{step.message}</p>
            </div>
            <small>{typeof step.durationMs === "number" ? `${step.durationMs} ms` : ""}</small>
          </article>
        ))}
      </div>
    </section>
  );
}

function ResultPanel({ scan }: { scan: InvokeResult<QuickScanResult> | null }) {
  if (!scan) {
    return <EmptyState title="尚未体检" description="点击开始体检后，这里会显示真实扫描结果和下一步动作。" />;
  }

  if (scan.state !== "ok") {
    return <EmptyState title="扫描接口未就绪" description={scan.message || pendingText} />;
  }

  return (
    <section className="panel">
      <div className="panel-heading">
        <h2>最近一次问题摘要</h2>
        <span className="badge info">{scan.data.target ?? scan.data.scannedAt}</span>
      </div>
      {scan.data.checks && scan.data.checks.length > 0 && (
        <div className="check-result-grid">
          {scan.data.checks.map((check) => (
            <div className={`check-result ${check.status}`} key={check.id}>
              <strong>{check.title}</strong>
              <span>{check.message}</span>
              <small>{check.durationMs} ms</small>
            </div>
          ))}
        </div>
      )}
      {scan.data.findings.length === 0 ? (
        <EmptyState title="没有返回问题" description="后端扫描命令已执行，但没有返回 Finding。" />
      ) : (
        <div className="finding-list">
          {scan.data.findings.map((finding) => (
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
      )}
    </section>
  );
}

function ChecklistPanel({ title, items }: { title: string; items: string[] }) {
  return (
    <section className="panel">
      <div className="panel-heading">
        <h2>{title}</h2>
        <span className="badge muted">等待接口适配</span>
      </div>
      <div className="check-list">
        {items.map((item) => (
          <div className="check-item" key={item}>
            <span className="check-dot" />
            <span>{item}</span>
          </div>
        ))}
      </div>
    </section>
  );
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
    active: event.phase !== "finished" && event.phase !== "failed",
    progress: event.progress,
    completed: event.completed,
    total: event.total,
    currentStepId: event.currentStepId,
    message: event.message,
    finishedAt: event.phase === "finished" || event.phase === "failed" ? event.emittedAt : current.finishedAt,
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

function scanCheckState(check: ScanCheck): LiveScanStep["state"] {
  if (check.status === "pass") return "pass";
  if (check.status === "warn") return "warn";
  if (check.status === "fail") return "fail";
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

function invokeDetail<T>(result: InvokeResult<T> | null): string {
  if (!result) return "加载中";
  return result.state === "ok" ? statusText("ok") : result.message || statusText(result.state);
}
