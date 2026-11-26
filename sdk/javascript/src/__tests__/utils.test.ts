import { NebulaClient, ModularClient, parseNbl, formatNbl } from "../index";

describe("SDK utilities and exports", () => {
  it("exports NebulaClient as an alias for ModularClient", () => {
    expect(NebulaClient).toBe(ModularClient);
  });

  it("converts NBL to wei and back without precision loss", () => {
    expect(parseNbl("1")).toBe("1000000000000000000");
    expect(parseNbl("1.5")).toBe("1500000000000000000");
    expect(parseNbl(0.001)).toBe("1000000000000000");
    expect(formatNbl("1500000000000000000")).toBe("1.5");
    expect(formatNbl("1000000000000000000")).toBe("1");
  });

  it("rejects invalid NBL amounts", () => {
    expect(() => parseNbl("-1")).toThrow();
    expect(() => parseNbl("abc")).toThrow();
  });
});
