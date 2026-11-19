import { useState } from "react";
import { AnimatePresence, motion } from "framer-motion";
import { ChevronDown, UserRound, ShieldCheck, Ghost, Plus } from "lucide-react";
import { useStore } from "../../lib/store";
import { addOfflineAccount, listAccounts, msLoginFinish, msLoginStart, openUrl, removeAccount } from "../../lib/tauri";
import type { AccountKind } from "../../lib/tauri";

const kindMeta: Record<AccountKind, { label: string; icon: JSX.Element }> = {
  microsoft: { label: "Microsoft", icon: <ShieldCheck size={16} /> },
  offline: { label: "Offline", icon: <UserRound size={16} /> },
  guest: { label: "Khách", icon: <Ghost size={16} /> },
};

export default function AccountSwitcher() {
  const { accounts, activeId, setActive, setAccounts, msClientId } = useStore();
  const [msCode, setMsCode] = useState<string | null>(null);
  const [open, setOpen] = useState(false);
  const [name, setName] = useState("");
  const [error, setError] = useState("");
  const active = accounts.find((a) => a.id === activeId);

  const addOffline = async () => {
    const n = name.trim();
    if (!n) return;
    try {
      await addOfflineAccount(n); // Rust tạo UUID offline
      setAccounts(await listAccounts());
      setName("");
      setError("");
    } catch (e) {
      setError(String(e));
    }
  };

  const msLogin = async () => {
    if (!msClientId.trim()) { setError("Nhập Azure Client ID ở tab Cài đặt trước"); return; }
    setError("");
    try {
      const d = await msLoginStart(msClientId);
      setMsCode(d.userCode);
      openUrl(d.verificationUri);
      await msLoginFinish(msClientId, d.deviceCode, d.interval, d.expiresIn);
      setAccounts(await listAccounts());
      setMsCode(null);
    } catch (e) {
      setMsCode(null);
      setError(String(e));
    }
  };

  const remove = async (id: string) => {
    try { await removeAccount(id); setAccounts(await listAccounts()); } catch (e) { setError(String(e)); }
  };

  return (
    <div className="relative">
      <motion.button
        whileTap={{ scale: 0.97 }}
        onClick={() => setOpen((o) => !o)}
        className="liquid-glass flex items-center gap-3 rounded-full py-2 pl-2 pr-4"
      >
        <div className="flex h-9 w-9 items-center justify-center rounded-full bg-gradient-to-br from-sky-400 to-indigo-500 text-sm font-semibold text-white">
          {active?.name?.[0]?.toUpperCase() ?? "?"}
        </div>
        <div className="text-left leading-tight">
          <div className="text-sm font-medium">{active?.name ?? "Đang tải…"}</div>
          <div className="text-[11px]" style={{ color: "var(--text-dim)" }}>
            {active ? kindMeta[active.kind].label : ""}
          </div>
        </div>
        <ChevronDown size={16} className={open ? "rotate-180 transition" : "transition"} />
      </motion.button>

      <AnimatePresence>
        {open && (
          <motion.div
            initial={{ opacity: 0, y: -8, scale: 0.96 }}
            animate={{ opacity: 1, y: 0, scale: 1 }}
            exit={{ opacity: 0, y: -8, scale: 0.96 }}
            transition={{ type: "spring", stiffness: 380, damping: 28 }}
            className="liquid-glass absolute left-0 top-full z-40 mt-2 w-72 rounded-[26px] p-2"
          >
            {accounts.map((a) => (
              <button
                key={a.id}
                onClick={() => { setActive(a.id); setOpen(false); }}
                className={`flex w-full items-center gap-3 rounded-2xl px-3 py-2 text-left transition ${a.id === activeId ? "bg-white/20" : "hover:bg-white/10"}`}
              >
                {kindMeta[a.kind].icon}
                <span className="flex-1 text-sm">{a.name}</span>
                <span className="text-[11px]" style={{ color: "var(--text-dim)" }}>{kindMeta[a.kind].label}</span>
                {accounts.length > 1 && (
                  <span role="button" onClick={(e) => { e.stopPropagation(); remove(a.id); }} className="px-1 text-xs opacity-60 hover:opacity-100">✕</span>
                )}
              </button>
            ))}

            <div className="mt-2 flex gap-2 border-t border-white/10 p-2 pt-3">
              <input
                value={name}
                onChange={(e) => setName(e.target.value)}
                placeholder="Tên nhân vật offline"
                maxLength={16}
                className="min-w-0 flex-1 rounded-xl bg-white/10 px-3 py-2 text-sm outline-none placeholder:opacity-50"
              />
              <button onClick={addOffline} className="rounded-xl bg-sky-500 px-3 text-white"><Plus size={16} /></button>
            </div>
            {error && <div className="px-3 pb-1 text-xs text-red-400">{error}</div>}
            {msCode ? (
              <div className="mt-1 rounded-xl bg-white/10 px-3 py-2 text-xs">
                Nhập mã này tại trang Microsoft vừa mở:
                <div className="mt-1 text-center text-lg font-bold tracking-widest">{msCode}</div>
              </div>
            ) : (
              <button onClick={msLogin} className="mt-1 w-full rounded-xl px-3 py-2 text-left text-sm hover:bg-white/10">
                Đăng nhập Microsoft…
              </button>
            )}
          </motion.div>
        )}
      </AnimatePresence>
    </div>
  );
}
