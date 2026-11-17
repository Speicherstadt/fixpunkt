/**
 * 3D-Ansicht der Baustelle (three.js, nur bei Bedarf geladen): Geländenetz aus dem Höhenraster (Farben wie die
 * 2D-Ansicht, Licht aus Nordwesten) oder die Punktwolke selbst (Farben aus der Datei oder nach Höhe). Maus: ziehen dreht, rechts ziehen verschiebt, Rad zoomt. Tastatur: Pfeile
 * verschieben die Ansicht. Ohne WebGL ein Hinweis.
 */
import { useEffect, useRef, useState } from "react";
import {
  AmbientLight,
  BufferAttribute,
  BufferGeometry,
  Color,
  DirectionalLight,
  Mesh,
  MeshLambertMaterial,
  PerspectiveCamera,
  Points,
  PointsMaterial,
  Scene,
  WebGLRenderer,
} from "three";
import { OrbitControls } from "three/examples/jsm/controls/OrbitControls.js";
import type { TerrainMesh } from "@/lib/view/mesh";
import type { PointCloudScene } from "@/lib/view/points";

export type Content =
  | { kind: "mesh"; mesh: TerrainMesh }
  | { kind: "points"; cloud: PointCloudScene };

import { m } from "@/paraglide/messages";

const cssVar = (name: string) =>
  getComputedStyle(document.documentElement).getPropertyValue(name).trim();

export default function Terrain3D({ content, label }: { content: Content; label: string }) {
  const wrap = useRef<HTMLDivElement>(null);
  const [failed, setFailed] = useState(false);

  useEffect(() => {
    const el = wrap.current;
    if (!el) return;
    let renderer: WebGLRenderer;
    try {
      renderer = new WebGLRenderer({ antialias: true, preserveDrawingBuffer: true });
    } catch {
      setFailed(true);
      return;
    }
    const width = el.clientWidth;
    const height = Math.max(320, Math.min(640, width * 0.6));
    renderer.setPixelRatio(Math.min(window.devicePixelRatio, 2));
    renderer.setSize(width, height);
    renderer.domElement.setAttribute("aria-hidden", "true");
    el.appendChild(renderer.domElement);

    const scene = new Scene();
    scene.background = new Color(cssVar("--subtle"));
    const data = content.kind === "mesh" ? content.mesh : content.cloud;
    const geometry = new BufferGeometry();
    geometry.setAttribute("position", new BufferAttribute(data.positions, 3));
    geometry.setAttribute("color", new BufferAttribute(data.colors, 3));
    let material: MeshLambertMaterial | PointsMaterial;
    if (content.kind === "mesh") {
      geometry.setIndex(new BufferAttribute(content.mesh.indices, 1));
      geometry.computeVertexNormals();
      material = new MeshLambertMaterial({ vertexColors: true });
      scene.add(new Mesh(geometry, material));
    } else {
      // Punkte immer 2 Pixel groß, unabhängig vom Abstand
      material = new PointsMaterial({ size: 2, sizeAttenuation: false, vertexColors: true });
      scene.add(new Points(geometry, material));
    }
    scene.add(new AmbientLight(0xffffff, 1.2));
    const sun = new DirectionalLight(0xffffff, 2);
    const [w, d, h] = data.size;
    const span = Math.max(w, d, h, 1);
    sun.position.set(-span, span, -span); // Nordwesten, wie die Schummerung in 2D
    scene.add(sun);

    const camera = new PerspectiveCamera(45, width / height, span / 1000, span * 20);
    camera.position.set(span * 0.7, span * 0.8 + h, span * 1.0); // von Südosten schräg oben
    const controls = new OrbitControls(camera, renderer.domElement);
    controls.target.set(0, h / 2, 0);
    controls.maxPolarAngle = Math.PI / 2 - 0.02;
    controls.listenToKeyEvents(el);
    controls.update();

    let frame = 0;
    const draw = () => {
      frame = requestAnimationFrame(draw);
      controls.update();
      renderer.render(scene, camera);
    };
    draw();
    const resize = new ResizeObserver(() => {
      const nw = el.clientWidth;
      const nh = Math.max(320, Math.min(640, nw * 0.6));
      renderer.setSize(nw, nh);
      camera.aspect = nw / nh;
      camera.updateProjectionMatrix();
    });
    resize.observe(el);
    return () => {
      cancelAnimationFrame(frame);
      resize.disconnect();
      controls.dispose();
      geometry.dispose();
      material.dispose();
      renderer.dispose();
      renderer.domElement.remove();
    };
  }, [content]);

  if (failed) return <p className="m-0 text-[13px] text-muted">{m.view3d_no_webgl()}</p>;
  return (
    <div
      ref={wrap}
      // biome-ignore lint/a11y/noNoninteractiveTabindex: die Ansicht lässt sich mit den Pfeiltasten bewegen
      tabIndex={0}
      role="application"
      aria-label={label}
      aria-roledescription={m.view3d_role()}
      data-triangles={content.kind === "mesh" ? content.mesh.triangles : undefined}
      data-points={content.kind === "points" ? content.cloud.positions.length / 3 : undefined}
      className="w-full overflow-hidden rounded-lg border border-border bg-subtle"
    />
  );
}
