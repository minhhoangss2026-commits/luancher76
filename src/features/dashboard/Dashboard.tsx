import { useEffect } from "react";
import { motion } from "framer-motion";
import { useStore } from "../../lib/store";
import { listAccounts } from "../../lib/tauri";
import AccountSwitcher from "./AccountSwitcher";
import DynamicDock from "./DynamicDock";
import PlayPanel from "./PlayPanel";
import AIBubble from "../ai/AIBubble";
import AIPanel from "../ai/AIPanel";
import BoostPanel from "../optimize/BoostPanel";
import ModBrowser from "../mods/ModBrowser";
import Wardrobe from "../wardrobe/Wardrobe";
import SettingsPanel from "../settings/SettingsPanel";
import BetaBadge from "../../components/BetaBadge";
import NotificationBell from "../../components/NotificationBell";

export default function Dashboard() {
  const { tab, setAccounts } = useStore();

  // Rust tự tạo hồ sơ Khách (Player_XXXX) ở lần mở đầu tiên
  useEffect(() => { listAccounts().then(setAccounts); }, [setAccounts]);

  return (
    <motion.div className="relative h-full w-full" initial={{ opacity: 0, scale: 0.96 }} animate={{ opacity: 1, scale: 1 }}
      transition={{ type: "spring", stiffness: 90, damping: 18 }}>
      <header className="flex items-center justify-between px-5" style={{ paddingTop: "calc(var(--safe-top) + 16px)" }}>
        <AccountSwitcher />
        <div className="flex items-center gap-2">
          <NotificationBell />
          <div className="liquid-glass hidden rounded-full px-4 py-2 text-xs sm:block">Cosmic Launcher</div>
          <BetaBadge />
        </div>
      </header>

      <main className="flex h-[calc(100%-140px)] items-start justify-center overflow-y-auto px-5 pt-4 sm:items-center sm:pt-0">
        {tab === "play" && <PlayPanel />}
        {tab === "boost" && <BoostPanel />}
        {tab === "mods" && <ModBrowser />}
        {tab === "wardrobe" && <Wardrobe />}
        {tab === "ai" && <AIPanel />}
        {tab === "settings" && <SettingsPanel />}
      </main>

      <DynamicDock />
      <AIBubble />
    </motion.div>
  );
}
