import { useEffect, useMemo, useRef, useState } from "react";
import { Activity, Braces, ChevronDown, CircleAlert, CircleCheck, FlaskConical, Play, RotateCcw } from "lucide-react";
import { Blockly, loadIr, toolbox, workspaceToIr } from "./blockly";
import { runDpJson, type WasmEngine } from "./engine";
import { blueArchive } from "./preset";
import type { ModelIr } from "./types";
import { validateLocally } from "./validator";

type Tab = "blocks" | "json";

export function App() {
  const blockHost = useRef<HTMLDivElement>(null);
  const workspace = useRef<Blockly.WorkspaceSvg>();
  const [model, setModel] = useState<ModelIr>(blueArchive);
  const modelRef = useRef<ModelIr>(blueArchive);
  const [json, setJson] = useState(JSON.stringify(blueArchive, null, 2));
  const [tab, setTab] = useState<Tab>("blocks");
  const [advanced, setAdvanced] = useState(false);
  const [runMessage, setRunMessage] = useState("모델을 검증한 뒤 엔진을 선택하세요.");
  const [result, setResult] = useState<{
    engine: string;
    trackedLeafIds: string[];
    clampEvents: number;
    joint: Array<{ counts: number[]; probability?: number; occurrences?: number; interval?: { estimate: number; lower: number; upper: number } }>;
  } | null>(null);
  const validation = useMemo(() => validateLocally(model), [model]);
  const hasError = validation.diagnostics.some((d) => d.severity === "error");

  useEffect(() => { modelRef.current = model; }, [model]);

  useEffect(() => {
    if (!blockHost.current || workspace.current) return;
    const ws = Blockly.inject(blockHost.current, {
      toolbox,
      renderer: "zelos",
      theme: Blockly.Theme.defineTheme("gacha", {
        name: "gacha",
        base: Blockly.Themes.Classic,
        componentStyles: {
          workspaceBackgroundColour: "#12151c",
          toolboxBackgroundColour: "#191d26",
          toolboxForegroundColour: "#dce2ef",
          flyoutBackgroundColour: "#202633",
          flyoutForegroundColour: "#dce2ef",
          scrollbarColour: "#4a5263",
        },
      }),
      grid: { spacing: 24, length: 3, colour: "#2a303d", snap: true },
      zoom: { controls: true, wheel: true, startScale: 0.85 },
      trashcan: true,
    });
    workspace.current = ws;
    loadIr(ws, blueArchive);
    const listener = (event: Blockly.Events.Abstract) => {
      if (event.isUiEvent) return;
      const next = workspaceToIr(ws, modelRef.current);
      setModel(next);
      setJson(JSON.stringify(next, null, 2));
    };
    ws.addChangeListener(listener);
    const resize = () => Blockly.svgResize(ws);
    window.addEventListener("resize", resize);
    return () => { window.removeEventListener("resize", resize); ws.dispose(); workspace.current = undefined; };
  }, []);

  function applyJson() {
    try {
      const next = JSON.parse(json) as ModelIr;
      setModel(next);
      if (workspace.current) loadIr(workspace.current, next);
      setRunMessage("JSON을 블록 워크스페이스에 적용했습니다.");
    } catch (error) {
      setRunMessage(`JSON 오류: ${String(error)}`);
    }
  }

  function reset() {
    const next = structuredClone(blueArchive);
    setModel(next);
    setJson(JSON.stringify(next, null, 2));
    if (workspace.current) loadIr(workspace.current, next);
    setRunMessage("블루 아카이브 프리셋을 불러왔습니다.");
  }

  async function run(engine: "dp" | "mc") {
    if (hasError) {
      setRunMessage("오류를 해결한 뒤 실행할 수 있습니다.");
      return;
    }
    setRunMessage(engine === "dp" ? "마르코프 DP 실행 준비 중…" : "몬테카를로 실행 준비 중…");
    try {
      const wasmPath = "/wasm/gacha_wasm.js";
      const wasm = await import(/* @vite-ignore */ wasmPath) as WasmEngine;
      await wasm.default?.();
      const execution = engine === "dp"
        ? runDpJson(wasm, model)
        : { engine: "MC" as const, json: wasm.run_mc_json(JSON.stringify(model), 100_000, 42) };
      const parsed = JSON.parse(execution.json);
      const clampEvents = parsed.clampEvents ?? 0;
      setResult({
        engine: execution.engine,
        trackedLeafIds: parsed.trackedLeafIds ?? [],
        clampEvents,
        joint: parsed.joint ?? [],
      });
      const clampMessage = clampEvents > 0 ? ` · 확률 보정 ${clampEvents}회` : "";
      setRunMessage(`${execution.engine} 완료 · ${parsed.joint?.length ?? 0}개 결과 셀 · ${parsed.elapsedMs ?? 0}ms${clampMessage}`);
    } catch (error) {
      const message = String(error);
      if (message.includes("dynamically imported module") || message.includes("Cannot find module")) {
        setRunMessage("WASM 패키지가 아직 배치되지 않았습니다. 루트에서 wasm-pack build crates/gacha-wasm --target web --out-dir ../../ui/public/wasm 를 실행하세요.");
      } else {
        setRunMessage(`실행 오류: ${message}`);
      }
    }
  }

  return (
    <div className="app-shell">
      <header>
        <div className="brand"><FlaskConical size={22} /><span>Gacha Lab</span><b>β</b></div>
        <div className="header-meta"><span>Model IR v1</span><button onClick={reset}><RotateCcw size={15} /> 프리셋 초기화</button></div>
      </header>

      <main>
        <section className="editor">
          <div className="toolbar">
            <div className="tabs">
              <button className={tab === "blocks" ? "active" : ""} onClick={() => setTab("blocks")}><Activity size={16} /> 블록</button>
              <button className={tab === "json" ? "active" : ""} onClick={() => setTab("json")}><Braces size={16} /> IR JSON</button>
            </div>
            <div className="model-name">{model.name}</div>
          </div>
          <div className={tab === "blocks" ? "block-host visible" : "block-host"} ref={blockHost} />
          {tab === "json" && (
            <div className="json-pane">
              <textarea value={json} onChange={(event) => setJson(event.target.value)} spellCheck={false} />
              <button className="apply" onClick={applyJson}>JSON 적용</button>
            </div>
          )}
        </section>

        <aside>
          <div className="aside-title"><span>컴파일 결과</span><span className={hasError ? "status bad" : "status good"}>{hasError ? "오류" : "정상"}</span></div>
          <div className="leaf-list">
            {validation.leaves.map((leaf) => (
              <div className="leaf-row" key={leaf.id}>
                <span>{leaf.name}<small>{leaf.id}</small></span>
                <code>{leaf.probability.toFixed(6)}</code>
              </div>
            ))}
            <div className="total"><span>합계</span><strong>{validation.leaves.reduce((s, l) => s + l.probability, 0).toFixed(6)}</strong></div>
          </div>

          <div className="analysis">
            <h3>분석</h3>
            <Metric ok label="마르코프 분석" value={validation.estimatedStates <= 50_000_000 ? "가능" : "제한"} />
            <Metric label="제어 상태" value={validation.controlStates.toLocaleString()} />
            <Metric label="예상 결합 상태" value={`~${validation.estimatedStates.toLocaleString()}`} />
            <Metric label="수치 백엔드" value={model.run.numeric === "scaled" ? "표준" : model.run.numeric} />
          </div>

          <div className="diagnostics">
            {validation.diagnostics.map((item, index) => (
              <button key={`${item.code}-${index}`} onClick={() => {
                if (item.blockId && workspace.current) workspace.current.centerOnBlock(item.blockId);
              }}>
                {item.severity === "error" ? <CircleAlert size={16} /> : <CircleCheck size={16} />}
                <span><b>{item.code}</b>{item.message}</span>
              </button>
            ))}
          </div>

          <button className="advanced" onClick={() => setAdvanced(!advanced)}>
            고급 설정 <ChevronDown size={16} className={advanced ? "open" : ""} />
          </button>
          {advanced && (
            <div className="advanced-panel">
              <label>수치 모드
                <select value={model.run.numeric} onChange={(event) => {
                  const next = { ...model, run: { ...model.run, numeric: event.target.value as ModelIr["run"]["numeric"] } };
                  setModel(next); setJson(JSON.stringify(next, null, 2));
                }}>
                  <option value="scaled">표준 (권장)</option>
                  <option value="f64">고속</option>
                  <option value="exact">정확</option>
                </select>
              </label>
            </div>
          )}

          <div className="run-box">
            <div className="run-actions">
              <button disabled={hasError} onClick={() => run("dp")}><Play size={16} /> DP 실행</button>
              <button disabled={hasError} className="secondary" onClick={() => run("mc")}><Play size={16} /> MC 실행</button>
            </div>
            <p>{runMessage}</p>
          </div>
          {result && (
            <div className="result-panel">
              <div className="result-title">
                <span>{result.engine} 결과</span>
                <small>
                  상위 {Math.min(30, result.joint.length)}개 셀
                  {result.clampEvents > 0 && ` · 확률 보정 ${result.clampEvents}회`}
                </small>
              </div>
              <div className="result-head">
                <span>{result.trackedLeafIds.join(" · ") || "집계"}</span><span>확률</span>
              </div>
              {result.joint.slice().sort((a, b) => {
                const pa = a.probability ?? a.interval?.estimate ?? 0;
                const pb = b.probability ?? b.interval?.estimate ?? 0;
                return pb - pa;
              }).slice(0, 30).map((cell, index) => {
                const probability = cell.probability ?? cell.interval?.estimate ?? 0;
                return (
                  <div className="result-row" key={`${cell.counts.join("-")}-${index}`}>
                    <span>{cell.counts.join(" × ")}</span>
                    <span className="probability">
                      <i style={{ width: `${Math.min(100, probability * 400)}%` }} />
                      <code>{probability.toExponential(5)}</code>
                      {cell.interval && <small>95% {cell.interval.lower.toExponential(2)}–{cell.interval.upper.toExponential(2)}</small>}
                    </span>
                  </div>
                );
              })}
            </div>
          )}
        </aside>
      </main>
    </div>
  );
}

function Metric({ label, value, ok }: { label: string; value: string; ok?: boolean }) {
  return <div className="metric"><span>{ok && <CircleCheck size={15} />} {label}</span><strong>{value}</strong></div>;
}
