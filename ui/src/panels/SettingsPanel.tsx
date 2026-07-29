import type { AppSettings } from "../settings";

export function SettingsPanel({
  settings,
  update,
}: {
  settings: AppSettings;
  update: (patch: Partial<AppSettings>) => void;
}) {
  return (
    <section className="content-panel settings-panel">
      <h1>설정</h1>
      <p>브라우저에 자동 저장됩니다.</p>
      <div className="settings-grid">
        <label>블록 효과음 볼륨 <output>{Math.round(settings.soundVolume * 100)}%</output>
          <input type="range" min="0" max="1" step="0.05" value={settings.soundVolume} onChange={(event) => update({ soundVolume: Number(event.target.value) })} />
        </label>
        <label>기본 수치 모드
          <select value={settings.numeric} onChange={(event) => update({ numeric: event.target.value as AppSettings["numeric"] })}>
            <option value="scaled">표준 (권장)</option><option value="f64">고속</option><option value="exact">정확</option>
          </select>
        </label>
        <label>시뮬레이션 기본 반복 수
          <input type="number" min="1" value={settings.mcRuns} onChange={(event) => update({ mcRuns: Number(event.target.value) })} />
        </label>
        <label>시뮬레이션 기본 시드
          <input type="number" min="0" value={settings.mcSeed} onChange={(event) => update({ mcSeed: Number(event.target.value) })} />
        </label>
        <label>확률 표기
          <select value={settings.probabilityFormat} onChange={(event) => update({ probabilityFormat: event.target.value as AppSettings["probabilityFormat"] })}>
            <option value="scientific">지수</option><option value="decimal">소수</option><option value="percent">퍼센트</option><option value="reciprocal">1/N</option>
          </select>
        </label>
        <label>결과 표 최대 행 수
          <input type="number" min="10" max="10000" value={settings.maxRows} onChange={(event) => update({ maxRows: Number(event.target.value) })} />
        </label>
      </div>
      <p className="setting-note">Blockly는 재생 시 볼륨 값을 받을 수 있어 워크스페이스를 다시 만들지 않고 즉시 반영합니다.</p>
    </section>
  );
}
