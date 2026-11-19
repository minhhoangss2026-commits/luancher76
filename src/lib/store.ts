import { create } from "zustand";
import { persist } from "zustand/middleware";
import type { Account } from "./tauri";

export type Tab = "play" | "boost" | "mods" | "wardrobe" | "ai" | "settings";
export type Provider = "anthropic" | "openai" | "gemini" | "ollama";

export interface AiCfg { provider: Provider; apiKey: string; model: string; baseUrl: string }
export interface FixEntry { time: number; version: string; loader: string; diagnosis: string; applied: string[] }

interface State {
  accounts: Account[];
  activeId: string | null;
  tab: Tab;
  aiOpen: boolean;
  // cài đặt được lưu lại
  loader: string;
  version: string;
  ram: number;
  boost: boolean;
  autoFix: boolean;
  aiCfg: AiCfg;
  history: FixEntry[];
  msClientId: string;
  curseKey: string;

  setAccounts: (a: Account[]) => void;
  setActive: (id: string) => void;
  setTab: (t: Tab) => void;
  toggleAI: () => void;
  set: (p: Partial<Pick<State, "loader" | "version" | "ram" | "boost" | "autoFix" | "aiCfg" | "msClientId" | "curseKey">>) => void;
  addHistory: (e: FixEntry) => void;
}

export const useStore = create<State>()(
  persist(
    (set) => ({
      accounts: [],
      activeId: null,
      tab: "play",
      aiOpen: false,
      loader: "Vanilla",
      version: "1.21.1",
      ram: 4096,
      boost: true,
      autoFix: true,
      aiCfg: { provider: "anthropic", apiKey: "", model: "", baseUrl: "" },
      history: [],
      msClientId: "",
      curseKey: "",

      setAccounts: (accounts) =>
        set((s) => ({ accounts, activeId: accounts.some((a) => a.id === s.activeId) ? s.activeId : accounts[0]?.id ?? null })),
      setActive: (activeId) => set({ activeId }),
      setTab: (tab) => set({ tab }),
      toggleAI: () => set((s) => ({ aiOpen: !s.aiOpen })),
      set: (p) => set(p),
      addHistory: (e) => set((s) => ({ history: [e, ...s.history].slice(0, 50) })),
    }),
    {
      name: "cosmic.settings",
      partialize: (s) => ({
        activeId: s.activeId, loader: s.loader, version: s.version, ram: s.ram,
        boost: s.boost, autoFix: s.autoFix, aiCfg: s.aiCfg, history: s.history,
        msClientId: s.msClientId, curseKey: s.curseKey,
      }),
    }
  )
);

export const aiReady = (c: AiCfg) => c.provider === "ollama" || c.apiKey.trim().length > 0;
