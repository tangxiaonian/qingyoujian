import { describe, expect, it } from "vitest";
import { errorText, moveIds, type Favorite } from "./types";
describe("favorites UI contract", () => {
  const items: Favorite[] = ["a", "b", "c"].map((id, sortOrder) => ({
    id,
    displayName: id,
    path: `/tmp/${id}`,
    sortOrder,
  }));
  it("moves one item without modifying source records or creating duplicates", () => {
    expect(moveIds(items, "b", -1)).toEqual(["b", "a", "c"]);
    expect(items.map((i) => i.id)).toEqual(["a", "b", "c"]);
    expect(moveIds(items, "a", -1)).toEqual(["a", "b", "c"]);
  });
  it("surfaces structured native errors", () => {
    expect(errorText({ code: "NOT_FOUND", message: "目录已移动" })).toBe(
      "目录已移动",
    );
  });
});
