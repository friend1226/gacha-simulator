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
  total: number;
}

export function probabilityOf(cell: ResultCell): number {
  return cell.probability ?? cell.interval?.estimate ?? 0;
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
  const values = new Map<string, number>();
  let total = 0;
  for (const cell of cells) {
    if (axes.some((axis, index) =>
      axis.role === "filter" && cell.counts[index] !== axis.filterValue)) continue;
    const row = rowIndex >= 0 ? cell.counts[rowIndex] : 0;
    const col = colIndex >= 0 ? cell.counts[colIndex] : 0;
    const probability = probabilityOf(cell);
    const key = `${row}\u0000${col}`;
    values.set(key, (values.get(key) ?? 0) + probability);
    total += probability;
  }
  const rows = [...new Set([...values.keys()].map((key) => Number(key.split("\u0000")[0])))].sort((a, b) => a - b);
  const cols = [...new Set([...values.keys()].map((key) => Number(key.split("\u0000")[1])))].sort((a, b) => a - b);
  const resultCells = [...values].map(([key, probability]) => {
    const [row, col] = key.split("\u0000").map(Number);
    return { row, col, probability, display: probability.toExponential(11) };
  });
  return {
    rowAxis: rowIndex >= 0 ? axes[rowIndex] : undefined,
    colAxis: colIndex >= 0 ? axes[colIndex] : undefined,
    rows,
    cols,
    cells: resultCells,
    total,
  };
}

export function toCsv(table: PivotTable): string {
  const lookup = new Map(table.cells.map((cell) => [`${cell.row}\u0000${cell.col}`, cell.display]));
  const header = [table.rowAxis?.label ?? "값", ...table.cols.map(String)];
  return [
    header.join(","),
    ...table.rows.map((row) => [
      row,
      ...table.cols.map((col) => lookup.get(`${row}\u0000${col}`) ?? "0"),
    ].join(",")),
  ].join("\n");
}
