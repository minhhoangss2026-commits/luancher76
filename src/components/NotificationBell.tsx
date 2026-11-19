import { useEffect, useState } from "react";
import { AnimatePresence, motion } from "framer-motion";
import { Bell, ExternalLink } from "lucide-react";
import { NEWS_URL, WEB_URL } from "../config";
import { openUrl } from "../lib/tauri";

interface NewsItem { id: number; title: string; body: string; date: string }
const SEEN_KEY = "cosmic.news.seen";

export default function NotificationBell() {
  const [items, setItems] = useState<NewsItem[]>([]);
  const [open, setOpen] = useState(false);
  const [seen, setSeen] = useState<number>(() => Number(localStorage.getItem(SEEN_KEY) ?? 0));
  const [err, setErr] = useState(false);

  // Lấy thông báo từ trang web khi mở app và mỗi 10 phút
  useEffect(() => {
    const load = () =>
      fetch(NEWS_URL, { cache: "no-store" })
        .then((r) => r.json())
        .then((d: NewsItem[]) => {
          // Lên lịch: chỉ hiện thông báo đã đến giờ đăng (date <= bây giờ)
          setItems(d.filter((i) => new Date(i.date).getTime() <= Date.now()));
          setErr(false);
        })
        .catch(() => setErr(true));
    load();
    const t = setInterval(load, 10 * 60 * 1000);
    return () => clearInterval(t);
  }, []);

  const unread = items.filter((i) => i.id > seen).length;
  const toggle = () => {
    setOpen((o) => !o);
    if (!open && items.length) {
      const max = Math.max(...items.map((i) => i.id));
      localStorage.setItem(SEEN_KEY, String(max));
      setSeen(max);
    }
  };

  return (
    <div className="relative">
      <motion.button whileTap={{ scale: 0.92 }} onClick={toggle} aria-label="Thông báo"
        className="liquid-glass relative flex h-9 w-9 items-center justify-center rounded-full">
        <Bell size={16} />
        {unread > 0 && (
          <span className="absolute -right-1 -top-1 flex h-4 min-w-4 items-center justify-center rounded-full bg-red-500 px-1 text-[10px] font-bold text-white">{unread}</span>
        )}
      </motion.button>

      <AnimatePresence>
        {open && (
          <motion.div initial={{ opacity: 0, y: -8, scale: 0.96 }} animate={{ opacity: 1, y: 0, scale: 1 }} exit={{ opacity: 0, y: -8, scale: 0.96 }}
            transition={{ type: "spring", stiffness: 380, damping: 28 }}
            className="liquid-glass absolute right-0 top-full z-50 mt-2 w-80 rounded-[24px] p-4 text-sm">
            <div className="font-semibold">Thông báo</div>
            <div className="mt-2 max-h-64 space-y-3 overflow-y-auto">
              {items.map((i) => (
                <div key={i.id}>
                  <div className="font-medium">{i.title}</div>
                  <div className="text-xs" style={{ color: "var(--text-dim)" }}>{new Date(i.date).toLocaleString()}</div>
                  <div className="mt-0.5 text-xs">{i.body}</div>
                </div>
              ))}
              {!items.length && (
                <div className="text-xs" style={{ color: "var(--text-dim)" }}>
                  {err ? "Chưa kết nối được trang web thông báo. Kiểm tra WEB_URL trong src/config.ts." : "Chưa có thông báo."}
                </div>
              )}
            </div>
            <button onClick={() => openUrl(WEB_URL)} className="mt-3 flex w-full items-center justify-center gap-2 rounded-2xl bg-sky-500 py-2 text-xs font-medium text-white">
              <ExternalLink size={14} /> Mở trang web thông báo
            </button>
          </motion.div>
        )}
      </AnimatePresence>
    </div>
  );
}
