import { useEffect } from "react";
import { motion } from "framer-motion";
import { IS_BETA } from "../../components/BetaBadge";

interface Props {
  onStart: () => void; // kích hoạt chuyển cảnh Warp
}

export default function WelcomeSplash({ onStart }: Props) {
  // Tự động bắt đầu sau 1.5 giây nếu người dùng chưa nhấn
  useEffect(() => {
    const t = setTimeout(onStart, 1500);
    return () => clearTimeout(t);
  }, [onStart]);

  return (
    <motion.div
      className="fixed inset-0 z-50 flex flex-col items-center justify-center"
      initial={{ opacity: 0 }}
      animate={{ opacity: 1 }}
      exit={{ opacity: 0, scale: 1.15, filter: "blur(12px)" }}
      transition={{ duration: 0.6 }}
    >
      {/* Logo phát sáng phong cách Apple */}
      <motion.div
        className="relative flex h-32 w-32 items-center justify-center rounded-[34px] liquid-glass"
        initial={{ scale: 0.6, opacity: 0 }}
        animate={{ scale: 1, opacity: 1 }}
        transition={{ type: "spring", stiffness: 140, damping: 14 }}
      >
        <motion.div
          className="absolute inset-0 rounded-[34px]"
          style={{ boxShadow: "0 0 80px 18px rgba(90,200,250,0.55)" }}
          animate={{ opacity: [0.5, 1, 0.5] }}
          transition={{ duration: 2.2, repeat: Infinity }}
        />
        <span className="text-6xl font-semibold bg-gradient-to-br from-white to-sky-300 bg-clip-text text-transparent">
          ◈
        </span>
      </motion.div>

      <motion.h1
        className="mt-8 text-4xl font-semibold tracking-tight"
        initial={{ y: 12, opacity: 0 }}
        animate={{ y: 0, opacity: 1 }}
        transition={{ delay: 0.25 }}
      >
        Cosmic Launcher
        {IS_BETA && (
          <span className="ml-3 align-middle rounded-full bg-gradient-to-r from-amber-400 to-orange-500 px-2.5 py-0.5 text-xs font-bold tracking-widest text-black">
            BETA
          </span>
        )}
      </motion.h1>
      <motion.p
        className="mt-2 text-sm"
        style={{ color: "var(--text-dim)" }}
        initial={{ opacity: 0 }}
        animate={{ opacity: 1 }}
        transition={{ delay: 0.45 }}
      >
        Minecraft · FPS Boost · AI Auto-Fix
      </motion.p>

      <motion.button
        onClick={onStart}
        whileTap={{ scale: 0.95 }}
        whileHover={{ scale: 1.04 }}
        className="liquid-glass mt-10 rounded-full px-8 py-3 text-base font-medium"
        initial={{ opacity: 0 }}
        animate={{ opacity: 1 }}
        transition={{ delay: 0.6 }}
      >
        Bắt đầu
      </motion.button>
    </motion.div>
  );
}
