const NBL_DECIMALS = 18;
const TEN = BigInt(10);
const WEI_PER_NBL = TEN ** BigInt(NBL_DECIMALS);

/**
 * Convert a human-readable NBL amount (e.g. "1.5") into a base-unit wei string.
 */
export function parseNbl(amount: string | number): string {
  const raw = String(amount).trim();
  if (!/^\d+(\.\d+)?$/.test(raw)) {
    throw new Error(`Invalid NBL amount: "${amount}"`);
  }
  const [wholePart, fracPart = ""] = raw.split(".");
  if (fracPart.length > NBL_DECIMALS) {
    throw new Error(`Too many decimal places (max ${NBL_DECIMALS}): "${amount}"`);
  }
  const paddedFrac = fracPart.padEnd(NBL_DECIMALS, "0");
  return (BigInt(wholePart) * WEI_PER_NBL + BigInt(paddedFrac)).toString();
}

/**
 * Convert a base-unit wei string or bigint into a human-readable NBL string.
 */
export function formatNbl(wei: string | bigint, maxDecimals: number = 6): string {
  const value = typeof wei === "bigint" ? wei : BigInt(wei);
  const whole = value / WEI_PER_NBL;
  const rem = value % WEI_PER_NBL;
  if (rem === BigInt(0) || maxDecimals <= 0) {
    return whole.toString();
  }
  const frac = rem
    .toString()
    .padStart(NBL_DECIMALS, "0")
    .slice(0, maxDecimals)
    .replace(/0+$/, "");
  return frac ? `${whole}.${frac}` : whole.toString();
}
