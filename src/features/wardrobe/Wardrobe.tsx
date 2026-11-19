import { useEffect, useRef, useState } from "react";
import { SkinViewer, WalkingAnimation } from "skinview3d";
import GlassCard from "../../components/GlassCard";
import { useStore } from "../../lib/store";

import { Look, readLook, writeLook } from "../../lib/look";
const DEFAULT_SKIN = "/skins/default.png";

const readFile = (f: File) => new Promise<string>((res, rej) => {
  const r = new FileReader();
  r.onload = () => res(String(r.result));
  r.onerror = () => rej(new Error("Không đọc được file"));
  r.readAsDataURL(f);
});

export default function Wardrobe() {
  const { activeId, accounts } = useStore();
  const acc = accounts.find((a) => a.id === activeId);
  const canvas = useRef<HTMLCanvasElement>(null);
  const viewer = useRef<SkinViewer | null>(null);
  const [look, setLook] = useState<Look>({ slim: false });
  const [msg, setMsg] = useState("");

  // nạp bộ đồ của tài khoản đang chọn
  useEffect(() => {
    if (!activeId) return;
    setLook(readLook(activeId));
  }, [activeId]);

  useEffect(() => {
    const v = new SkinViewer({ canvas: canvas.current!, width: 280, height: 380 });
    v.autoRotate = true;
    v.animation = new WalkingAnimation();
    viewer.current = v;
    return () => { v.dispose(); viewer.current = null; };
  }, []);

  useEffect(() => {
    const v = viewer.current;
    if (!v) return;
    v.loadSkin(look.skin ?? DEFAULT_SKIN, { model: look.slim ? "slim" : "default" }).catch(() => setMsg("Skin không hợp lệ (cần PNG 64x64 hoặc 64x32)"));
    if (look.cape) v.loadCape(look.cape).catch(() => {}); else v.resetCape();
  }, [look]);

  const save = (l: Look) => { setLook(l); if (activeId) writeLook(activeId, l); };

  const pick = (kind: "skin" | "cape") => async (e: React.ChangeEvent<HTMLInputElement>) => {
    const f = e.target.files?.[0];
    if (!f) return;
    if (!f.type.includes("png")) { setMsg("Chỉ nhận file .png"); return; }
    setMsg("");
    save({ ...look, [kind]: await readFile(f) });
  };

  return (
    <GlassCard initial={{ opacity: 0, y: 20 }} animate={{ opacity: 1, y: 0 }} className="mx-auto flex w-full max-w-2xl flex-wrap items-center justify-center gap-6 p-6">
      <canvas ref={canvas} className="rounded-2xl bg-white/5" />
      <div className="min-w-[220px] flex-1 space-y-3 text-sm">
        <h2 className="text-xl font-semibold">Trang phục · {acc?.name ?? "—"}</h2>
        <label className="block">
          <span className="text-xs" style={{ color: "var(--text-dim)" }}>Skin (.png)</span>
          <input type="file" accept="image/png" onChange={pick("skin")} className="mt-1 block w-full text-xs" />
        </label>
        <label className="block">
          <span className="text-xs" style={{ color: "var(--text-dim)" }}>Áo choàng (.png)</span>
          <input type="file" accept="image/png" onChange={pick("cape")} className="mt-1 block w-full text-xs" />
        </label>
        <div className="flex gap-2">
          {[false, true].map((slim) => (
            <button key={String(slim)} onClick={() => save({ ...look, slim })}
              className={`rounded-full px-4 py-1.5 text-xs ${look.slim === slim ? "bg-sky-500 text-white" : "bg-white/10"}`}>
              {slim ? "Slim" : "Classic"}
            </button>
          ))}
        </div>
        <button onClick={() => save({ slim: false })} className="rounded-full bg-white/10 px-4 py-1.5 text-xs">Đặt lại mặc định</button>
        <p className="text-[11px]" style={{ color: "var(--text-dim)" }}>
          Bộ đồ được lưu riêng cho từng tài khoản. Tài khoản Offline/Khách: skin được áp vào game tự động khi bấm PLAY (qua resource pack, tốt nhất ở Minecraft 1.19.3 trở lên). Tài khoản Microsoft dùng skin thật của Mojang. Áo choàng chỉ xem trong launcher.
        </p>
        {msg && <div className="text-xs text-red-400">{msg}</div>}
      </div>
    </GlassCard>
  );
}
