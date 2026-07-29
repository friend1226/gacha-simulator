import { Activity, Braces, CircleAlert, CircleCheck } from "lucide-react";
import type { RefObject } from "react";
import type { Diagnostic, ModelIr, ValidationView } from "../types";

export function ModelPanel({
  blockHost,
  editorTab,
  setEditorTab,
  model,
  json,
  setJson,
  applyJson,
  validation,
  focusDiagnostic,
  openHelp,
}: {
  blockHost: RefObject<HTMLDivElement>;
  editorTab: "blocks" | "json";
  setEditorTab: (tab: "blocks" | "json") => void;
  model: ModelIr;
  json: string;
  setJson: (value: string) => void;
  applyJson: () => void;
  validation: ValidationView;
  focusDiagnostic: (diagnostic: Diagnostic) => void;
  openHelp: (code: string) => void;
}) {
  const hasError = validation.diagnostics.some((item) => item.severity === "error");
  return (
    <div className="model-layout">
      <section className="editor">
        <div className="toolbar">
          <div className="tabs">
            <button className={editorTab === "blocks" ? "active" : ""} onClick={() => setEditorTab("blocks")}><Activity size={16} /> 블록</button>
            <button className={editorTab === "json" ? "active" : ""} onClick={() => setEditorTab("json")}><Braces size={16} /> IR JSON</button>
          </div>
          <div className="model-name">{model.name}</div>
        </div>
        <div className={editorTab === "blocks" ? "block-host visible" : "block-host"} ref={blockHost} />
        {editorTab === "json" && (
          <div className="json-pane">
            <textarea value={json} onChange={(event) => setJson(event.target.value)} spellCheck={false} />
            <button className="apply" onClick={applyJson}>JSON 적용</button>
          </div>
        )}
      </section>
      <aside className="validation-panel">
        <div className="aside-title">
          <span>검증 결과</span>
          <span className={hasError ? "status bad" : "status good"}>{hasError ? "오류" : "정상"}</span>
        </div>
        <div className="leaf-list">
          {validation.leaves.map((leaf) => (
            <div className="leaf-row" key={leaf.id}>
              <span>{leaf.name}<small>{leaf.id}</small></span>
              <code>{leaf.probability.toFixed(6)}</code>
            </div>
          ))}
          <div className="total"><span>합계</span><strong>{validation.leaves.reduce((sum, leaf) => sum + leaf.probability, 0).toFixed(6)}</strong></div>
        </div>
        <div className="analysis">
          <h3>분석</h3>
          <Metric label="정확 계산" value={validation.estimatedStates <= 50_000_000 ? "가능" : "제한"} ok />
          <Metric label="가챠 규칙 상태" value={validation.controlStates.toLocaleString()} />
          <Metric label="계산 규모" value={`~${validation.estimatedStates.toLocaleString()}`} />
          <Metric label="수치 모드" value={model.run.numeric === "scaled" ? "표준" : model.run.numeric} />
        </div>
        <div className="diagnostics">
          {validation.diagnostics.map((item, index) => (
            <button key={`${item.code}-${index}`} onClick={() => item.blockId ? focusDiagnostic(item) : openHelp(item.code)}>
              {item.severity === "error" ? <CircleAlert size={16} /> : <CircleCheck size={16} />}
              <span><b>{item.code}</b>{item.message}<small onClick={(event) => { event.stopPropagation(); openHelp(item.code); }}>해결 방법 보기</small></span>
            </button>
          ))}
        </div>
      </aside>
    </div>
  );
}

function Metric({ label, value, ok }: { label: string; value: string; ok?: boolean }) {
  return <div className="metric"><span>{ok && <CircleCheck size={15} />} {label}</span><strong>{value}</strong></div>;
}
