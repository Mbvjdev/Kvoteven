import { describe, it, expect } from "vitest";
import { petSvg } from "./pet";

function rectCount(svg: string): number {
  return (svg.match(/<rect /g) ?? []).length;
}

describe("petSvg", () => {
  it("renders the happy pet with the brand green body and open eyes", () => {
    const svg = petSvg("happy");
    expect(svg).toContain("#9bd4a6");
    expect(svg).toContain('x="8" y="12" width="2" height="3"'); // open eyes
    expect(svg).toContain('shape-rendering="crispEdges"');
  });

  it("uses a distinct body color per mood", () => {
    expect(petSvg("steady")).toContain("#b8d19e");
    expect(petSvg("sleepy")).toContain("#e3cc8f");
    expect(petSvg("asleep")).toContain("#bfbdd6");
    expect(petSvg("unknown")).toContain("#c2c7c4");
  });

  it("draws closed eyes and a mouth for the sleeping moods", () => {
    const asleep = petSvg("asleep");
    expect(asleep).toContain('x="7" y="14" width="4" height="1"'); // closed eyes
    expect(asleep).not.toContain('x="8" y="12" width="2" height="3"'); // no open eyes
    const sleepy = petSvg("sleepy");
    expect(sleepy).toContain('x="8" y="13" width="2" height="1"'); // half lids
  });

  it("produces the expected box counts for each mood", () => {
    expect(rectCount(petSvg("happy"))).toBe(24);
    expect(rectCount(petSvg("steady"))).toBe(24);
    expect(rectCount(petSvg("sleepy"))).toBe(24);
    expect(rectCount(petSvg("asleep"))).toBe(22);
    expect(rectCount(petSvg("unknown"))).toBe(22);
  });
});
