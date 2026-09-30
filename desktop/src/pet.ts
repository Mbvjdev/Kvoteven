import type { Mood } from "./types";

/**
 * The original pixel pet, ported 1:1 from PetSprite.swift onto a 24-unit SVG
 * grid. `shape-rendering="crispEdges"` keeps every box sharp. Colors match the
 * documented brand art (docs/assets/hero.svg).
 */

const INK = "#30473b";

const BODY: Record<Mood, string> = {
  happy: "#9bd4a6",
  steady: "#b8d19e",
  sleepy: "#e3cc8f",
  asleep: "#bfbdd6",
  unknown: "#c2c7c4",
};

const BLUSH = "#e69c8f";

interface Box {
  x: number;
  y: number;
  w: number;
  h: number;
  fill: string;
  opacity?: number;
}

function box(x: number, y: number, w: number, h: number, fill: string, opacity?: number): Box {
  return { x, y, w, h, fill, opacity };
}

function boxes(mood: Mood): Box[] {
  const body = BODY[mood];
  const out: Box[] = [
    // ink outline
    box(5, 3, 3, 6, INK),
    box(16, 3, 3, 6, INK),
    box(6, 6, 12, 3, INK),
    box(4, 8, 16, 12, INK),
    box(3, 10, 18, 8, INK),
    box(6, 19, 12, 3, INK),
    box(6, 21, 4, 2, INK),
    box(14, 21, 4, 2, INK),
    box(20, 15, 3, 4, INK),
    // body inner fill
    box(6, 5, 1, 4, body),
    box(17, 5, 1, 4, body),
    box(7, 7, 10, 3, body),
    box(5, 9, 14, 10, body),
    box(4, 11, 16, 6, body),
    box(7, 19, 10, 2, body),
    box(20, 15, 1, 3, body),
    // highlight
    box(7, 9, 5, 1, "#ffffff", 0.42),
    // blush
    box(6, 16, 2, 1, BLUSH),
    box(16, 16, 2, 1, BLUSH),
  ];

  if (mood === "asleep" || mood === "sleepy") {
    out.push(box(7, 14, 4, 1, INK), box(13, 14, 4, 1, INK));
    if (mood === "sleepy") out.push(box(8, 13, 2, 1, INK), box(14, 13, 2, 1, INK));
    out.push(box(11, 17, 2, 1, INK));
  } else {
    out.push(box(8, 12, 2, 3, INK), box(14, 12, 2, 3, INK));
    if (mood === "unknown") {
      out.push(box(11, 17, 2, 1, INK));
    } else {
      out.push(box(10, 16, 1, 1, INK), box(13, 16, 1, 1, INK), box(11, 17, 2, 1, INK));
    }
  }
  return out;
}

/** SVG markup for the pet at the given mood. Self-generated, no external input. */
export function petSvg(mood: Mood): string {
  const rects = boxes(mood)
    .map(
      (b) =>
        `<rect x="${b.x}" y="${b.y}" width="${b.w}" height="${b.h}" fill="${b.fill}"` +
        (b.opacity !== undefined ? ` opacity="${b.opacity}"` : "") +
        `/>`,
    )
    .join("");
  return (
    `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" width="100%" height="100%" ` +
    `shape-rendering="crispEdges" focusable="false" aria-hidden="true">${rects}</svg>`
  );
}
