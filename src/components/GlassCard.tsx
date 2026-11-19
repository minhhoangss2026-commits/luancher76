import { motion, HTMLMotionProps } from "framer-motion";
import clsx from "clsx";

export default function GlassCard({ className, children, ...rest }: HTMLMotionProps<"div">) {
  return (
    <motion.div className={clsx("liquid-glass rounded-squircle", className)} {...rest}>
      {children}
    </motion.div>
  );
}
