import { Activity, Braces, CircleAlert, CircleCheck, X } from "lucide-react";
import type { RefObject } from "react";
import type { UnsupportedBlockItem } from "../blockly";
import type { Diagnostic, ModelIr, ValidationView } from "../types";

export function ModelPanel({
  blockHost,
  editorTab,
  setEditorTab,
  showMobileBlockNotice,
  dismissMobileBlockNotice,
  unsupportedBlockItems,
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
  showMobileBlockNotice: boolean;
  dismissMobileBlockNotice: () => void;
  unsupportedBlockItems: UnsupportedBlockItem[];
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
        {editorTab === "blocks" && showMobileBlockNotice && (
          <div className="mobile-block-notice" role="note">
            <span>작은 화면에서는 블록 편집이 불편할 수 있습니다. IR JSON 탭을 쓰거나 큰 화면에서 편집하세요.</span>
            <button type="button" aria-label="안내 닫기" onClick={dismissMobileBlockNotice}><X size={16} /></button>
          </div>
        )}
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
        {unsupportedBlockItems.length > 0 && (
          <div className="unsupported-block-warning" role="status">
            <b>블록으로 표현되지 않는 규칙 {unsupportedBlockItems.length}개를 사용합니다.</b>
            <span>블록을 편집해도 그대로 보존됩니다. 내용은 IR JSON 탭에서 볼 수 있습니다.</span>
            {unsupportedBlockItems.length >= 4 ? (
              <details>
                <summary>규칙 경로 {unsupportedBlockItems.length}개 보기</summary>
                <UnsupportedBlockList items={unsupportedBlockItems} />
              </details>
            ) : (
              <UnsupportedBlockList items={unsupportedBlockItems} />
            )}
            <button type="button" onClick={() => setEditorTab("json")}><Braces size={14} /> IR JSON 열기</button>
          </div>
        )}
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

function UnsupportedBlockList({ items }: { items: UnsupportedBlockItem[] }) {
  return <ul>{items.map((item) => <li key={item.path}><code>{item.path}</code> · {item.description}</li>)}</ul>;
}

function Metric({ label, value, ok }: { label: string; value: string; ok?: boolean }) {
  return <div className="metric"><span>{ok && <CircleCheck size={15} />} {label}</span><strong>{value}</strong></div>;
}
