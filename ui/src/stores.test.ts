import { describe, expect, it } from "vitest";
import { collections, transfers } from "./Home";
import { reportable } from "./Portal";

describe("the stores", () => {
  it("splits collections into those to get ready and those to hand over", () => {
    const c = collections([
      { id: "a", status: "Ordered" },
      { id: "b", status: "Ready" },
      { id: "c", status: "Collected" },
    ]);
    expect(c.ordered.map((x) => x.id)).toEqual(["a"]);
    expect(c.ready.map((x) => x.id)).toEqual(["b"]);
  });
});

describe("transfers and franchise reports", () => {
  it("splits transfers into those asked for and those on the way", () => {
    const t = transfers([
      { id: "a", status: "Requested" },
      { id: "b", status: "Sent" },
      { id: "c", status: "Received" },
      { id: "d", status: "Part Received" },
      { id: "e", status: "Closed Short" },
    ]);
    expect([t.asked.map((x) => x.id), t.onTheWay.map((x) => x.id)]).toEqual([["a"], ["b", "d"]]);
  });

  it("offers reports only under agreements begun", () => {
    const a = [
      { id: "now", starts_on: "2026-01-01", ends_on: "2030-12-31", reports: [] },
      { id: "later", starts_on: "2027-01-01", ends_on: "2031-12-31", reports: [] },
      { id: "over", starts_on: "2020-01-01", ends_on: "2025-12-31", reports: [] },
    ];
    expect(reportable(a, "2026-11-06").map((x) => x.id)).toEqual(["now", "over"]);
  });
});
