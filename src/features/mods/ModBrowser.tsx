import { useEffect, useState } from "react";
import GlassCard from "../../components/GlassCard";
import { useStore } from "../../lib/store";
import { installCurse, installMod, listMods, searchCurse, searchMods, toggleMod, InstalledMod, ModHit } from "../../lib/tauri";

export default function ModBrowser() {
  const { loader, version, curseKey } = useStore();
  const [src, setSrc] = useState<"modrinth" | "curse">("modrinth");
  const [q, setQ] = useState("");
  const [hits, setHits] = useState<ModHit[]>([]);
  const [installed, setInstalled] = useState<InstalledMod[]>([]);
  const [msg, setMsg] = useState("");
  const [busyId, setBusyId] = useState("");
  const ok = loader !== "Vanilla";

  const refresh = () => listMods(version, loader).then(setInstalled).catch(() => {});
  const search = async () => {
    setMsg("Đang tìm…");
    try { setHits(src === "curse" ? await searchCurse(curseKey, q, version, loader) : await searchMods(q, version, loader)); setMsg(""); } catch (e) { setMsg(String(e)); setHits([]); }
  };
  useEffect(() => { if (ok) { refresh(); search(); } /* eslint-disable-next-line */ }, [version, loader, src]);

  const install = async (h: ModHit) => {
    setBusyId(h.projectId); setMsg("");
    try { const r = src === "curse" ? await installCurse(curseKey, h.projectId, version, loader) : await installMod(h.projectId, version, loader); setMsg(`Đã cài ${h.title} (${r.length} file, gồm phụ thuộc).`); refresh(); }
    catch (e) { setMsg(String(e)); } finally { setBusyId(""); }
  };
  const toggle = async (m: InstalledMod) => { await toggleMod(version, loader, m.file, !m.enabled); refresh(); };

  if (!ok) {
    return (
      <GlassCard className="mx-auto w-full max-w-2xl p-8 text-center">
        <h2 className="text-xl font-semibold">Kho Mod (Modrinth)</h2>
        <p className="mt-2 text-sm" style={{ color: "var(--text-dim)" }}>Chọn loader Fabric, Quilt, Forge hoặc NeoForge ở tab Chơi để tìm và cài mod.</p>
      </GlassCard>
    );
  }

  return (
    <GlassCard initial={{ opacity: 0, y: 20 }} animate={{ opacity: 1, y: 0 }} className="mx-auto flex max-h-full w-full max-w-3xl flex-col p-5">
      <h2 className="text-xl font-semibold">Kho Mod · {loader} {version}</h2>
      <div className="mt-2 flex gap-2">
        {(["modrinth", "curse"] as const).map((x) => (
          <button key={x} onClick={() => setSrc(x)} className={`rounded-full px-3 py-1 text-xs ${src === x ? "bg-sky-500 text-white" : "bg-white/10"}`}>
            {x === "modrinth" ? "Modrinth" : "CurseForge"}
          </button>
        ))}
      </div>
      <div className="mt-3 flex gap-2">
        <input value={q} onChange={(e) => setQ(e.target.value)} onKeyDown={(e) => e.key === "Enter" && search()}
          placeholder={`Tìm mod trên ${src === "curse" ? "CurseForge" : "Modrinth"}…`} className="min-w-0 flex-1 rounded-2xl bg-white/10 px-4 py-2 text-sm outline-none" />
        <button onClick={search} className="rounded-2xl bg-sky-500 px-4 text-sm text-white">Tìm</button>
      </div>
      {msg && <div className="mt-2 break-words text-xs">{msg}</div>}

      <div className="mt-3 grid flex-1 gap-2 overflow-y-auto sm:grid-cols-2">
        {hits.map((h) => (
          <div key={h.projectId} className="flex gap-3 rounded-2xl bg-white/10 p-3">
            {h.iconUrl ? <img src={h.iconUrl} className="h-12 w-12 rounded-xl" /> : <div className="h-12 w-12 rounded-xl bg-white/20" />}
            <div className="min-w-0 flex-1">
              <div className="truncate text-sm font-medium">{h.title}</div>
              <div className="line-clamp-2 text-xs" style={{ color: "var(--text-dim)" }}>{h.description}</div>
              <button onClick={() => install(h)} disabled={busyId === h.projectId}
                className="mt-1 rounded-full bg-sky-500 px-3 py-0.5 text-xs text-white disabled:opacity-60">
                {busyId === h.projectId ? "Đang cài…" : "Cài đặt"}
              </button>
            </div>
          </div>
        ))}
      </div>

      {installed.length > 0 && (
        <div className="mt-3 border-t border-white/10 pt-3">
          <div className="text-sm font-medium">Đã cài ({installed.length})</div>
          <div className="mt-1 max-h-28 space-y-1 overflow-y-auto text-xs">
            {installed.map((m) => (
              <label key={m.file} className="flex items-center gap-2">
                <input type="checkbox" checked={m.enabled} onChange={() => toggle(m)} className="accent-sky-500" />
                <span className={m.enabled ? "" : "line-through opacity-50"}>{m.file.replace(".disabled", "")}</span>
              </label>
            ))}
          </div>
        </div>
      )}
    </GlassCard>
  );
}
