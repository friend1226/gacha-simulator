import type { ResultCell } from "./types";

export type AxisRole = "row" | "col" | "sum" | "filter";
export interface Axis {
  key: string;
  label: string;
  role: AxisRole;
  filterValue?: number;
}

export interface PivotCell {
  row: number;
  col: number;
  probability: number;
  display: string;
}

export interface PivotTable {
  rowAxis?: Axis;
  colAxis?: Axis;
  rows: number[];
  cols: number[];
  cells: PivotCell[];
  lookup: Map<string, PivotCell>;
  total: number;
}

interface DecimalValue {
  coefficient: bigint;
  exponent: number;
}

export function pivotKey(row: number, col: number): string {
  return `${row}\u0000${col}`;
}

export function probabilityOf(cell: ResultCell): number {
  return cell.probability ?? cell.interval?.estimate ?? 0;
}

function parseDecimal(source: string): DecimalValue | undefined {
  const match = source.trim().match(/^([+-]?)(\d+)(?:\.(\d*))?(?:e([+-]?\d+))?$/i);
  if (!match) return undefined;
  const fraction = match[3] ?? "";
  const digits = `${match[2]}${fraction}`.replace(/^0+/, "") || "0";
  const scientificExponent = Number(match[4] ?? 0);
  if (!Number.isSafeInteger(scientificExponent)) return undefined;
  const sign = match[1] === "-" ? -1n : 1n;
  return normalizeDecimal({
    coefficient: sign * BigInt(digits),
    exponent: scientificExponent - fraction.length,
  });
}

function normalizeDecimal(value: DecimalValue): DecimalValue {
  if (value.coefficient === 0n) return { coefficient: 0n, exponent: 0 };
  let { coefficient, exponent } = value;
  while (coefficient % 10n === 0n) {
    coefficient /= 10n;
    exponent += 1;
  }
  return { coefficient, exponent };
}

function addDecimal(left: DecimalValue | undefined, right: DecimalValue): DecimalValue {
  if (!left || left.coefficient === 0n) return right;
  if (right.coefficient === 0n) return left;
  const exponent = Math.min(left.exponent, right.exponent);
  const leftScale = BigInt(left.exponent - exponent);
  const rightScale = BigInt(right.exponent - exponent);
  return normalizeDecimal({
    coefficient: left.coefficient * (10n ** leftScale)
      + right.coefficient * (10n ** rightScale),
    exponent,
  });
}

function decimalFromRatio(numeratorSource: string, denominatorSource: string, significantDigits = 12): DecimalValue | undefined {
  let numerator: bigint;
  let denominator: bigint;
  try {
    numerator = BigInt(numeratorSource);
    denominator = BigInt(denominatorSource);
  } catch {
    return undefined;
  }
  if (denominator === 0n) return undefined;
  if (numerator === 0n) return { coefficient: 0n, exponent: 0 };
  const negative = (numerator < 0n) !== (denominator < 0n);
  if (numerator < 0n) numerator = -numerator;
  if (denominator < 0n) denominator = -denominator;
  let exponent = numerator.toString().length - denominator.toString().length;
  const comparison = exponent >= 0
    ? numerator < denominator * (10n ** BigInt(exponent))
    : numerator * (10n ** BigInt(-exponent)) < denominator;
  if (comparison) exponent -= 1;
  const scale = significantDigits - 1 - exponent;
  const scaledNumerator = scale >= 0 ? numerator * (10n ** BigInt(scale)) : numerator;
  const scaledDenominator = scale >= 0 ? denominator : denominator * (10n ** BigInt(-scale));
  let coefficient = scaledNumerator / scaledDenominator;
  if ((scaledNumerator % scaledDenominator) * 2n >= scaledDenominator) coefficient += 1n;
  return normalizeDecimal({
    coefficient: negative ? -coefficient : coefficient,
    exponent: exponent - significantDigits + 1,
  });
}

function decimalForCell(cell: ResultCell): DecimalValue {
  const display = cell.display ? parseDecimal(cell.display) : undefined;
  if (display) return display;
  if (cell.numerator && cell.denominator) {
    const ratio = decimalFromRatio(cell.numerator, cell.denominator);
    if (ratio) return ratio;
  }
  return parseDecimal(probabilityOf(cell).toExponential(17)) ?? { coefficient: 0n, exponent: 0 };
}

function formatScientific(value: DecimalValue, significantDigits = 12): string {
  if (value.coefficient === 0n) return "0";
  const negative = value.coefficient < 0n;
  let digits = (negative ? -value.coefficient : value.coefficient).toString();
  let scientificExponent = value.exponent + digits.length - 1;
  if (digits.length > significantDigits) {
    const discarded = digits.length - significantDigits;
    const divisor = 10n ** BigInt(discarded);
    let rounded = BigInt(digits) / divisor;
    if ((BigInt(digits) % divisor) * 2n >= divisor) rounded += 1n;
    digits = rounded.toString();
    if (digits.length > significantDigits) {
      scientificExponent += 1;
      digits = digits.slice(0, significantDigits);
    }
  }
  digits = digits.padEnd(significantDigits, "0");
  const mantissa = significantDigits === 1 ? digits : `${digits[0]}.${digits.slice(1)}`;
  return `${negative ? "-" : ""}${mantissa}e${scientificExponent >= 0 ? "+" : ""}${scientificExponent}`;
}

export function defaultAxes(ids: string[]): Axis[] {
  return ids.map((key, index) => ({
    key,
    label: key,
    role: index === 0 ? "row" : index === 1 ? "col" : "sum",
  }));
}

export function pivot(cells: ResultCell[], axes: Axis[]): PivotTable {
  const rowIndex = axes.findIndex((axis) => axis.role === "row");
  const colIndex = axes.findIndex((axis) => axis.role === "col");
  const values = new Map<string, { probability: number; decimal?: DecimalValue }>();
  let total = 0;
  for (const cell of cells) {
    if (axes.some((axis, index) =>
      axis.role === "filter" && cell.counts[index] !== axis.filterValue)) continue;
    const row = rowIndex >= 0 ? cell.counts[rowIndex] : 0;
    const col = colIndex >= 0 ? cell.counts[colIndex] : 0;
    const probability = probabilityOf(cell);
    const key = pivotKey(row, col);
    const previous = values.get(key);
    values.set(key, {
      probability: (previous?.probability ?? 0) + probability,
      decimal: addDecimal(previous?.decimal, decimalForCell(cell)),
    });
    total += probability;
  }
  const rows = [...new Set([...values.keys()].map((key) => Number(key.split("\u0000")[0])))].sort((a, b) => a - b);
  const cols = [...new Set([...values.keys()].map((key) => Number(key.split("\u0000")[1])))].sort((a, b) => a - b);
  const resultCells = [...values].map(([key, value]) => {
    const [row, col] = key.split("\u0000").map(Number);
    return {
      row,
      col,
      probability: value.probability,
      display: formatScientific(value.decimal ?? { coefficient: 0n, exponent: 0 }),
    };
  });
  const lookup = new Map(resultCells.map((cell) => [pivotKey(cell.row, cell.col), cell]));
  return {
    rowAxis: rowIndex >= 0 ? axes[rowIndex] : undefined,
    colAxis: colIndex >= 0 ? axes[colIndex] : undefined,
    rows,
    cols,
    cells: resultCells,
    lookup,
    total,
  };
}

export function toCsv(table: PivotTable): string {
  const header = [table.rowAxis?.label ?? "값", ...table.cols.map(String)];
  return [
    header.join(","),
    ...table.rows.map((row) => [
      row,
      ...table.cols.map((col) => table.lookup.get(pivotKey(row, col))?.display ?? "0"),
    ].join(",")),
  ].join("\n");
}
