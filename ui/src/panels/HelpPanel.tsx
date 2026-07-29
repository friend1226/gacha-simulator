import { diagnosticHelp } from "../labels";

const glossary = [
  ["뽑기 결과", "뽑았을 때 나올 수 있는 것 (예: 3성, 픽업)"],
  ["최종 항목", "더 이상 나뉘지 않는 뽑기 결과 (리프)"],
  ["가챠 규칙 변수", "확률에 영향을 주는 값 (예: 천장 카운터)"],
  ["집계 변수", "통계용으로만 세며 확률에는 영향을 주지 않는 값"],
  ["결과에 따른 변화", "특정 결과가 나왔을 때 규칙 변수를 바꾸는 전이"],
  ["시행 횟수 이벤트", "N회째에 확정 지급처럼 자동으로 일어나는 트리거"],
];

export function HelpPanel({ focusCode }: { focusCode?: string }) {
  return (
    <section className="content-panel help-panel">
      <h1>도움말</h1>
      <div className="help-card">
        <h2>3분 안내</h2>
        <ol>
          <li>모델 탭에서 프리셋을 열고 확률이나 시행 횟수를 바꿉니다.</li>
          <li>검증 결과에서 오류가 없는지 확인합니다.</li>
          <li>결과 탭에서 정확 계산 또는 시뮬레이션을 실행합니다.</li>
          <li>행·열·집계·필터를 바꿔 원하는 확률을 읽습니다.</li>
        </ol>
      </div>
      <h2>용어집</h2>
      <dl className="glossary">{glossary.map(([term, description]) => <div key={term}><dt>{term}</dt><dd>{description}</dd></div>)}</dl>
      <h2>계산 방식</h2>
      <div className="help-columns">
        <div className="help-card"><h3>정확 계산 (DP)</h3><p>가능한 상태를 모두 합산합니다. 같은 입력은 항상 같은 결과를 내며, 계산 규모가 크면 느려집니다.</p></div>
        <div className="help-card"><h3>시뮬레이션 (MC)</h3><p>무작위로 여러 번 실행합니다. 빠르지만 오차 범위가 있으며, 결과에는 반복 수와 시드가 함께 표시됩니다.</p></div>
      </div>
      <h2>수치 모드</h2>
      <p><b>표준</b>은 극소 확률을 보존하고, <b>고속</b>은 일반 부동소수점, <b>정확</b>은 BigInt 공통분모로 계산합니다.</p>
      <h2>진단 코드</h2>
      <div className="diagnostic-help">
        {Object.entries(diagnosticHelp).map(([code, item]) => (
          <article id={`help-${code}`} className={focusCode === code ? "focused" : ""} key={code}>
            <b>{code} · {item.title}</b><span>{item.fix}</span>
          </article>
        ))}
      </div>
      <p>더 알아보기: <code>docs/USAGE.md</code> · <code>docs/DESIGN.md</code></p>
    </section>
  );
}
