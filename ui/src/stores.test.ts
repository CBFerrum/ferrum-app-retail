import { describe, expect, it } from "vitest";
import { collections } from "./Home";

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
