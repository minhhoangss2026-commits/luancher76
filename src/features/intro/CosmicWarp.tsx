import { useEffect, useRef } from "react";
import * as THREE from "three";

interface Props {
  onReveal?: () => void; // gọi khi bắt đầu tan biến (hiện Dashboard bên dưới)
  onDone: () => void;    // gọi sau khi đã giải phóng GPU
  duration?: number;     // tổng thời gian (ms)
}

const STAR_COUNT = 6000;
const DEPTH = 1800;

export default function CosmicWarp({ onReveal, onDone, duration = 3600 }: Props) {
  const mountRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const mount = mountRef.current!;
    const isSmall = window.innerWidth < 900;
    const count = isSmall ? 3200 : STAR_COUNT; // giảm tải cho thiết bị di động

    // ---------- Scene / Camera / Renderer ----------
    const scene = new THREE.Scene();
    const camera = new THREE.PerspectiveCamera(70, window.innerWidth / window.innerHeight, 1, DEPTH * 2);
    camera.position.z = 0;

    const renderer = new THREE.WebGLRenderer({ antialias: false, alpha: true, powerPreference: "high-performance" });
    renderer.setPixelRatio(Math.min(window.devicePixelRatio, 2));
    renderer.setSize(window.innerWidth, window.innerHeight);
    renderer.setClearColor(0x000000, 1);
    mount.appendChild(renderer.domElement);

    // ---------- Vệt sao: mỗi sao = 1 đoạn thẳng (đuôi -> đầu) ----------
    const positions = new Float32Array(count * 6);
    const colors = new Float32Array(count * 6);
    const base = new Float32Array(count * 3);
    const palette = [new THREE.Color("#9fd8ff"), new THREE.Color("#ffffff"), new THREE.Color("#c9a7ff"), new THREE.Color("#ffd6a5")];

    for (let i = 0; i < count; i++) {
      const r = 20 + Math.random() * 900;
      const a = Math.random() * Math.PI * 2;
      base[i * 3] = Math.cos(a) * r;
      base[i * 3 + 1] = Math.sin(a) * r;
      base[i * 3 + 2] = -Math.random() * DEPTH;
      const c = palette[(Math.random() * palette.length) | 0];
      for (let k = 0; k < 2; k++) {
        const o = i * 6 + k * 3;
        colors[o] = c.r; colors[o + 1] = c.g; colors[o + 2] = c.b;
      }
    }

    const geo = new THREE.BufferGeometry();
    geo.setAttribute("position", new THREE.BufferAttribute(positions, 3));
    geo.setAttribute("color", new THREE.BufferAttribute(colors, 3));
    const mat = new THREE.LineBasicMaterial({
      vertexColors: true, transparent: true, opacity: 1,
      blending: THREE.AdditiveBlending, depthWrite: false,
    });
    const streaks = new THREE.LineSegments(geo, mat);
    streaks.frustumCulled = false;
    scene.add(streaks);

    // ---------- Tinh vân: sprite gradient lớn, mờ ----------
    const nebulaTex = makeNebulaTexture();
    const nebulae: THREE.Sprite[] = [];
    const nebulaColors = ["#3a6bff", "#a64bff", "#00c2ff", "#ff4bd8"];
    for (let i = 0; i < 6; i++) {
      const sm = new THREE.SpriteMaterial({
        map: nebulaTex, color: nebulaColors[i % 4], transparent: true, opacity: 0.35,
        blending: THREE.AdditiveBlending, depthWrite: false,
      });
      const s = new THREE.Sprite(sm);
      s.scale.set(900, 900, 1);
      s.position.set((Math.random() - 0.5) * 900, (Math.random() - 0.5) * 600, -Math.random() * DEPTH);
      scene.add(s);
      nebulae.push(s);
    }

    // ---------- Vòng lặp animation ----------
    let raf = 0;
    let revealed = false;
    let cleaned = false;
    const t0 = performance.now();
    let last = t0;

    const ease = (x: number) => x * x * (3 - 2 * x); // smoothstep

    // ---------- Giải phóng toàn bộ tài nguyên GPU ----------
    const cleanup = () => {
      if (cleaned) return;
      cleaned = true;
      cancelAnimationFrame(raf);
      window.removeEventListener("resize", onResize);
      geo.dispose();
      mat.dispose();
      nebulae.forEach((s) => (s.material as THREE.SpriteMaterial).dispose());
      nebulaTex.dispose();
      renderer.dispose();
      renderer.forceContextLoss(); // trả GPU context ngay
      if (renderer.domElement.parentNode) renderer.domElement.parentNode.removeChild(renderer.domElement);
    };

    const tick = (now: number) => {
      const t = now - t0;
      const dt = Math.min(now - last, 50) / 16.67; // độc lập tần số quét 60-120Hz
      last = now;
      const p = Math.min(t / duration, 1);

      // Pha 1 (0-0.55): tăng tốc; Pha 2 (0.55-0.8): tốc độ ánh sáng + zoom; Pha 3 (0.8-1): tan biến
      const speed = 6 + ease(Math.min(p / 0.55, 1)) * 90 + (p > 0.55 ? ease((p - 0.55) / 0.25) * 140 : 0);
      const tail = 4 + speed * 2.2; // độ dài vệt tỷ lệ vận tốc

      for (let i = 0; i < count; i++) {
        let z = base[i * 3 + 2] + speed * dt;
        if (z > 0) z -= DEPTH;
        base[i * 3 + 2] = z;
        const x = base[i * 3], y = base[i * 3 + 1];
        const o = i * 6;
        positions[o] = x; positions[o + 1] = y; positions[o + 2] = z;
        positions[o + 3] = x; positions[o + 4] = y; positions[o + 5] = z - tail;
      }
      geo.attributes.position.needsUpdate = true;

      const fade = 1 - Math.max(0, (p - 0.8) / 0.2);
      nebulae.forEach((s) => {
        s.position.z += speed * 0.35 * dt;
        if (s.position.z > 200) s.position.z = -DEPTH;
        (s.material as THREE.SpriteMaterial).opacity = 0.35 * fade;
      });

      // Zoom FOV (hiệu ứng hút vào) rồi nở ra khi tan biến
      camera.fov = 70 + (p > 0.55 ? ease((p - 0.55) / 0.45) * 55 : 0);
      camera.updateProjectionMatrix();

      // Tan biến: mờ dần canvas -> lộ Dashboard bên dưới
      if (p > 0.8) {
        if (!revealed) { revealed = true; onReveal?.(); }
        const f = Math.max(fade, 0);
        mat.opacity = f;
        renderer.domElement.style.opacity = String(f);
        renderer.domElement.style.transform = `scale(${1 + (1 - f) * 0.25})`;
      }

      renderer.render(scene, camera);

      if (p < 1) {
        raf = requestAnimationFrame(tick);
      } else {
        cleanup();
        onDone();
      }
    };

    const onResize = () => {
      camera.aspect = window.innerWidth / window.innerHeight;
      camera.updateProjectionMatrix();
      renderer.setSize(window.innerWidth, window.innerHeight);
    };
    window.addEventListener("resize", onResize);
    raf = requestAnimationFrame(tick);

    return cleanup;
  }, [onDone, onReveal, duration]);

  return <div ref={mountRef} className="fixed inset-0 z-[60]" style={{ pointerEvents: "none" }} />;
}

// Tạo texture tinh vân bằng Canvas 2D (không cần tải ảnh ngoài)
function makeNebulaTexture(): THREE.CanvasTexture {
  const c = document.createElement("canvas");
  c.width = c.height = 256;
  const g = c.getContext("2d")!;
  const grad = g.createRadialGradient(128, 128, 0, 128, 128, 128);
  grad.addColorStop(0, "rgba(255,255,255,0.9)");
  grad.addColorStop(0.35, "rgba(255,255,255,0.25)");
  grad.addColorStop(1, "rgba(255,255,255,0)");
  g.fillStyle = grad;
  g.fillRect(0, 0, 256, 256);
  return new THREE.CanvasTexture(c);
}
