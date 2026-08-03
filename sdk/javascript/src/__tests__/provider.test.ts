import { HttpProvider } from "../providers/http";

describe("HttpProvider route mapping", () => {
  const provider = new HttpProvider("http://localhost:8545");

  it("maps governance to node paths", () => {
    const map = (provider as any).mapMethodToEndpoint.bind(provider);
    expect(map("get_proposals").path).toBe("/governance");
    expect(map("get_proposal").path).toBe("/proposal/");
    expect(map("get_validators").path).toBe("/validators");
    expect(map("get_delegations").path).toBe("/delegations/");
  });

  it("maps WASM and faucet routes", () => {
    const map = (provider as any).mapMethodToEndpoint.bind(provider);
    expect(map("deploy_wasm").path).toBe("/deploy_wasm");
    expect(map("call_wasm").path).toBe("/call_wasm");
    expect(map("faucet_request").path).toBe("/faucet/request");
    expect(map("list_wasm_contracts").path).toBe("/wasm/contracts");
  });

  it("builds tx history URL with limit query", () => {
    const build = (provider as any).buildGetUrl.bind(provider);
    const url = build("/txs/", ["0xabc", 25], "get_tx_history");
    expect(url).toBe("/txs/0xabc?limit=25");
  });
});
