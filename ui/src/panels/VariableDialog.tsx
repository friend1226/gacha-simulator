import { useEffect, useState, type FormEvent } from "react";
import type { WorkspaceVariableDefinition } from "../blockly";

type VariableDraft = Omit<WorkspaceVariableDefinition, "variableId">;

function emptyDraft(role: "control" | "accumulator"): VariableDraft {
  return {
    id: role === "control" ? "pity" : "spent",
    role,
    init: 0,
    max: role === "control" ? 89 : 60_000,
    ...(role === "accumulator" ? { clampPolicy: "saturate" as const } : {}),
  };
}

export function VariableDialog({
  variables,
  initialRole,
  onSave,
  onDelete,
  onClose,
}: {
  variables: WorkspaceVariableDefinition[];
  initialRole?: "control" | "accumulator";
  onSave: (draft: VariableDraft, variableId?: string) => string | undefined;
  onDelete: (variableId: string) => void;
  onClose: () => void;
}) {
  const [selectedId, setSelectedId] = useState<string>();
  const [draft, setDraft] = useState<VariableDraft>(() => emptyDraft(initialRole ?? "control"));
  const [error, setError] = useState("");

  useEffect(() => {
    if (initialRole) {
      setSelectedId(undefined);
      setDraft(emptyDraft(initialRole));
      return;
    }
    const first = variables[0];
    setSelectedId(first?.variableId);
    setDraft(first ? definitionToDraft(first) : emptyDraft("control"));
  }, [initialRole, variables]);

  function choose(variableId: string) {
    if (!variableId) {
      setSelectedId(undefined);
      setDraft(emptyDraft("control"));
    } else {
      const variable = variables.find((item) => item.variableId === variableId);
      if (!variable) return;
      setSelectedId(variable.variableId);
      setDraft(definitionToDraft(variable));
    }
    setError("");
  }

  function submit(event: FormEvent) {
    event.preventDefault();
    const message = onSave(draft, selectedId);
    if (message) {
      setError(message);
      return;
    }
    onClose();
  }

  return (
    <div className="dialog-backdrop" role="presentation">
      <section className="variable-dialog" role="dialog" aria-modal="true" aria-labelledby="variable-dialog-title">
        <header>
          <div>
            <h2 id="variable-dialog-title">변수 관리</h2>
            <p>변수 ID를 바꾸면 연결된 모든 참조도 함께 바뀝니다.</p>
          </div>
          <button type="button" aria-label="변수 관리 닫기" onClick={onClose}>×</button>
        </header>
        <form onSubmit={submit}>
          {!initialRole && (
            <label>
              편집할 변수
              <select value={selectedId ?? ""} onChange={(event) => choose(event.target.value)}>
                <option value="">새 변수</option>
                {variables.map((variable) => (
                  <option key={variable.variableId} value={variable.variableId}>
                    {variable.id} · {variable.role === "control" ? "제어" : "집계"}
                  </option>
                ))}
              </select>
            </label>
          )}
          <div className="variable-form-grid">
            <label>
              ID
              <input
                value={draft.id}
                onChange={(event) => setDraft((current) => ({ ...current, id: event.target.value }))}
                autoFocus
              />
            </label>
            <label>
              표시 이름
              <input
                value={draft.name ?? ""}
                placeholder="선택 사항"
                onChange={(event) => setDraft((current) => ({
                  ...current,
                  name: event.target.value || undefined,
                }))}
              />
            </label>
            <label>
              역할
              <select
                value={draft.role}
                disabled={Boolean(selectedId)}
                title={selectedId ? "기존 변수의 역할은 참조 안전을 위해 변경할 수 없습니다." : undefined}
                onChange={(event) => {
                  const role = event.target.value as VariableDraft["role"];
                  setDraft((current) => ({
                    ...current,
                    role,
                    clampPolicy: role === "accumulator"
                      ? current.clampPolicy ?? "saturate"
                      : current.clampPolicy,
                  }));
                }}
              >
                <option value="control">제어 변수</option>
                <option value="accumulator">집계 변수</option>
              </select>
            </label>
            <label>
              초기값
              <input
                type="number"
                min={0}
                step={1}
                value={draft.init}
                onChange={(event) => setDraft((current) => ({
                  ...current,
                  init: Number(event.target.value),
                }))}
              />
            </label>
            <label>
              상한
              <input
                type="number"
                min={0}
                step={1}
                value={draft.max ?? ""}
                onChange={(event) => setDraft((current) => ({
                  ...current,
                  max: event.target.value === "" ? undefined : Number(event.target.value),
                }))}
              />
            </label>
            <label title="error를 선택하면 갱신식이 범위를 넘을 가능성이 있을 때 E004로 실행을 막습니다.">
              범위 초과 정책
              <select
                value={draft.clampPolicy ?? "saturate"}
                onChange={(event) => setDraft((current) => ({
                  ...current,
                  clampPolicy: event.target.value as "saturate" | "error",
                }))}
              >
                <option value="saturate">상한으로 보정</option>
                <option value="error">범위 초과 시 오류</option>
              </select>
            </label>
          </div>
          <p className="variable-dialog-note">
            ‘범위 초과 시 오류’는 갱신식이 0..상한을 벗어날 가능성이 있으면 E004를 표시합니다.
          </p>
          {error && <p className="variable-dialog-error" role="alert">{error}</p>}
          <footer>
            {selectedId && (
              <button
                type="button"
                className="danger"
                onClick={() => {
                  onDelete(selectedId);
                  onClose();
                }}
              >
                변수 삭제
              </button>
            )}
            <span />
            <button type="button" onClick={onClose}>취소</button>
            <button type="submit" className="primary">저장</button>
          </footer>
        </form>
      </section>
    </div>
  );
}

function definitionToDraft(variable: WorkspaceVariableDefinition): VariableDraft {
  return {
    id: variable.id,
    role: variable.role,
    init: variable.init,
    max: variable.max,
    ...(variable.name ? { name: variable.name } : {}),
    ...(variable.clampPolicy ? { clampPolicy: variable.clampPolicy } : {}),
    ...(variable.blockId ? { blockId: variable.blockId } : {}),
  };
}
