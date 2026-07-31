export const engineLabels = {
  DP: "정확 계산 (DP)",
  Exact: "정확 계산 (Exact)",
  MC: "시뮬레이션 (MC)",
} as const;

export const confidenceLabels: Record<string, string> = {
  official: "공식 공시",
  datamined: "데이터마이닝",
  "community-estimate": "커뮤니티 추정",
};

export function confidenceLabel(value: string): string {
  return confidenceLabels[value] ?? value;
}

export const diagnosticHelp: Record<string, { title: string; fix: string }> = {
  E000: { title: "지원하지 않는 모델 버전", fix: "IR 버전을 1 또는 2로 바꾸세요." },
  E001: { title: "ID 중복", fix: "뽑기 결과마다 서로 다른 ID를 사용하세요." },
  E002: { title: "음수 확률", fix: "확률식의 최솟값이 0 이상이 되게 수정하세요." },
  E003: {
    title: "확률표 계산 실패",
    fix: "미해석 변수, 음수 확률, 최상위 확률 합 1 초과, 리프 확률 질량 불일치를 확인하세요.",
  },
  E004: { title: "상태 상한 오류", fix: "초기값과 상한을 0 이상의 정수로 지정하세요." },
  E005: { title: "없는 결과를 참조", fix: "확률 규칙의 대상 ID를 확인하세요." },
  E006: { title: "표현식 오류", fix: "식의 연산자와 리터럴 형식을 확인하세요." },
  E007: { title: "확정 지급 대상 오류", fix: "최종 항목(리프) ID를 선택하세요." },
  E008: { title: "자동 카운터 직접 변경", fix: "리프 카운터 대신 가챠 규칙 변수를 변경하세요." },
  E009: { title: "잘못된 stat 선언", fix: "통계용 값은 집계 변수로 선언하세요." },
  E010: { title: "집계 변수 테이블 한도 초과", fix: "집계 변수 max를 줄이거나 갱신식에서 제어 변수·시행 번호 의존을 제거하세요." },
  E011: { title: "DP 실행 상태 한도 초과", fix: "시행 횟수나 추적 대상을 줄이거나 수치 모드를 시뮬레이션(MC)으로 바꾸세요." },
  E012: { title: "확률표 한도 초과", fix: "시행 횟수·뽑기 결과 수·도달 가능한 제어 상태를 줄이거나 확률식에서 제어 변수 참조를 제거하세요." },
  W001: { title: "자식 확률 보정", fix: "자식 확률 합을 부모 확률 이하로 맞추세요." },
  W002: { title: "그외 항목 자동 생성", fix: "정상 동작이며 수정할 필요가 없습니다." },
  W003: { title: "달성 불가능한 조건", fix: "시행 횟수나 목표 개수를 확인하세요." },
  W004: { title: "큰 계산 규모", fix: "시행 횟수나 추적 대상을 줄이거나 수치 모드를 시뮬레이션(MC)으로 바꾸세요." },
  W005: { title: "큰 정확 분모", fix: "표준 수치 모드를 고려하세요." },
  W006: { title: "추적 대상 없음", fix: "뽑기 결과 또는 집계 변수 ID를 확인하세요." },
  W007: { title: "지급 시행 슬롯 부족", fix: "maxTrials 또는 지급 시점을 조정하세요." },
  W008: { title: "중복 집계 축 제거", fix: "자동 카운터로 계산되어 별도 상태 축은 만들지 않았습니다." },
  W009: { title: "큰 집계 변수 테이블", fix: "표시된 제어·시행·max 축을 확인하고 불필요한 의존성이나 상한을 줄이세요." },
  W010: { title: "큰 확률표", fix: "시행 횟수·뽑기 결과 수·도달 가능한 제어 상태를 줄이거나 확률식에서 제어 변수 참조를 제거하세요." },
};
