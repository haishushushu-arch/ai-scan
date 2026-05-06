export function statusText(state: string): string {
  switch (state) {
    case "ok":
      return "已接通";
    case "not_implemented":
      return "等待接口适配";
    case "unavailable":
      return "未在 Tauri 环境";
    default:
      return "需要检查";
  }
}

export function severityLabel(severity: string): string {
  if (severity === "error") return "错误";
  if (severity === "warning" || severity === "medium" || severity === "high") return "警告";
  if (severity === "low") return "提示";
  return "信息";
}

export function apiKeyStatusText(status: string): string {
  switch (status) {
    case "active":
      return "可用";
    case "inactive":
      return "已停用";
    case "quota_exhausted":
      return "额度用尽";
    case "expired":
      return "已过期";
    case "disabled":
      return "已禁用";
    default:
      return "未知";
  }
}

export function formatOptionalDate(value?: string): string {
  if (!value) return "无";
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return value;
  return date.toLocaleString();
}
