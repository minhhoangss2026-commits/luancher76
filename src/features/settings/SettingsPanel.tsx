import { useEffect, useState } from "react";
import GlassCard from "../../components/GlassCard";
import { Provider, useStore } from "../../lib/store";
import { getDataDir, listRuntimes, openUrl, removeRuntime, setDataDir } from "../../lib/tauri";
import { WEB_URL } from "../../config";

const PROVIDERS: { id: Provider; label: string; model: string }[] = [
  { id: "anthropic", label: "Anthropic (Claude)", model: "claude-sonnet-5-5" },
  { id: "openai", label: "OpenAI", model: "gpt-4o-mini" },
  { id: "gemini", label: "Google Gemini", model: "gemini-2.0-flash" },
  { id: "ollama", label: "Ollama (chạy cục bộ)", model: "llama3.1" },
];

export default function SettingsPanel() {
  const { aiCfg, accounts, set, msClientId, curseKey } = useStore();
  const [dir, setDir] = useState("");
  const [newDir, setNewDir] = useState("");
  const [runtimes, setRuntimes] = useState<string[]>([]);
  const [note, setNote] = useState("");
  const loadRt = () => listRuntimes().then(setRuntimes).catch(() => {});
  useEffect(() => { getDataDir().then(setDir); loadRt(); }, []);
  const saveDir = async () => {
    try { await setDataDir(newDir); setNote("Đã lưu. Khởi động lại launcher để dùng thư mục mới (dữ liệu cũ không tự chuyển)."); }
    catch (e) { setNote(String(e)); }
  };
  const delRt = async (n: string) => { try { await removeRuntime(n); loadRt(); } catch (e) { setNote(String(e)); } };
  const cur = PROVIDERS.find((p) => p.id === aiCfg.provider)!;
  const upd = (p: Partial<typeof aiCfg>) => set({ aiCfg: { ...aiCfg, ...p } });
  const input = "mt-1 w-full rounded-2xl bg-white/10 px-4 py-2 text-sm outline-none";

  return (
    <GlassCard initial={{ opacity: 0, y: 20 }} animate={{ opacity: 1, y: 0 }} className="mx-auto max-h-full w-full max-w-2xl space-y-5 overflow-y-auto p-6 text-sm">
      <h2 className="text-xl font-semibold">Cài đặt</h2>

      <section>
        <div className="font-medium">Trợ lý AI</div>
        <label className="mt-2 block text-xs" style={{ color: "var(--text-dim)" }}>Nhà cung cấp</label>
        <select value={aiCfg.provider} onChange={(e) => upd({ provider: e.target.value as Provider, model: "" })} className={input}>
          {PROVIDERS.map((p) => <option key={p.id} value={p.id} className="text-black">{p.label}</option>)}
        </select>
        {aiCfg.provider !== "ollama" && (
          <>
            <label className="mt-2 block text-xs" style={{ color: "var(--text-dim)" }}>API Key (lưu trên máy bạn)</label>
            <input type="password" value={aiCfg.apiKey} onChange={(e) => upd({ apiKey: e.target.value })} placeholder="sk-…" className={input} />
          </>
        )}
        <label className="mt-2 block text-xs" style={{ color: "var(--text-dim)" }}>Model (để trống = {cur.model})</label>
        <input value={aiCfg.model} onChange={(e) => upd({ model: e.target.value })} placeholder={cur.model} className={input} />
        {(aiCfg.provider === "ollama" || aiCfg.provider === "openai") && (
          <>
            <label className="mt-2 block text-xs" style={{ color: "var(--text-dim)" }}>Base URL (tùy chọn)</label>
            <input value={aiCfg.baseUrl} onChange={(e) => upd({ baseUrl: e.target.value })}
              placeholder={aiCfg.provider === "ollama" ? "http://localhost:11434" : "https://api.openai.com/v1"} className={input} />
          </>
        )}
      </section>

      <section>
        <div className="font-medium">Đăng nhập Microsoft</div>
        <label className="mt-2 block text-xs" style={{ color: "var(--text-dim)" }}>Azure Client ID (ứng dụng public client, đã được Mojang cho phép dùng Minecraft API)</label>
        <input value={msClientId} onChange={(e) => set({ msClientId: e.target.value })} placeholder="xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx" className={input} />
      </section>

      <section>
        <div className="font-medium">CurseForge</div>
        <label className="mt-2 block text-xs" style={{ color: "var(--text-dim)" }}>API Key (lấy tại console.curseforge.com)</label>
        <input type="password" value={curseKey} onChange={(e) => set({ curseKey: e.target.value })} className={input} />
      </section>

      <section>
        <div className="font-medium">Tài khoản ({accounts.length})</div>
        <ul className="mt-1 text-xs" style={{ color: "var(--text-dim)" }}>
          {accounts.map((a) => <li key={a.id}>{a.name} · {a.kind}</li>)}
        </ul>
      </section>

      <section>
        <div className="font-medium">Thư mục dữ liệu độc lập</div>
        <div className="mt-1 break-all rounded-2xl bg-white/10 px-4 py-2 text-xs">{dir}</div>
        <p className="mt-1 text-[11px]" style={{ color: "var(--text-dim)" }}>Game, Java và mod nằm trong đây, không đụng thư mục .minecraft.</p>
        <div className="mt-2 flex gap-2">
          <input value={newDir} onChange={(e) => setNewDir(e.target.value)} placeholder="Đường dẫn thư mục mới (để trống = mặc định)" className={input} />
          <button onClick={saveDir} className="mt-1 rounded-2xl bg-sky-500 px-4 text-xs text-white">Lưu</button>
        </div>
      </section>

      <section>
        <div className="font-medium">Java Runtime đã tải</div>
        <ul className="mt-1 space-y-1 text-xs">
          {runtimes.map((r) => (
            <li key={r} className="flex items-center justify-between rounded-xl bg-white/10 px-3 py-1.5">
              <span>{r.replace("jre-", "Java ")}</span>
              <button onClick={() => delRt(r)} className="text-red-400">Xóa</button>
            </li>
          ))}
          {!runtimes.length && <li style={{ color: "var(--text-dim)" }}>Chưa có. Java tự tải khi bạn bấm PLAY lần đầu.</li>}
        </ul>
      </section>
      {note && <div className="break-words rounded-2xl bg-white/10 px-4 py-3 text-xs">{note}</div>}

      <section>
        <div className="font-medium">Trang web thông báo</div>
        <button onClick={() => openUrl(WEB_URL)} className="mt-1 break-all text-left text-xs text-sky-400 underline">{WEB_URL}</button>
      </section>
    </GlassCard>
  );
}
