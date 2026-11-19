import { useEffect, useRef, useState } from "react";
import { motion } from "framer-motion";
import { Play } from "lucide-react";
import GlassCard from "../../components/GlassCard";
import { aiReady, useStore } from "../../lib/store";
import { aiAutofix, applySkin, launchGame, listVersions, onGameExit, onProgress } from "../../lib/tauri";
import { readLook } from "../../lib/look";
import { isMobile } from "../../lib/platform";

const LOADERS = ["Vanilla", "Fabric", "Quilt", "Forge", "NeoForge"] as const;
const HINT: Record<string, string> = {
  Forge: "Forge: hỗ trợ từ Minecraft 1.13. Lần đầu cài mất vài phút.",
  NeoForge: "NeoForge: hỗ trợ từ Minecraft 1.20.2. Lần đầu cài mất vài phút.",
};
const SUPPORTED = ["Vanilla", "Fabric", "Quilt", "Forge", "NeoForge"];
const FALLBACK = ["1.21.4", "1.21.1", "1.20.4", "1.20.1", "1.19.4", "1.18.2", "1.16.5", "1.12.2", "1.8.9"];

export default function PlayPanel() {
  const s = useStore();
  const { activeId, loader, version, ram, boost, autoFix, aiCfg } = s;
  const [versions, setVersions] = useState<string[]>(FALLBACK);
  const [busy, setBusy] = useState(false);
  const [running, setRunning] = useState(false);
  const [progress, setProgress] = useState(0);
  const [status, setStatus] = useState("");
  const [error, setError] = useState("");
  const attempts = useRef(0);

  // luôn giữ giá trị mới nhất cho callback sự kiện
  const latest = useRef({ activeId, loader, version, ram, boost, autoFix, aiCfg });
  latest.current = { activeId, loader, version, ram, boost, autoFix, aiCfg };

  useEffect(() => {
    listVersions().then((v) => v.length && setVersions(v)).catch(() => {});
  }, []);

  const start = async (ramMb: number) => {
    const L = latest.current;
    if (!L.activeId) return;
    // Offline/Khách: áp skin đã chọn vào game bằng resource pack
    const acc = useStore.getState().accounts.find((a) => a.id === L.activeId);
    if (acc && acc.kind !== "microsoft") {
      await applySkin(L.version, L.loader, readLook(L.activeId).skin ?? null).catch(() => {});
    }
    await launchGame(L.activeId, L.version, L.loader, ramMb, L.boost);
    setRunning(true);
  };

  // Auto-Heal: game văng -> AI đọc log -> sửa -> khởi động lại
  const heal = async () => {
    const L = latest.current;
    if (attempts.current >= 2) {
      setError("AI đã thử sửa 2 lần nhưng game vẫn lỗi. Xem lịch sử ở tab Trợ lý AI.");
      attempts.current = 0;
      return;
    }
    attempts.current += 1;
    setBusy(true);
    setProgress(100);
    setStatus("AI đang phân tích lỗi…");
    try {
      const r = await aiAutofix(L.aiCfg, L.version, L.loader, L.ram);
      useStore.getState().addHistory({ time: Date.now(), version: L.version, loader: L.loader, diagnosis: r.diagnosis, applied: r.applied });
      if (r.ramMb) useStore.getState().set({ ram: r.ramMb });
      if (r.applied.length) {
        setStatus("Đã sửa lỗi, khởi động lại…");
        await start(r.ramMb ?? L.ram);
      } else {
        setError(`AI: ${r.diagnosis} (chưa có cách tự sửa)`);
      }
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
      setStatus("");
    }
  };

  useEffect(() => {
    let offP: (() => void) | undefined;
    let offE: (() => void) | undefined;
    onProgress((p) => {
      setStatus(p.total > 1 && p.stage.startsWith("Tải") ? `${p.stage} ${p.current}/${p.total}` : p.stage);
      setProgress(p.percent);
    }).then((f) => (offP = f));
    onGameExit((code) => {
      setRunning(false);
      setStatus("");
      setProgress(0);
      const L = latest.current;
      if (code !== null && code !== 0 && L.autoFix && aiReady(L.aiCfg)) heal();
    }).then((f) => (offE = f));
    return () => { offP?.(); offE?.(); };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const play = async () => {
    if (!activeId || busy || running) return;
    attempts.current = 0;
    setError("");
    setBusy(true);
    setProgress(0);
    setStatus("Bắt đầu…");
    try {
      await start(ram);
    } catch (e) {
      setError(String(e));
      setStatus("");
      setProgress(0);
    } finally {
      setBusy(false);
    }
  };

  const label = running ? "ĐANG CHẠY…" : busy ? status : "PLAY";

  return (
    <GlassCard initial={{ opacity: 0, y: 24 }} animate={{ opacity: 1, y: 0 }}
      transition={{ type: "spring", stiffness: 160, damping: 20 }} className="mx-auto w-full max-w-2xl p-6">
      <h2 className="text-2xl font-semibold tracking-tight">Sẵn sàng chơi</h2>

      <div className="mt-5 flex flex-wrap gap-2">
        {LOADERS.map((l) => {
          const ok = SUPPORTED.includes(l);
          return (
            <button key={l} disabled={!ok} onClick={() => s.set({ loader: l })}
              className={`rounded-full px-4 py-1.5 text-sm transition ${loader === l ? "bg-sky-500 text-white" : "bg-white/10 hover:bg-white/20"} ${ok ? "" : "cursor-not-allowed opacity-40"}`}>
              {l}{ok ? "" : " · sắp có"}
            </button>
          );
        })}
      </div>

      {HINT[loader] && <div className="mt-2 text-xs" style={{ color: "var(--text-dim)" }}>{HINT[loader]}</div>}

      <label className="mt-5 block text-xs" style={{ color: "var(--text-dim)" }}>Phiên bản Minecraft</label>
      <select value={version} onChange={(e) => s.set({ version: e.target.value })}
        className="mt-1 w-full rounded-2xl bg-white/10 px-4 py-3 text-sm outline-none">
        {(versions.includes(version) ? versions : [version, ...versions]).map((v) => (
          <option key={v} value={v} className="text-black">{v}</option>
        ))}
      </select>

      <div className="mt-5">
        <div className="flex justify-between text-xs" style={{ color: "var(--text-dim)" }}>
          <span>RAM cấp phát</span><span>{(ram / 1024).toFixed(1)} GB</span>
        </div>
        <input type="range" min={1024} max={16384} step={512} value={ram}
          onChange={(e) => s.set({ ram: +e.target.value })} className="mt-2 w-full accent-sky-500" />
      </div>

      <label className="mt-4 flex items-center gap-2 text-sm">
        <input type="checkbox" checked={boost} onChange={(e) => s.set({ boost: e.target.checked })} className="accent-sky-500" />
        FPS Boost (cờ JVM tối ưu{loader !== "Vanilla" ? " + tự cài mod tăng FPS" : ""})
      </label>

      <motion.button onClick={play} disabled={busy || running} whileTap={{ scale: 0.97 }} whileHover={{ scale: 1.02 }}
        className="relative mt-6 flex h-16 w-full items-center justify-center gap-3 overflow-hidden rounded-[26px] bg-gradient-to-r from-sky-500 to-indigo-500 text-xl font-semibold text-white shadow-lg disabled:opacity-90">
        {busy && (
          <motion.div className="absolute inset-y-0 left-0 bg-white/25" animate={{ width: `${progress}%` }}
            transition={{ type: "spring", stiffness: 120, damping: 20 }} />
        )}
        <span className="relative flex items-center gap-2">
          {!busy && !running && <Play fill="white" size={22} />} {label}
        </span>
      </motion.button>

      {isMobile() && (
        <div className="mt-3 rounded-2xl bg-amber-500/20 px-4 py-3 text-xs">
          Android/iOS: bản này mới có giao diện, chưa chạy được game trên điện thoại (xem docs/MOBILE.md).
        </div>
      )}
      {error && <div className="mt-3 break-words rounded-2xl bg-red-500/20 px-4 py-3 text-sm text-red-300">{error}</div>}
    </GlassCard>
  );
}
