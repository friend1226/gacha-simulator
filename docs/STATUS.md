# 구현 상태

## 완료

- Cargo workspace: `gacha-core`, `gacha-cli`, `gacha-wasm`
- 정확 십진/분수/지수 리터럴 파서 (`0.007` → `7/1000`)
- `Prob` 계층과 `F64`, 비트 정규화 기반 `ScaledF64`
- 공통분모 BigInt exact DP (내부 루프 GCD 없음, 선택적 레이어 약분)
- Model IR v1 역직렬화, E001/E003/E004/E006/E007/E008 중심 구조화 진단
- 표현식 AST → 스택 바이트코드 및 유리수 평가
- 엔티티 트리 → 배타적 리프 분할과 `__other__` 생성
- 제어상태·시행별 정확 유리수 확률표 사전계산
- 제어 전이, 확정 지급, 제어상태 기반 동적 확률
- 재현 가능한 xoshiro256++ MC와 alias sampling, Wilson 95% 구간
- MC↔DP 10개 모델·희귀 비율 교차검증, exact↔ScaledF64 복합·극소확률 비교, 200회 지급 전파 회귀 테스트
- GitHub Actions 기반 Rust/UI CI
- 희소 DP, 추적 리프 축소, 프루닝 손실 보고, 최초 달성 PMF/CDF
- 정확 모드 시행/상태/메모리/분모 가드레일과 진행/취소 콜백
- CLI 검증/DP/exact/MC 명령과 WASM JSON API
- 블루 아카이브 및 하드 천장 프리셋
- Blockly 기반 Model IR 편집기, 즉시 검증 패널, 결과/CI 표

## 후속 마일스톤

- Exact DP의 최초 달성 흡수 모드
- 확률표 1천만 제어상태 초과 시 lazy cache
- `consumesTrial=true` 지급의 별도 시행 인덱스 전개
- snapshot의 GCHS/zstd/checkpoint 직렬화
- Rayon/Web Worker 병렬 MC와 자동 CI 폭 정지
- 전체 E002/E005/W003 정적 구간·만족가능성 분석
- Tauri 데스크톱 패키징
- 10개 이상 게임 프리셋 및 골든 파일
- 지급 의미론 4조합, 기하/음이항 해석해, IR 퍼징

## 이 환경에서 실행한 검증

- UI 단위 테스트: 통과
- TypeScript strict 타입 검사: 통과
- Vite 프로덕션 빌드: 통과
- 모든 Rust 파일 tree-sitter 문법 검사: 통과
- Rust `cargo build --workspace && cargo test --workspace` (2026-07-27 최신): 빌드 성공, 테스트 17/17 통과. 경고 1건(`compile.rs:230` `EntityDef.name` 미사용)

## 2026-07-27 감사 (설계문서 vs 코드 정합성)

`docs/DESIGN.md`/`CLAUDE.md`를 저장소에 반영한 직후, 문서의 주장을 실제 코드와 전수 대조했다. 결과는 `docs/DESIGN.md` §13에 반영했다. 요약:

- §13.1(스펙 차이), §13.2(미구현 진단) 표는 여전히 정확함을 확인 — 드리프트 없음
- 신규 발견(§13.5, 문서에 없던 것): `numeric: "exact"` 선택 시 `dp` 경로가 조용히 `ScaledF64`로 강등되고 결과 라벨도 `"scaled"`로 잘못 표시되는 버그, UI "정확" 옵션이 exact 엔진을 호출하지 않는 버그, `ExactResult`에 `clamp_events` 누락(절대 규칙 7 부분 위반)
- 코드 수정은 이번 라운드에 하지 않았다 — §13.5가 다음 작업 우선순위

## 2026-07-27 exact 경로 수정

- `run.numeric: "exact"`가 DP CLI/WASM에서도 BigInt exact 엔진으로 실행됨
- 웹 UI의 정확 모드가 `run_exact_json`을 직접 호출함
- exact 결과에 `numeric`, `clampEvents`를 포함하고 UI에도 보정 횟수를 표시함
- exact 디스패치·clamp 보고 코어 테스트와 UI 백엔드 선택 테스트 추가
- UI 테스트 4개, TypeScript strict 검사, Vite 프로덕션 빌드 통과
- 후속 Rust 변경은 현재 Codex 환경에 Rust 툴체인이 없어 `cargo test --workspace` 재실행 필요

## 2026-07-27 PR #2 리뷰 (Codex 수정 검증)

PR #2(`fix: wire exact backend`)를 이 환경에서 직접 체크아웃해 검증했다.

- `cargo build --workspace`: 성공, 신규 경고 없음 (기존 `EntityDef.name` dead-code 경고 1건만 유지)
- `cargo test --workspace`: **8/8 통과** (신규 exact 디스패치 테스트 포함) — Codex 환경에서 못 돌렸던 부분 해소
- `npx tsc --noEmit`, `npm test`(4/4): 통과
- 수동 스모크 테스트: `numeric: "exact"` 모델을 `dp` 커맨드로 실행 → `"numeric": "exact"` + BigInt 분자(`1,4,6,4,1`)/분모(`16`)로 이항분포와 정확히 일치. exact 강등 버그·UI 미배선 버그·`clamp_events` 누락이 모두 실제로 해결됐음을 확인
- 코드 수정 없이 문서만 갱신(`docs/DESIGN.md` §13.4에 검증 기록 추가) 후 push
- 다음 우선순위는 변경 없음: §13.3의 3대 핵심 테스트(MC↔DP 교차검증, exact↔ScaledF64 일치, 지급 전파)

## 2026-07-27 §13.3 핵심 검증 병합

- 병렬 구현의 §13.3 우선순위 1~3 검증을 함께 유지:
  - 10개 모델 각각 MC 10^6회 ↔ ScaledF64 DP Wilson 95% 교차검증
  - 실제 희귀 비율(`star3=0.03`, `pickup=0.007`) 모델의 MC 10^6회 교차검증
  - 중첩·동적 확률·전이·지급 모델의 exact ↔ ScaledF64 전 셀 상대오차 ≤ 1e-10 검증
  - `N=200, p=0.007` 전 셀과 `0.007^200` 극소확률을 유리수로 직접 비교
  - 200회 확정 픽업 지급 시 전체 `nStar3` 분포가 정확히 +1 이동하는지 검증
- 최초 교차검증에서 86셀 중 78셀이 이탈해 MC alias-table 버그를 발견했다. 한 bucket이 빈 상태에서 튜플 `pop()`이 다른 bucket 원소까지 버리던 루프를 수정하고 직접 회귀 테스트를 추가했다.
- Wilson 구간의 0회/전회 관측 경계에서 발생하던 부동소수점 반올림도 정확한 `0.0`/`1.0`으로 수정하고 회귀 테스트를 추가했다.
- 병합 전 작업 브랜치의 GitHub Actions: Rust 13/13, UI 4/4 통과.
- 병합 후 로컬 `cargo test --workspace`: **17/17 통과**. 기존 경고 `EntityDef.name` 미사용 1건 외 신규 경고 없음.
- §13.3 잔여: 지급 의미론 4조합, 기하/음이항 해석해, 프리셋 골든 파일, IR 퍼징.
