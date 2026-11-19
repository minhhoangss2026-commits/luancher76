import GlassCard from "../../components/GlassCard";
import { aiReady, useStore } from "../../lib/store";

export default function AIPanel() {
  const { autoFix, aiCfg, history, set, setTab } = useStore();
  const ready = aiReady(aiCfg);
  return (
    <GlassCard initial={{ opacity: 0, y: 20 }} animate={{ opacity: 1, y: 0 }} className="mx-auto flex max-h-full w-full max-w-2xl flex-col p-6">
      <h2 className="text-xl font-semibold">Trợ lý AI · Chẩn đoán & Tự sửa lỗi</h2>
      <label className="mt-3 flex items-center gap-2 text-sm">
        <input type="checkbox" checked={autoFix} onChange={(e) => set({ autoFix: e.target.checked })} className="accent-sky-500" />
        Tự động sửa lỗi khi game bị văng (tối đa 2 lần/lượt chơi)
      </label>
      {!ready && (
        <div className="mt-3 rounded-2xl bg-amber-500/20 px-4 py-3 text-sm">
          Chưa cấu hình AI. <button className="underline" onClick={() => setTab("settings")}>Vào Cài đặt để nhập API Key</button> (hoặc chọn Ollama chạy cục bộ).
        </div>
      )}
      <div className="mt-4 text-sm font-medium">Lịch sử tự sửa lỗi</div>
      <div className="mt-2 flex-1 space-y-2 overflow-y-auto">
        {history.map((h) => (
          <div key={h.time} className="rounded-2xl bg-white/10 p-3 text-xs">
            <div style={{ color: "var(--text-dim)" }}>{new Date(h.time).toLocaleString()} · {h.loader} {h.version}</div>
            <div className="mt-1">{h.diagnosis}</div>
            {h.applied.map((a) => <div key={a} className="mt-0.5">🔧 {a}</div>)}
          </div>
        ))}
        {!history.length && <div className="text-xs" style={{ color: "var(--text-dim)" }}>Chưa có lỗi nào được xử lý.</div>}
      </div>
    </GlassCard>
  );
}
