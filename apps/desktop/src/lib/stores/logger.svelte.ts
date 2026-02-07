import { attachConsole, debug, error, info, trace, warn } from "@tauri-apps/plugin-log";

export async function initLogger(): Promise<void> {
  await attachConsole();
  info("Logger initialized");
}

export function logDebug(message: string, data?: Record<string, unknown>): void {
  const fullMessage = data ? `${message} ${JSON.stringify(data)}` : message;
  debug(fullMessage).catch(() => console.debug("[AgentV]", message, data));
}

export function logInfo(message: string, data?: Record<string, unknown>): void {
  const fullMessage = data ? `${message} ${JSON.stringify(data)}` : message;
  info(fullMessage).catch(() => console.info("[AgentV]", message, data));
}

export function logWarn(message: string, data?: Record<string, unknown>): void {
  const fullMessage = data ? `${message} ${JSON.stringify(data)}` : message;
  warn(fullMessage).catch(() => console.warn("[AgentV]", message, data));
}

export function logError(message: string, data?: Record<string, unknown>): void {
  const fullMessage = data ? `${message} ${JSON.stringify(data)}` : message;
  error(fullMessage).catch(() => console.error("[AgentV]", message, data));
}

export function logTrace(message: string, data?: Record<string, unknown>): void {
  const fullMessage = data ? `${message} ${JSON.stringify(data)}` : message;
  trace(fullMessage).catch(() => console.log("[AgentV]", message, data));
}
