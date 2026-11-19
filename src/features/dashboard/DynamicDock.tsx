import { motion } from "framer-motion";
import { Gamepad2, Zap, Package, Shirt, Sparkles, Settings } from "lucide-react";
import { useStore } from "../../lib/store";
import type { Tab } from "../../lib/store";

const items: { id: Tab; label: string; icon: JSX.Element }[] = [
  { id: "play", label: "Chơi", icon: <Gamepad2 size={22} /> },
  { id: "boost", label: "FPS Boost", icon: <Zap size={22} /> },
  { id: "mods", label: "Kho Mod", icon: <Package size={22} /> },
  { id: "wardrobe", label: "Trang phục", icon: <Shirt size={22} /> },
  { id: "ai", label: "Trợ lý AI", icon: <Sparkles size={22} /> },
  { id: "settings", label: "Cài đặt", icon: <Settings size={22} /> },
];

export default function DynamicDock() {
  const { tab, setTab } = useStore();

  return (
    <motion.nav
      initial={{ y: 80, opacity: 0 }}
      animate={{ y: 0, opacity: 1 }}
      transition={{ type: "spring", stiffness: 220, damping: 22, delay: 0.1 }}
      className="liquid-glass fixed left-1/2 z-30 flex -translate-x-1/2 gap-1 rounded-[32px] p-2"
      style={{ bottom: "calc(var(--safe-bottom) + 16px)" }}
    >
      {items.map((it) => {
        const active = tab === it.id;
        return (
          <motion.button
            key={it.id}
            onClick={() => setTab(it.id)}
            whileHover={{ scale: 1.12, y: -4 }}
            whileTap={{ scale: 0.92 }}
            transition={{ type: "spring", stiffness: 400, damping: 18 }}
            className="relative flex h-14 w-14 flex-col items-center justify-center rounded-[22px]"
            aria-label={it.label}
            title={it.label}
          >
            {active && (
              <motion.div
                layoutId="dock-pill" // viên thuốc trượt mượt giữa các tab
                className="absolute inset-0 rounded-[22px] bg-white/25"
                style={{ boxShadow: "inset 0 1px 0 rgba(255,255,255,0.6)" }}
                transition={{ type: "spring", stiffness: 380, damping: 30 }}
              />
            )}
            <span className="relative">{it.icon}</span>
            {active && <span className="relative mt-0.5 text-[9px] font-medium">{it.label}</span>}
          </motion.button>
        );
      })}
    </motion.nav>
  );
}
