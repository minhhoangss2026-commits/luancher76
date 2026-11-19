import { useState } from "react";
import { AnimatePresence, motion } from "framer-motion";

// Đổi thành false khi launcher đủ tính năng để ẩn nhãn BETA
export const IS_BETA = true;
export const APP_VERSION = "0.3.0";

const done = ["Giao diện Liquid Glass + Cosmic Warp", "Tài khoản Khách / Offline", "Chạy Minecraft Vanilla, Fabric, Quilt, Forge, NeoForge (PC)", "Tự tải Java từ Adoptium", "FPS Boost + preset đồ họa", "Kho Mod Modrinth 1-click", "AI Auto-Fixer + trợ lý chat", "Xem skin/áo choàng 3D", "Thông báo từ trang web", "Đăng nhập Microsoft", "CurseForge", "Skin trong game (Offline/Khách)", "Quản lý Java + thư mục dữ liệu"];
const todo = ["Áo choàng trong game", "Chạy game trên Android / iOS", "Kiểm thử trên nhiều máy"];

export default function BetaBadge({ small = false }: { small?: boolean }) {
  const [open, setOpen] = useState(false);
  if (!IS_BETA) return null;

  return (
    <div className="relative">
      <motion.button
        onClick={() => setOpen((o) => !o)}
        whileTap={{ scale: 0.94 }}
        className={`rounded-full bg-gradient-to-r from-amber-400 to-orange-500 font-bold tracking-widest text-black shadow-lg ${
          small ? "px-2.5 py-0.5 text-[10px]" : "px-3 py-1 text-xs"
        }`}
        aria-label="Phiên bản BETA"
      >
        BETA
      </motion.button>

      <AnimatePresence>
        {open && (
          <motion.div
            initial={{ opacity: 0, y: -8, scale: 0.96 }}
            animate={{ opacity: 1, y: 0, scale: 1 }}
            exit={{ opacity: 0, y: -8, scale: 0.96 }}
            transition={{ type: "spring", stiffness: 380, damping: 28 }}
            className="liquid-glass absolute right-0 top-full z-50 mt-2 w-72 rounded-[24px] p-4 text-left text-sm"
          >
            <div className="font-semibold">Cosmic Launcher {APP_VERSION} · BETA</div>
            <p className="mt-1 text-xs" style={{ color: "var(--text-dim)" }}>
              Bản thử nghiệm, có thể còn lỗi. Nhãn này sẽ biến mất khi đủ tính năng.
            </p>
            <div className="mt-3 text-xs font-semibold">Đã có</div>
            <ul className="mt-1 space-y-1 text-xs">
              {done.map((t) => <li key={t}>✅ {t}</li>)}
            </ul>
            <div className="mt-3 text-xs font-semibold">Sắp có</div>
            <ul className="mt-1 space-y-1 text-xs" style={{ color: "var(--text-dim)" }}>
              {todo.map((t) => <li key={t}>🚧 {t}</li>)}
            </ul>
          </motion.div>
        )}
      </AnimatePresence>
    </div>
  );
}
