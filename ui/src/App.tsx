import { useEffect, useMemo, useRef, useState } from "react";
import { BookOpen, FlaskConical, RotateCcw, Save, Settings, Upload } from "lucide-react";
import {
  Blockly,
  GachaConnectionChecker,
  deleteWorkspaceVariable,
  getUnsupportedBlockItems,
  installVariableToolbox,
  installWorkspaceVolume,
  listWorkspaceVariables,
  loadIr,
  refreshVariableToolbox,
  saveWorkspaceVariable,
  toolbox,
  workspaceToIr,
  type UnsupportedBlockItem,
  type WorkspaceVariableDefinition,
} from "./blockly";
import { EngineCancelledError, loadEngineBackend, runDpJson, type EngineBackend, type EngineProgress } from "./engine";
import { parseEngineError, type EngineErrorPresentation } from "./engineDiagnostics";
import { confidenceLabel as confidenceLabelFor } from "./labels";
import { blueArchive, presets } from "./preset";
import { initialProvenance, nextProvenance, serializeModelForExport, type ModelProvenance, type ProvenanceEvent } from "./provenance";
import { loadSettings, normalizeSettings, saveSettings, type AppSettings } from "./settings";
import { normalizeFirstHit } from "./firstHit";
import type { Diagnostic, EngineResult, ModelIr } from "./types";
import { validateLocally } from "./validator";
import { HelpPanel } from "./panels/HelpPanel";
import { ModelPanel } from "./panels/ModelPanel";
import { ResultPanel } from "./panels/ResultPanel";
import { SettingsPanel } from "./panels/SettingsPanel";
import { VariableDialog } from "./panels/VariableDialog";
import { MODEL_STORAGE } from "./storage";

type TopTab = "model" | "results" | "help" | "settings";
const MOBILE_BLOCK_NOTICE_STORAGE = "gacha-lab.mobile-block-notice.dismissed";
const MOBILE_LAYOUT_CSS_FLAG = "--gacha-mobile-layout";

function initialModel(): { model: ModelIr; provenance: ModelProvenance; selectedPreset: string } {
  try {
    const source = localStorage.getItem(MODEL_STORAGE);
    return {
      model: source ? JSON.parse(source) as ModelIr : structuredClone(blueArchive),
      provenance: initialProvenance(Boolean(source)),
      selectedPreset: source ? "" : "blue-archive-pickup",
    };
  } catch {
    return {
      model: structuredClone(blueArchive),
      provenance: "pristine",
      selectedPreset: "blue-archive-pickup",
    };
  }
}

export function App() {
  const [initial] = useState(initialModel);
  const blockHost = useRef<HTMLDivElement>(null);
  const workspace = useRef<Blockly.WorkspaceSvg>();
  const engineBackend = useRef<EngineBackend>();
  const executionId = useRef(0);
  const [model, setModel] = useState<ModelIr>(initial.model);
  const modelRef = useRef(model);
  const [provenance, setProvenance] = useState<ModelProvenance>(initial.provenance);
  const [json, setJson] = useState(() => JSON.stringify(model, null, 2));
  const [topTab, setTopTab] = useState<TopTab>("model");
  const [editorTab, setEditorTab] = useState<"blocks" | "json">("blocks");
  const [unsupportedBlockItems, setUnsupportedBlockItems] = useState<UnsupportedBlockItem[]>([]);
  const [variableDialog, setVariableDialog] = useState<{
    initialRole?: "control" | "accumulator";
  }>();
  const [showMobileBlockNotice, setShowMobileBlockNotice] = useState(false);
  const [settings, setSettings] = useState<AppSettings>(loadSettings);
  const settingsRef = useRef(settings);
  const [selectedPreset, setSelectedPreset] = useState(initial.selectedPreset);
  const [helpCode, setHelpCode] = useState<string>();
  const [message, setMessage] = useState("모델을 검증한 뒤 계산 방식을 선택하세요.");
  const [running, setRunning] = useState<"dp" | "mc">();
  const [canCancel, setCanCancel] = useState(false);
  const [progress, setProgress] = useState<EngineProgress>();
  const [engineError, setEngineError] = useState<EngineErrorPresentation>();
  const [results, setResults] = useState<{ dp?: EngineResult; mc?: EngineResult }>({});
  const validation = useMemo(() => validateLocally(model), [model]);
  const hasError = validation.diagnostics.some((item) => item.severity === "error");

  useEffect(() => {
    modelRef.current = model;
    try {
      localStorage.setItem(MODEL_STORAGE, JSON.stringify(model));
    } catch {
      // Keep the current model usable when persistent storage is unavailable.
    }
    setEngineError(undefined);
  }, [model]);
  useEffect(() => { settingsRef.current = settings; saveSettings(settings); }, [settings]);

  useEffect(() => {
    if (editorTab !== "blocks") {
      setShowMobileBlockNotice(false);
      return;
    }
    const updateNotice = () => {
      let dismissed = false;
      try {
        dismissed = localStorage.getItem(MOBILE_BLOCK_NOTICE_STORAGE) === "1";
      } catch {
        // Keep the notice available when persistent storage is unavailable.
      }
      const mobileLayout = getComputedStyle(document.documentElement)
        .getPropertyValue(MOBILE_LAYOUT_CSS_FLAG)
        .trim() === "1";
      setShowMobileBlockNotice(mobileLayout && !dismissed);
    };
    updateNotice();
    window.addEventListener("resize", updateNotice);
    return () => window.removeEventListener("resize", updateNotice);
  }, [editorTab]);

  useEffect(() => {
    if (!blockHost.current || workspace.current) return;
    const ws = Blockly.inject(blockHost.current, {
      toolbox,
      renderer: "thrasos",
      sounds: true,
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
      zoom: { controls: true, wheel: true, startScale: 0.8 },
      trashcan: true,
      plugins: { connectionChecker: GachaConnectionChecker },
    });
    workspace.current = ws;
    installWorkspaceVolume(ws, () => settingsRef.current.soundVolume);
    installVariableToolbox(ws, {
      create: (role) => setVariableDialog({ initialRole: role }),
      manage: () => setVariableDialog({}),
    });
    setUnsupportedBlockItems(loadIr(ws, modelRef.current));
    refreshVariableToolbox(ws);
    const listener = (event: Blockly.Events.Abstract) => {
      if (event.isUiEvent) return;
      if (event.type === Blockly.Events.VAR_CREATE
        || event.type === Blockly.Events.VAR_DELETE
        || event.type === Blockly.Events.VAR_RENAME) {
        refreshVariableToolbox(ws);
      }
      syncWorkspaceModel(ws);
    };
    ws.addChangeListener(listener);
    const resize = () => Blockly.svgResize(ws);
    window.addEventListener("resize", resize);
    return () => {
      window.removeEventListener("resize", resize);
      ws.dispose();
      workspace.current = undefined;
    };
  }, []);

  useEffect(() => {
    if (topTab === "model" && workspace.current) {
      requestAnimationFrame(() => Blockly.svgResize(workspace.current!));
    }
  }, [topTab]);

  function setModelSynced(
    next: ModelIr,
    reloadBlocks = false,
    provenanceEvent: ProvenanceEvent = "updateModel",
  ) {
    setModel(next);
    setJson(JSON.stringify(next, null, 2));
    setProvenance((current) => nextProvenance(current, provenanceEvent));
    if (provenanceEvent === "openModel" || provenanceEvent === "restore") {
      setSelectedPreset("");
    }
    if (reloadBlocks && workspace.current) {
      setUnsupportedBlockItems(loadIr(workspace.current, next));
      refreshVariableToolbox(workspace.current);
    }
  }

  function syncWorkspaceModel(ws = workspace.current) {
    if (!ws) return;
    const next = workspaceToIr(ws, modelRef.current);
    setModelSynced(next, false, "blockEdit");
    setUnsupportedBlockItems(getUnsupportedBlockItems(ws));
  }

  function saveVariable(
    draft: Omit<WorkspaceVariableDefinition, "variableId">,
    variableId?: string,
  ) {
    const ws = workspace.current;
    if (!ws) return "블록 워크스페이스가 준비되지 않았습니다.";
    try {
      saveWorkspaceVariable(ws, draft, variableId);
      refreshVariableToolbox(ws);
      syncWorkspaceModel(ws);
      return undefined;
    } catch (error) {
      return error instanceof Error ? error.message : String(error);
    }
  }

  function deleteVariable(variableId: string) {
    const ws = workspace.current;
    if (!ws) return;
    deleteWorkspaceVariable(ws, variableId);
    refreshVariableToolbox(ws);
    syncWorkspaceModel(ws);
  }

  function applyJson() {
    try {
      const next = JSON.parse(json) as ModelIr;
      setModelSynced(next, true, "applyJson");
      setMessage("JSON을 블록 워크스페이스에 적용했습니다.");
    } catch (error) {
      setMessage(`JSON 오류: ${String(error)}`);
    }
  }

  function loadPreset(id = selectedPreset) {
    const preset = presets.find((item) => item.id === id) ?? presets[0];
    setSelectedPreset(preset.id);
    const next = structuredClone(preset.model);
    next.run.numeric = settingsRef.current.numeric;
    setModelSynced(next, true, "loadPreset");
    setResults({});
    setMessage(`${preset.meta.game} · ${preset.meta.banner} 프리셋을 불러왔습니다.`);
  }

  function updateSettings(patch: Partial<AppSettings>) {
    setSettings((current) => normalizeSettings({ ...current, ...patch }));
  }

  function dismissMobileBlockNotice() {
    try {
      localStorage.setItem(MOBILE_BLOCK_NOTICE_STORAGE, "1");
    } catch {
      // Dismiss for this session even when persistent storage is unavailable.
    }
    setShowMobileBlockNotice(false);
  }

  function focusDiagnostic(diagnostic: Diagnostic) {
    if (diagnostic.blockId && workspace.current) workspace.current.centerOnBlock(diagnostic.blockId);
  }

  function openHelp(code: string) {
    setHelpCode(code);
    setTopTab("help");
    requestAnimationFrame(() => document.getElementById(`help-${code}`)?.scrollIntoView({ block: "center" }));
  }

  async function run(engine: "dp" | "mc", runs: number, seed: number) {
    if (hasError) {
      setMessage("모델 오류를 해결한 뒤 실행할 수 있습니다.");
      setTopTab("model");
      return;
    }
    const currentExecution = ++executionId.current;
    setRunning(engine);
    setCanCancel(false);
    setProgress(undefined);
    setEngineError(undefined);
    setMessage(engine === "dp" ? "정확 계산을 실행하는 중…" : "시뮬레이션을 실행하는 중…");
    try {
      const backend = await loadEngineBackend();
      engineBackend.current = backend;
      if (currentExecution !== executionId.current) return;
      setCanCancel(backend.platform === "web");
      const updateProgress = (next: EngineProgress) => {
        if (currentExecution === executionId.current) setProgress(next);
      };
      const source = JSON.stringify(model);
      const execution = engine === "dp"
        ? await runDpJson(backend, model, updateProgress)
        : { engine: "MC" as const, json: await backend.runMcJson(source, runs, seed, updateProgress) };
      if (currentExecution !== executionId.current) return;
      const parsed = JSON.parse(execution.json) as Partial<EngineResult>;
      const result: EngineResult = {
        engine: execution.engine === "EXACT" ? "Exact" : execution.engine,
        numeric: parsed.numeric ?? (engine === "mc" ? model.run.numeric : "scaled"),
        trials: parsed.trials ?? model.run.maxTrials,
        peakStates: parsed.peakStates,
        runs: parsed.runs,
        seed: parsed.seed,
        trackedLeafIds: parsed.trackedLeafIds ?? [],
        joint: parsed.joint ?? [],
        firstHit: normalizeFirstHit(parsed.firstHit, parsed.runs),
        prunedMass: parsed.prunedMass ?? 0,
        elapsedMs: parsed.elapsedMs ?? 0,
        clampEvents: parsed.clampEvents ?? 0,
        accumulatorClampEvents: parsed.accumulatorClampEvents ?? 0,
        trialSeries: parsed.trialSeries,
        modelHash: parsed.modelHash,
      };
      setResults((current) => engine === "mc" ? { ...current, mc: result } : { ...current, dp: result });
      setEngineError(undefined);
      setMessage(`${engine === "dp" ? "정확 계산" : "시뮬레이션"} 완료 · ${result.joint.length.toLocaleString()}개 결과 셀 · ${result.elapsedMs}ms`);
      setTopTab("results");
    } catch (error) {
      if (error instanceof EngineCancelledError || currentExecution !== executionId.current) return;
      const text = String(error);
      if (text.includes("dynamically imported module") || text.includes("Cannot find module")) {
        setEngineError(undefined);
        setMessage("WASM 패키지가 없습니다. 루트에서 wasm-pack build 명령을 실행하세요.");
      } else {
        const presentation = parseEngineError(text);
        setEngineError(presentation.diagnostics.length ? presentation : undefined);
        setMessage(presentation.diagnostics.length ? "엔진 진단을 확인하세요." : `실행 오류: ${text}`);
      }
    } finally {
      if (currentExecution === executionId.current) {
        setRunning(undefined);
        setCanCancel(false);
        setProgress(undefined);
      }
    }
  }

  function cancelRun() {
    if (!running || !canCancel) return;
    executionId.current += 1;
    engineBackend.current?.cancel();
    setRunning(undefined);
    setCanCancel(false);
    setProgress(undefined);
    setEngineError(undefined);
    setMessage("계산을 취소했습니다.");
  }

  function saveModel() {
    const url = URL.createObjectURL(new Blob([serializeModelForExport(model, provenance)], { type: "application/json" }));
    const anchor = document.createElement("a");
    anchor.href = url; anchor.download = "gacha-model.json"; anchor.click();
    URL.revokeObjectURL(url);
  }

  async function openModel(file?: File) {
    if (!file) return;
    try {
      setModelSynced(JSON.parse(await file.text()) as ModelIr, true, "openModel");
      setMessage(`${file.name}을 불러왔습니다.`);
    } catch (error) {
      setMessage(`모델 파일 오류: ${String(error)}`);
    }
  }

  const preset = presets.find((item) => item.id === selectedPreset);
  const confidenceLabel = preset ? confidenceLabelFor(preset.meta.confidence) : "";
  return (
    <div className="app-shell">
      <header className="app-header">
        <div className="brand"><FlaskConical size={22} /><span>Gacha Lab</span><b>β</b></div>
        <nav className="top-tabs">
          <button className={topTab === "model" ? "active" : ""} onClick={() => setTopTab("model")}>모델</button>
          <button className={topTab === "results" ? "active" : ""} onClick={() => setTopTab("results")}>결과</button>
          <button className={topTab === "help" ? "active" : ""} onClick={() => setTopTab("help")}><BookOpen size={14} /> 도움말</button>
          <button className={topTab === "settings" ? "active" : ""} onClick={() => setTopTab("settings")}><Settings size={14} /> 설정</button>
        </nav>
        <div className="header-actions">
          <select value={selectedPreset} onChange={(event) => loadPreset(event.target.value)}>
            {provenance === "none" && <option value="">현재 모델 · 출처 없음</option>}
            {presets.map((item) => <option value={item.id} key={item.id}>{item.meta.game} · {item.meta.banner}</option>)}
          </select>
          {provenance !== "none" && preset && (
            <span
              className={`source-badge ${provenance === "dirty" ? "modified" : preset.meta.confidence === "official" ? "official" : ""}`}
              title={`${preset.meta.confidence}${provenance === "dirty" ? " · 수정됨" : ""}`}
            >
              {confidenceLabel}{provenance === "dirty" ? " · 수정됨" : ""}
            </span>
          )}
          <button onClick={() => loadPreset()}><RotateCcw size={14} /> 초기화</button>
          <button onClick={saveModel}><Save size={14} /> 저장</button>
          <label className="file-button"><Upload size={14} /> 열기<input type="file" accept=".json,application/json" onChange={(event) => openModel(event.target.files?.[0])} /></label>
        </div>
      </header>
      <main className="app-main">
        <div hidden={topTab !== "model"} className="tab-fill">
          <ModelPanel blockHost={blockHost} editorTab={editorTab} setEditorTab={setEditorTab} showMobileBlockNotice={showMobileBlockNotice} dismissMobileBlockNotice={dismissMobileBlockNotice} unsupportedBlockItems={unsupportedBlockItems} model={model} json={json} setJson={setJson} applyJson={applyJson} validation={validation} focusDiagnostic={focusDiagnostic} openHelp={openHelp} />
        </div>
        {topTab === "results" && <ResultPanel model={model} updateModel={(next) => setModelSynced(next)} settings={settings} results={results} running={running} canCancel={canCancel} progress={progress} engineError={engineError} message={message} run={run} cancelRun={cancelRun} openHelp={openHelp} />}
        {topTab === "help" && <HelpPanel focusCode={helpCode} />}
        {topTab === "settings" && <SettingsPanel settings={settings} update={updateSettings} />}
      </main>
      {variableDialog && (
        <VariableDialog
          variables={workspace.current ? listWorkspaceVariables(workspace.current) : []}
          initialRole={variableDialog.initialRole}
          onSave={saveVariable}
          onDelete={deleteVariable}
          onClose={() => setVariableDialog(undefined)}
        />
      )}
    </div>
  );
}
