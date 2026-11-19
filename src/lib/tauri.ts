import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { AiCfg } from "./store";

export type AccountKind = "microsoft" | "offline" | "guest";
export interface Account { id: string; kind: AccountKind; name: string; uuid: string; skinUrl?: string }
export interface Progress { stage: string; percent: number; current: number; total: number }
export interface ModHit { projectId: string; slug: string; title: string; description: string; iconUrl?: string; downloads: number; author: string }
export interface InstalledMod { file: string; enabled: boolean }
export interface FixResult { diagnosis: string; applied: string[]; ramMb: number | null }

export const inTauri = "__TAURI_INTERNALS__" in window;

const only = () => Promise.reject(new Error("Tính năng này chỉ hoạt động trong ứng dụng desktop"));
function call<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  return inTauri ? invoke<T>(cmd, args) : only();
}

// ---- dữ liệu giả khi xem giao diện trên trình duyệt ----
let mockAccounts: Account[] = [{ id: "g1", kind: "guest", name: "Player_4821", uuid: "0-0-0-0-0" }];
const mockCbs = new Set<(p: Progress) => void>();

export async function listAccounts(): Promise<Account[]> {
  return inTauri ? invoke<Account[]>("list_accounts") : mockAccounts;
}
export async function addOfflineAccount(name: string): Promise<Account> {
  if (!inTauri) {
    const a: Account = { id: crypto.randomUUID(), kind: "offline", name, uuid: crypto.randomUUID() };
    mockAccounts = [...mockAccounts, a];
    return a;
  }
  return invoke<Account>("add_offline_account", { name });
}
export async function listVersions(): Promise<string[]> {
  return inTauri ? invoke<string[]>("list_versions") : ["1.21.4", "1.21.1", "1.20.4", "1.20.1", "1.19.4", "1.18.2", "1.16.5", "1.12.2", "1.8.9"];
}
export async function launchGame(accountId: string, version: string, loader: string, ramMb: number, boost: boolean): Promise<void> {
  if (!inTauri) {
    const steps = ["Tải game", "Tải thư viện", "Tải assets", "Chuẩn bị Java", "Game đang chạy"];
    for (let i = 0; i < steps.length; i++) {
      mockCbs.forEach((cb) => cb({ stage: steps[i], percent: 100, current: i + 1, total: steps.length }));
      await new Promise((r) => setTimeout(r, 400));
    }
    return;
  }
  return invoke("launch_game", { accountId, version, loader, ramMb, boost });
}
export const getDataDir = () => (inTauri ? invoke<string>("get_data_dir") : Promise.resolve("(chỉ có trong app desktop)"));

// ---- mod ----
export const searchMods = (query: string, version: string, loader: string) => call<ModHit[]>("search_mods", { query, version, loader });
export const installMod = (project: string, version: string, loader: string) => call<string[]>("install_mod", { project, version, loader });
export const installBoost = (version: string, loader: string) => call<{ installed: string[]; skipped: string[] }>("install_boost", { version, loader });
export const listMods = (version: string, loader: string) => (inTauri ? invoke<InstalledMod[]>("list_mods", { version, loader }) : Promise.resolve([]));
export const toggleMod = (version: string, loader: string, file: string, enable: boolean) => call<boolean>("toggle_mod", { version, loader, file, enable });
export const applyGraphics = (version: string, loader: string, preset: string) => call<void>("apply_graphics", { version, loader, preset });

// ---- AI ----
export const aiChat = (cfg: AiCfg, system: string, messages: { role: string; content: string }[]) => call<string>("ai_chat", { cfg, system, messages });
export const aiAutofix = (cfg: AiCfg, version: string, loader: string, ramMb: number) => call<FixResult>("ai_autofix", { cfg, version, loader, ramMb });

// ---- sự kiện ----
export function onProgress(cb: (p: Progress) => void): Promise<() => void> {
  if (!inTauri) { mockCbs.add(cb); return Promise.resolve(() => { mockCbs.delete(cb); }); }
  return listen<Progress>("launch-progress", (e) => cb(e.payload));
}
export function onGameExit(cb: (code: number | null) => void): Promise<() => void> {
  if (!inTauri) return Promise.resolve(() => {});
  return listen<{ code: number | null }>("game-exit", (e) => cb(e.payload.code));
}

// ---- mở link ngoài ----
export async function openUrl(url: string) {
  if (inTauri) {
    const { open } = await import("@tauri-apps/plugin-shell");
    await open(url);
  } else {
    window.open(url, "_blank");
  }
}

// ---- Microsoft ----
export interface MsStart { userCode: string; verificationUri: string; deviceCode: string; interval: number; expiresIn: number }
export const msLoginStart = (clientId: string) => call<MsStart>("ms_login_start", { clientId });
export const msLoginFinish = (clientId: string, deviceCode: string, interval: number, expiresIn: number) =>
  call<Account>("ms_login_finish", { clientId, deviceCode, interval, expiresIn });
export const removeAccount = (id: string) => call<void>("remove_account", { id });

// ---- CurseForge ----
export const searchCurse = (key: string, query: string, version: string, loader: string) => call<ModHit[]>("search_curseforge", { key, query, version, loader });
export const installCurse = (key: string, modId: string, version: string, loader: string) => call<string[]>("install_curseforge", { key, modId, version, loader });

// ---- skin / java / thư mục ----
export const applySkin = (version: string, loader: string, dataUrl: string | null) => (inTauri ? invoke<void>("apply_skin", { version, loader, dataUrl }) : Promise.resolve());
export const listRuntimes = () => (inTauri ? invoke<string[]>("list_runtimes") : Promise.resolve([] as string[]));
export const removeRuntime = (name: string) => call<void>("remove_runtime", { name });
export const setDataDir = (path: string) => call<void>("set_data_dir", { path });
