import { Copy, Download, Play, Square } from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import { defaultAxes, pivot, pivotKey, probabilityOf, toCsv, type Axis, type AxisRole, type PivotCell } from "../pivot";
import { engineLabels } from "../labels";
import type { EngineProgress } from "../engine";
import type { EngineErrorPresentation } from "../engineDiagnostics";
import { formatProbability, type AppSettings } from "../settings";
import type { EngineResult, ModelIr, ResultCell } from "../types";

export function ResultPanel({
  model,
  updateModel,
  settings,
  results,
  running,
  canCancel,
  progress,
  engineError,
  message,
  run,
  cancelRun,
  openHelp,
}: {
  model: ModelIr;
  updateModel: (model: ModelIr) => void;
  settings: AppSettings;
  results: { dp?: EngineResult; mc?: EngineResult };
  running?: "dp" | "mc";
  canCancel: boolean;
  progress?: EngineProgress;
  engineError?: EngineErrorPresentation;
  message: string;
  run: (engine: "dp" | "mc", runs: number, seed: number) => void;
  cancelRun: () => void;
  openHelp: (code: string) => void;
}) {
  const ids = results.dp?.trackedLeafIds ?? results.mc?.trackedLeafIds ?? model.run.trackJoint;
  const [axes, setAxes] = useState<Axis[]>(() => defaultAxes(ids));
  const [runs, setRuns] = useState(settings.mcRuns);
  const [seed, setSeed] = useState(settings.mcSeed);
  useEffect(() => setAxes(defaultAxes(ids)), [ids.join("\u0000")]);
  const candidates = useMemo(() => [
    ...collectEntityIds(model.entities),
    ...model.stateVars.filter((variable) => variable.role === "accumulator").map((variable) => variable.id),
  ], [model]);

  function updateRun(patch: Partial<ModelIr["run"]>) {
    updateModel({ ...model, run: { ...model.run, ...patch } });
  }

  function toggleTrack(id: string) {
    const selected = model.run.trackJoint.includes(id);
    updateRun({ trackJoint: selected ? model.run.trackJoint.filter((item) => item !== id) : [...model.run.trackJoint, id] });
  }

  return (
    <section className="results-page">
      <div className="run-toolbar">
        <label>시행 횟수<input type="number" min="1" value={model.run.maxTrials} onChange={(event) => updateRun({ maxTrials: Math.max(1, Number(event.target.value)) })} /></label>
        <label>수치 모드<select value={model.run.numeric} onChange={(event) => updateRun({ numeric: event.target.value as ModelIr["run"]["numeric"] })}><option value="scaled">표준</option><option value="f64">고속</option><option value="exact">정확</option></select></label>
        <label>시리즈<select value={model.run.trialSeries ?? "marginal"} onChange={(event) => updateRun({ trialSeries: event.target.value as ModelIr["run"]["trialSeries"] })}><option value="marginal">시행별 주변분포</option><option value="checkpoints">체크포인트 결합</option><option value="none">저장 안 함</option></select></label>
        {model.run.trialSeries === "checkpoints" && <label>체크포인트 (최대 20개)<input value={(model.run.seriesCheckpoints ?? []).join(",")} placeholder="10,50,100,200" onChange={(event) => updateRun({ seriesCheckpoints: event.target.value.split(",").map((value) => Number(value.trim())).filter((value) => Number.isInteger(value) && value > 0).slice(0, 20) })} /></label>}
        <label>반복 수<input type="number" min="1" value={runs} onChange={(event) => setRuns(Math.max(1, Number(event.target.value)))} /></label>
        <label>시드<input type="number" min="0" value={seed} onChange={(event) => setSeed(Math.max(0, Number(event.target.value)))} /></label>
        <div className="run-buttons">
          <button disabled={Boolean(running)} onClick={() => run("dp", runs, seed)}><Play size={15} /> 정확 계산</button>
          <button disabled={Boolean(running)} className="secondary" onClick={() => run("mc", runs, seed)}><Play size={15} /> 시뮬레이션</button>
          {running && canCancel && <button className="cancel" onClick={cancelRun}><Square size={14} /> 취소</button>}
        </div>
        <p>{message}</p>
        {running && progress && (
          <div className="run-progress" role="status">
            <progress value={progress.completed} max={progress.total} />
            <span>{progress.completed.toLocaleString()} / {progress.total.toLocaleString()} ({Math.floor(progress.completed / progress.total * 100)}%)</span>
          </div>
        )}
        {engineError && <EngineErrorNotice error={engineError} openHelp={openHelp} />}
      </div>
      <details className="track-picker">
        <summary>추적 대상 · {model.run.trackJoint.join(", ") || "선택 없음"}</summary>
        <div>{candidates.map((id) => <label key={id}><input type="checkbox" checked={model.run.trackJoint.includes(id)} onChange={() => toggleTrack(id)} />{id}</label>)}</div>
      </details>
      {(results.dp || results.mc) ? (
        <>
          <AxisControls axes={axes} cells={results.dp?.joint ?? results.mc?.joint ?? []} onChange={setAxes} />
          <div className="comparison-grid">
            {results.dp && <ResultView result={results.dp} axes={axes} settings={settings} />}
            {results.mc && <ResultView result={results.mc} axes={axes} settings={settings} />}
          </div>
          <SeriesComparison dp={results.dp} mc={results.mc} settings={settings} />
          {(results.dp?.firstHit || results.mc?.firstHit) && <FirstHitView result={results.dp ?? results.mc!} settings={settings} />}
        </>
      ) : <div className="empty-results"><h2>아직 계산 결과가 없습니다</h2><p>위 실행 바에서 계산 방식을 선택하세요.</p></div>}
    </section>
  );
}

function EngineErrorNotice({
  error,
  openHelp,
}: {
  error: EngineErrorPresentation;
  openHelp: (code: string) => void;
}) {
  return (
    <section className="engine-diagnostics" role="alert">
      <h2>계산을 실행하지 못했습니다</h2>
      <ul>
        {error.diagnostics.map((diagnostic, index) => (
          <li key={`${diagnostic.code}-${index}`}>
            <b>{diagnostic.code}{diagnostic.title ? ` · ${diagnostic.title}` : ""}</b>
            <span>{diagnostic.fix ?? diagnostic.original}</span>
            {diagnostic.title && <button type="button" onClick={() => openHelp(diagnostic.code)}>도움말에서 보기</button>}
          </li>
        ))}
      </ul>
      <details>
        <summary>영문 원문 자세히</summary>
        <pre>{error.original}</pre>
      </details>
    </section>
  );
}

function collectEntityIds(entities: ModelIr["entities"]): string[] {
  return entities.flatMap((entity) => [
    entity.id,
    ...(entity.children?.length ? collectEntityIds(entity.children) : []),
  ]);
}

function AxisControls({ axes, cells, onChange }: { axes: Axis[]; cells: ResultCell[]; onChange: (axes: Axis[]) => void }) {
  function setRole(index: number, role: AxisRole) {
    const next = axes.map((axis, axisIndex) => {
      if (axisIndex === index) return { ...axis, role };
      if ((role === "row" || role === "col") && axis.role === role) return { ...axis, role: "sum" as const };
      return axis;
    });
    onChange(next);
  }
  return (
    <div className="axis-controls">
      <span>피벗 축</span>
      {axes.map((axis, index) => {
        const values = [...new Set(cells.map((cell) => cell.counts[index]))].sort((a, b) => a - b);
        return <div className="axis-control" key={axis.key}><b>{axis.label}</b>
          <select value={axis.role} onChange={(event) => setRole(index, event.target.value as AxisRole)}>
            <option value="row">행</option><option value="col">열</option><option value="sum">집계</option><option value="filter">필터</option>
          </select>
          {axis.role === "filter" && <select value={axis.filterValue ?? values[0]} onChange={(event) => onChange(axes.map((item, itemIndex) => itemIndex === index ? { ...item, filterValue: Number(event.target.value) } : item))}>{values.map((value) => <option key={value}>{value}</option>)}</select>}
        </div>;
      })}
    </div>
  );
}

function ResultView({ result, axes, settings }: { result: EngineResult; axes: Axis[]; settings: AppSettings }) {
  const table = useMemo(() => pivot(result.joint, axes), [result, axes]);
  const [selected, setSelected] = useState<{ row: number; col: number }>();
  const max = table.cells.reduce((largest, cell) => Math.max(largest, cell.probability), 0);
  const visibleRows = table.rows.slice(0, settings.maxRows);
  const visibleCols = table.cols.slice(0, settings.maxRows);
  const hiddenRows = table.rows.length - visibleRows.length;
  const hiddenCols = table.cols.length - visibleCols.length;
  const title = engineLabels[result.engine === "Exact" ? "Exact" : result.engine];
  function download(name: string, contents: string, type: string) {
    const url = URL.createObjectURL(new Blob([contents], { type }));
    const anchor = document.createElement("a");
    anchor.href = url; anchor.download = name; anchor.click();
    URL.revokeObjectURL(url);
  }
  const selectedSources = selected ? result.joint.filter((source) => {
    const rowIndex = axes.findIndex((axis) => axis.role === "row");
    const colIndex = axes.findIndex((axis) => axis.role === "col");
    return (rowIndex < 0 || source.counts[rowIndex] === selected.row)
      && (colIndex < 0 || source.counts[colIndex] === selected.col)
      && axes.every((axis, index) => axis.role !== "filter" || source.counts[index] === axis.filterValue);
  }) : [];
  const selectedOccurrences = selectedSources.reduce((sum, source) => sum + (source.occurrences ?? 0), 0);
  const selectedInterval = result.runs && selectedOccurrences >= 0
    ? wilson(selectedOccurrences, result.runs)
    : selectedSources.length === 1 ? selectedSources[0]?.interval : undefined;
  const selectedExact = selectedSources.length === 1
    ? selectedSources[0].display ?? (selectedSources[0].numerator && selectedSources[0].denominator ? `${selectedSources[0].numerator}/${selectedSources[0].denominator}` : undefined)
    : selected ? table.lookup.get(pivotKey(selected.row, selected.col))?.display : undefined;
  return (
    <article className="result-card">
      <header><div><h2>{title}</h2><p>{result.elapsedMs}ms · {result.numeric}{result.runs ? ` · ${result.runs.toLocaleString()}회 · seed ${result.seed}` : ""}{result.modelHash ? ` · ${result.modelHash.slice(0, 12)}` : ""}</p></div>
        <div className="export-actions"><button onClick={() => navigator.clipboard.writeText(toCsv(table))}><Copy size={14} /> CSV 복사</button><button onClick={() => download("gacha-pivot.csv", toCsv(table), "text/csv")}><Download size={14} /> CSV</button><button onClick={() => download("gacha-result.json", JSON.stringify(result, null, 2), "application/json")}><Download size={14} /> JSON</button></div>
      </header>
      {(result.prunedMass ?? 0) > 0 && <p className="warning">표시되지 않은 프루닝 확률 질량: {formatProbability(result.prunedMass!, "scientific")}</p>}
      {result.clampEvents > 0 && <p className="warning">확률 보정 발생: {result.clampEvents}회</p>}
      {result.accumulatorClampEvents > 0 && <p className="warning">집계 변수 상한 보정: {result.accumulatorClampEvents}회</p>}
      {table.colAxis ? (
        <div className="heatmap-wrap"><table className="heatmap"><caption>{table.rowAxis?.label} × {table.colAxis.label} 결합 확률</caption><thead><tr><th scope="col">{table.rowAxis?.label}</th>{visibleCols.map((col) => <th scope="col" key={col}>{col}</th>)}</tr></thead>
          <tbody>{visibleRows.map((row) => <tr key={row}><th scope="row">{row}</th>{visibleCols.map((col) => {
            const cell = table.lookup.get(pivotKey(row, col));
            const probability = cell?.probability ?? 0;
            return <td key={col} style={{ backgroundColor: `rgba(122,92,240,${max ? 0.08 + probability / max * 0.72 : 0})` }}><button title={cell?.display ?? "0"} onClick={() => setSelected({ row, col })}>{formatPivotCell(cell, settings.probabilityFormat)}</button></td>;
          })}</tr>)}</tbody></table></div>
      ) : <BarChart cells={table.cells.map((cell) => ({ value: cell.row, probability: cell.probability, display: cell.display }))} format={settings.probabilityFormat} />}
      {(hiddenRows > 0 || hiddenCols > 0) && <p className="warning" role="status">표시 한도 {settings.maxRows.toLocaleString()}개 적용: {hiddenRows > 0 ? `행 ${hiddenRows.toLocaleString()}개` : ""}{hiddenRows > 0 && hiddenCols > 0 ? " · " : ""}{hiddenCols > 0 ? `열 ${hiddenCols.toLocaleString()}개` : ""}가 생략되었습니다. CSV 내보내기에는 전체 데이터가 포함됩니다.</p>}
      {selected && <div className="cell-detail" role="status"><b>{table.rowAxis?.label ?? "값"} {selected.row}{table.colAxis ? ` · ${table.colAxis.label} ${selected.col}` : ""}</b>
        <span>정확 표기: <code>{selectedExact ?? "0"}</code>{selectedSources.length > 1 && ` · ${selectedSources.length}개 원본 셀 집계`}</span>
        {selectedInterval && <span>오차 범위 (95%): <code>{selectedInterval.lower.toExponential(8)} – {selectedInterval.upper.toExponential(8)}</code> · 관측 {selectedOccurrences.toLocaleString()}회</span>}
        <button onClick={() => setSelected(undefined)}>닫기</button>
      </div>}
      <p className="mass">현재 슬라이스 확률 합: {formatProbability(table.total, settings.probabilityFormat)}</p>
    </article>
  );
}

function formatPivotCell(cell: PivotCell | undefined, format: AppSettings["probabilityFormat"]): string {
  if (!cell) return "0";
  if (format === "scientific" || (cell.probability === 0 && cell.display !== "0")) {
    return cell.display;
  }
  return formatProbability(cell.probability, format);
}

function wilson(successes: number, total: number) {
  if (total <= 0) return { estimate: 0, lower: 0, upper: 1 };
  const estimate = successes / total;
  const z = 1.959963984540054;
  const z2 = z * z;
  const denominator = 1 + z2 / total;
  const center = (estimate + z2 / (2 * total)) / denominator;
  const margin = z * Math.sqrt((estimate * (1 - estimate) / total) + z2 / (4 * total * total)) / denominator;
  return { estimate, lower: successes === 0 ? 0 : Math.max(0, center - margin), upper: successes === total ? 1 : Math.min(1, center + margin) };
}

function BarChart({ cells, format, cdf = false }: { cells: Array<{ value: number; probability: number; display?: string }>; format: AppSettings["probabilityFormat"]; cdf?: boolean }) {
  let running = 0;
  const values = cells.map((cell) => ({ ...cell, probability: cdf ? (running += cell.probability) : cell.probability }));
  const max = values.reduce((largest, cell) => Math.max(largest, cell.probability), Number.MIN_VALUE);
  const width = Math.max(420, values.length * 34);
  return <div className="chart-scroll"><svg className="bar-chart" role="img" aria-label={cdf ? "누적 확률 차트" : "확률 막대 차트"} viewBox={`0 0 ${width} 230`}>
    {values.map((cell, index) => {
      const height = cell.probability / max * 170;
      const display = !cdf && (format === "scientific" || (cell.probability === 0 && cell.display && cell.display !== "0"))
        ? cell.display
        : formatProbability(cell.probability, format);
      return <g key={cell.value} transform={`translate(${index * 34 + 8},0)`}><title>{cell.value}: {display}</title><rect x="0" y={190 - height} width="24" height={height} rx="3" /><text x="12" y="210" textAnchor="middle">{cell.value}</text></g>;
    })}
  </svg></div>;
}

function SeriesComparison({ dp, mc, settings }: { dp?: EngineResult; mc?: EngineResult; settings: AppSettings }) {
  const [axis, setAxis] = useState(0);
  const points = dp?.trialSeries?.marginal ?? mc?.trialSeries?.marginal;
  if (!points?.length) return null;
  const axes = points[0].axes;
  const expected = (result?: EngineResult) => result?.trialSeries?.marginal?.map((point) => ({
    trial: point.trial,
    value: point.axes[axis]?.cells.reduce((sum, cell) => sum + cell.value * (cell.probability ?? cell.interval?.estimate ?? 0), 0) ?? 0,
  })) ?? [];
  const dpPoints = expected(dp); const mcPoints = expected(mc);
  const all = [...dpPoints, ...mcPoints];
  const maxTrial = all.reduce((largest, point) => Math.max(largest, point.trial), 1);
  const maxValue = all.reduce((largest, point) => Math.max(largest, point.value), Number.MIN_VALUE);
  const path = (series: typeof all) => series.map((point, index) => `${index ? "L" : "M"} ${30 + point.trial / maxTrial * 720} ${210 - point.value / maxValue * 170}`).join(" ");
  return <article className="series-card"><header><div><h2>시행 수에 따른 추이</h2><p>결측 시행 번호는 데이터 점을 만들지 않습니다.</p></div><select value={axis} onChange={(event) => setAxis(Number(event.target.value))}>{axes.map((item, index) => <option value={index} key={item.id}>{item.id}</option>)}</select></header>
    <svg viewBox="0 0 780 235" role="img" aria-label="시행별 기대 개수"><path className="grid-line" d="M30 210 H750 M30 40 V210" />{dpPoints.length > 0 && <path className="dp-line" d={path(dpPoints)} />}{mcPoints.length > 0 && <path className="mc-line" d={path(mcPoints)} />}<text x="30" y="228">1</text><text x="725" y="228">{maxTrial}</text></svg>
    <p>보라색: 정확 계산 · 초록색: 시뮬레이션 · 최댓값 {formatProbability(maxValue, settings.probabilityFormat)}</p>
  </article>;
}

function numberValue(value: number | { probability: number }): number {
  return typeof value === "number" ? value : value.probability;
}

function FirstHitView({ result, settings }: { result: EngineResult; settings: AppSettings }) {
  const first = result.firstHit!;
  const [cdf, setCdf] = useState(false);
  const values = (cdf ? first.cdf : first.pmf).map((value, trial) => ({ value: trial, probability: numberValue(value) })).filter((cell) => cell.value > 0 && cell.probability > 0);
  const percentile = (level: number) => first.percentiles.find(([candidate]) => Math.abs(candidate - level) < 1e-9)?.[1] ?? "—";
  return <article className="series-card first-hit"><header><div><h2>첫 달성 분포</h2><p>목표를 처음 달성하는 시행</p></div><button onClick={() => setCdf(!cdf)}>{cdf ? "PMF 보기" : "누적(CDF) 보기"}</button></header>
    <div className="summary-metrics"><span>평균 <b>{first.mean?.toFixed(2) ?? "—"}</b></span><span>중앙값 <b>{percentile(0.5)}</b></span><span>90% <b>{percentile(0.9)}</b></span><span>실패 확률 <b>{formatProbability(numberValue(first.failureReachable), settings.probabilityFormat)}</b></span></div>
    <BarChart cells={values} format={settings.probabilityFormat} />
  </article>;
}
