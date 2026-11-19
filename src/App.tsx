import { useCallback, useState } from "react";
import { AnimatePresence } from "framer-motion";
import WelcomeSplash from "./features/intro/WelcomeSplash";
import CosmicWarp from "./features/intro/CosmicWarp";
import Dashboard from "./features/dashboard/Dashboard";

type Phase = "splash" | "warp" | "main";

export default function App() {
  const [phase, setPhase] = useState<Phase>("splash");
  const [showDash, setShowDash] = useState(false);

  const startWarp = useCallback(() => setPhase((p) => (p === "splash" ? "warp" : p)), []);
  const reveal = useCallback(() => setShowDash(true), []);
  const done = useCallback(() => setPhase("main"), []);

  return (
    <div className="h-full w-full bg-black">
      {/* Dashboard mount sớm (lúc Warp tan biến) để lộ mượt */}
      {showDash && <Dashboard />}

      <AnimatePresence>
        {phase === "splash" && <WelcomeSplash key="splash" onStart={startWarp} />}
      </AnimatePresence>

      {phase === "warp" && <CosmicWarp onReveal={reveal} onDone={done} />}
    </div>
  );
}
