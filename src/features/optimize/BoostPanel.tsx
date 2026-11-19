import { useEffect, useState } from "react";
import GlassCard from "../../components/GlassCard";
import { useStore } from "../../lib/store";
import { applyGraphics, installBoost, listMods, InstalledMod } from "../../lib/tauri";

const FPS_FABRIC = ["Sodium", "Lithium", "FerriteCore", "ImmediatelyFast", "Entity Culling"];
const FPS_FORGE = ["Embeddium", "FerriteCore", "ModernFix", "ImmediatelyFast", "Entity Culling"];

export default function BoostPanel() {
  const { loader, version, boost, set } = useStore();
  const [mods, setMods] = useState<InstalledMod[]>([]);
  const [msg, setMsg] = useState("");
  const [busy, setBusy] = useState(false);
  const modded = loader !== "Vanilla";
  const FPS_LIST = loader === "Forge" || loader === "NeoForge" ? FPS_FORGE : FPS_FABRIC;

  const refresh = () => listMods(version, loader).then(setMods).catch(() => {});
  useEffect(() => { refresh(); /* eslint-disable-next-line */ }, [version, loader]);

  const has = (name: string) => mods.some((m) => m.enabled && m.file.toLowerCase().replace(/[-_ ]/g, "").includes(name.toLowerCase().replace(/[-_ ]/g, "")));

  const install = async () => {
    setBusy(true); setMsg("Đang cài bộ mod FPS…");
    try {
      const r = await installBoost(version, loader);
      setMsg(`Đã cài: ${r.installed.join(", ") || "không có"}${r.skipped.length ? ` · Chưa hỗ trợ bản ${version}: ${r.skipped.join(", ")}` : ""}`);
      refresh();
    } catch (e) { setMsg(String(e)); } finally { setBusy(false); }
  };

  const preset = async (p: string) => {
    try { await applyGraphics(version, loader, p); setMsg(p === "performance" ? "Đã áp dụng đồ họa Hiệu năng (có hiệu lực lần chạy sau)." : "Đã khôi phục đồ họa Chất lượng."); }
    catch (e) { setMsg(String(e)); }
  };

  return (
    <GlassCard initial={{ opacity: 0, y: 20 }} animate={{ opacity: 1, y: 0 }} className="mx-auto w-full max-w-2xl p-6">
      <h2 className="text-xl font-semibold">Tối ưu hóa & FPS Boost</h2>
      <p className="mt-1 text-xs" style={{ color: "var(--text-dim)" }}>Hồ sơ hiện tại: {loader} {version}</p>

      <label className="mt-4 flex items-center gap-2 text-sm">
        <input type="checkbox" checked={boost} onChange={(e) => set({ boost: e.target.checked })} className="accent-sky-500" />
        Cờ JVM tối ưu (G1GC giảm giật, áp dụng cả máy ARM64 / Apple Silicon)
      </label>

      <div className="mt-5 text-sm font-medium">Bộ mod tăng FPS</div>
      {modded ? (
        <>
          <ul className="mt-2 grid grid-cols-2 gap-1 text-sm">
            {FPS_LIST.map((n) => <li key={n}>{has(n) ? "✅" : "⬜"} {n}</li>)}
          </ul>
          <button onClick={install} disabled={busy} className="mt-3 rounded-2xl bg-sky-500 px-4 py-2 text-sm font-medium text-white disabled:opacity-60">
            {busy ? "Đang cài…" : "Cài / cập nhật bộ mod FPS"}
          </button>
        </>
      ) : (
        <p className="mt-2 text-sm" style={{ color: "var(--text-dim)" }}>Chọn loader Fabric, Quilt, Forge hoặc NeoForge ở tab Chơi để dùng bộ mod FPS.</p>
      )}

      <div className="mt-6 text-sm font-medium">Đồ họa nhanh</div>
      <div className="mt-2 flex gap-2">
        <button onClick={() => preset("performance")} className="rounded-2xl bg-white/15 px-4 py-2 text-sm hover:bg-white/25">⚡ Hiệu năng</button>
        <button onClick={() => preset("quality")} className="rounded-2xl bg-white/15 px-4 py-2 text-sm hover:bg-white/25">✨ Chất lượng</button>
      </div>
      {msg && <div className="mt-4 break-words rounded-2xl bg-white/10 px-4 py-3 text-xs">{msg}</div>}
    </GlassCard>
  );
}
