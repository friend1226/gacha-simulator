# 구현 상태

## 완료

- Cargo workspace: `gacha-core`, `gacha-cli`, `gacha-wasm`
- 정확 십진/분수/지수 리터럴 파서 (`0.007` → `7/1000`)
- `Prob` 계층과 `F64`, 비트 정규화 기반 `ScaledF64`
- 공통분모 BigInt exact DP (내부 루프 GCD 없음, 선택적 레이어 약분)
- Model IR v1 역직렬화, E002 음수 확률 범위 분석과 W003 조건 만족가능성을 포함한 구조화 진단
- 표현식 AST → 스택 바이트코드 및 유리수 평가
- 엔티티 트리 → 배타적 리프 분할과 `__other__` 생성
- 제어상태·시행별 정확 유리수 확률표 사전계산
- 제어 전이, 확정 지급, 제어상태 기반 동적 확률
- 재현 가능한 병렬 xoshiro256++ MC와 alias sampling, Wilson 95% 구간
- MC↔DP 10개 모델·희귀 비율 교차검증, exact↔ScaledF64 복합·극소확률 비교, 지급 의미론 4조합과 해석해 회귀 테스트
- `consumesTrial` 논리 시행 슬롯을 MC·ScaledF64 DP·ExactInt DP에 동일하게 구현
- 두 프리셋 SHA-256 골든과 결정적 IR 컴파일러/엔진 퍼징
- GitHub Actions 기반 Rust/UI CI
- 희소 DP, 추적 리프 축소, 프루닝 손실 보고, 최초 달성 PMF/CDF
- ExactInt DP 최초 달성 흡수 상태와 공통분모 BigInt PMF/CDF
- DP/Exact 공용 mixed-radix `u64` 상태 코덱
- native MC·DP·ExactInt의 결정적 Rayon 병렬 전개
- GCHS/zstd 스냅샷과 aggregate/checkpoint/full 정책
- 정확 모드 시행/상태/메모리/분모 가드레일과 진행/취소 콜백
- CLI 검증/DP/exact/MC 명령과 WASM JSON API
- 블루 아카이브 및 하드 천장 프리셋
- Blockly 기반 Model IR 편집기, 즉시 검증 패널, 결과/CI 표

## 후속 마일스톤

- 확률표 1천만 제어상태 초과 시 lazy cache
- Web Worker 병렬 MC와 자동 CI 폭 정지
- E005 순환 참조 정적 분석 보강
- Tauri 데스크톱 패키징
- 10개 이상 게임 프리셋 및 골든 파일

## 이 환경에서 실행한 검증

- UI 단위 테스트: 통과
- TypeScript strict 타입 검사: 통과
- Vite 프로덕션 빌드: 통과
- 모든 Rust 파일 tree-sitter 문법 검사: 통과
- Rust `cargo build --workspace && cargo test --workspace` (2026-07-27 최신): 빌드 성공, 테스트 33/33 통과. 경고 1건(`compile.rs` `EntityDef.name` 미사용)

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

## 2026-07-27 §13.3 병합 리뷰 검증

- 병합 커밋(`0200844`)을 이 환경에서 직접 체크아웃해 `cargo test --workspace` 17/17 통과를 재확인했다.
- 컨플릭트 해결 4개 파일에 잔여 마커 없음을 확인했다.
- 이전 리뷰에서 지적한 `report.rs`의 Wilson 하한 버그(`successes=0`일 때 정확히 0.0이 아닌 문제)가 병합된 수정으로 해소됐음을 재확인했다.
- 다음 우선순위: `docs/DESIGN.md` §13.1의 `consumesTrial` 미배선 해소 → §13.3 4번(지급 의미론 4조합) → 5~7번(해석해 대조, 프리셋 골든 파일, IR 퍼징) 순.

## 2026-07-27 §13.3 잔여 검증 완료

- `consumesTrial`을 세 엔진에 배선하고 지급 슬롯·최초 달성·마지막 시행 경계를 회귀 테스트로 고정했다.
- 남은 시행 예산을 초과해 적용되지 않는 `consumesTrial` 지급은 컴파일 경고 `W007`로 보고한다.
- 지급 의미론 4조합을 MC·ScaledF64 DP·ExactInt DP에서 비교했다.
- 기하분포·음이항분포 closed-form 대조, 프리셋 2종 골든, 결정적 IR 퍼징을 추가했다.
- 로컬 `cargo test --workspace`: **27/27 통과**. 다음 정확성 우선순위는 E002/W003 코어 진단과 exact 최초 달성 흡수 모드다.

## 2026-07-27 코어 진단 및 exact 흡수 상태 완료

- E002를 코어 affine 범위 분석과 제한된 exact 상태 전수 평가로 구현해 CLI/WASM 경로에서도 음수 확률을 블록 ID와 함께 거부한다.
- W003은 엔티티 카운트를 배타적 리프로 전개해 포함관계·시행 상한·영확률 도달 불가를 정적으로 경고하며, 지급으로 가능한 조건은 경고하지 않는다.
- ExactInt DP가 최초 달성 질량을 흡수하고 exact PMF/CDF·미달성 질량을 반환한다. 지급이 소모한 논리 시행 번호와 레이어 약분도 검증했다.
- 로컬 `cargo test --workspace`: **33/33 통과**. Lazy 확률표 캐시는 실제 한계 초과 모델이 필요할 때 착수하며 M8 항목은 계속 보류한다.

## 2026-07-27 §13.3 후속(W007) 및 §13.2 코어 진단 리뷰 검증

- W007 커밋을 체크아웃해 `cargo test --workspace` 27/27 재확인, 경고 판정 로직이 런타임 소모 판정과 일치함을 코드로 확인했다.
- E002/W003/exact 흡수 커밋을 체크아웃해 `cargo test --workspace` 33/33 재확인, affine 상관항 소거·W003 심플렉스 분석·exact 흡수 질량보존 검사를 코드 레벨로 검증했다.
- 코드 결함 없음. CLAUDE.md 고정 순서(§1~4)가 모두 완료됐고, 다음은 M8 성능 작업 또는 프리셋 확장 등 사용자 결정이 필요하다.

## 2026-07-28 M8 Phase 1 — mixed-radix 상태 패킹

- DP와 ExactInt에 중복돼 있던 `State { control, counts }`를 제거하고 공용 `StateCodec`과 `HashMap<u64, P>` 레이어로 교체했다.
- 제어 차원을 하위 자릿수에 배치해 확률표 제어 인덱스를 나눗셈 한 번으로 구하며, 상태별 전이 버퍼를 재사용한다.
- 지급량까지 포함한 카운트 상한으로 패킹 공간을 계산하고 `u64` 초과를 명시적으로 거부한다.
- 로컬 `cargo test --workspace`: **35/35 통과**, 프리셋 골든 SHA-256 결과 불변.
- release/기본 프루닝 실측(180 제어상태, N=1,000, 3개 리프): 픽업 단독 4,923ms, `pickup × star3__self` 결합 194,856ms. §6.6 목표(300ms/8s)는 아직 미달이며 Phase 3 병렬 전개와 후속 프로파일링이 필요하다.

## 2026-07-28 M8 Phase 2 — MC 병렬화

- native 기본 기능에 Rayon을 연결하고 4,096 run 고정 청크마다 xoshiro256++ `jump()`로 독립 스트림을 만든다.
- 청크 번호와 RNG 스트림을 스레드 스케줄링에서 분리해 동일 시드 결과가 1/4 스레드에서 히스토그램·최초 달성 배열까지 완전히 일치한다.
- 블루 아카이브 프리셋 MC 100만 회 release 실측: 1스레드 5,008ms, 4스레드 1,823ms(약 2.75배).
- 로컬 `cargo test --workspace`: **36/36 통과**. 기존 경고 `EntityDef.name` 미사용 1건 외 신규 경고 없음.

## 2026-07-28 M8 Phase 3 — DP 레이어 병렬 전개

- ScaledF64/F64와 ExactInt 레이어를 256셀 고정 청크로 병렬 전개하고, 결정적 `FxHashMap` 로컬 레이어를 청크 순서대로 병합한다.
- 프루닝·진행 콜백·ExactInt 레이어 약분과 질량 보존 단언은 병합 이후 단일 스레드 배리어에서 수행한다.
- ExactInt 결과는 1/4 스레드에서 경과 시간을 제외한 JSON 바이트가 완전히 동일하며 프리셋 골든도 불변이다.
- release/기본 프루닝 4스레드 실측(Phase 1과 동일 모델): 픽업 단독 2,107ms(1스레드 5,372ms), 결합 84,254ms. Phase 1 대비 약 2.3배 개선됐지만 §6.6 목표는 여전히 미달이다.
- 로컬 `cargo test --workspace`: **37/37 통과**. 기존 경고 1건 외 신규 경고 없음.

## 2026-07-28 M8 Phase 4 — GCHS 스냅샷

- IR canonical JSON SHA-256을 모델에 고정하고 GCHS v1 헤더, 정렬 상태 delta+varint, zstd 레벨 3 본문을 구현했다.
- `aggregate`(기본, 매 레이어 주변분포), `checkpoint`(1/2/5 로그 간격+핀), `full`(매 레이어) 정책을 ScaledF64/F64/ExactInt에 연결했다.
- ExactInt 스냅샷은 레이어 공통분모와 BigInt 분자를 보존한다. 로더는 모델 해시 불일치를 명확히 거부한다.
- 200MB 사전 경고와 사용 가능 메모리 50% 거부, `full` 명시 확인을 구현했다. CLI는 `gacha snapshot <MODEL> <OUTPUT> --policy ...`로 노출한다.
- N=1,000 결합 추적 휴리스틱 용량 스모크 범위는 aggregate 1~20MB, checkpoint 10~200MB, full 1~10GB로 고정했다.
- `restore_dp_snapshot`은 핀 레이어를 재계산해 복원한다. 가장 가까운 체크포인트에서 재개하려면 최초 달성 누적 질량까지 직렬화해야 하므로 §13.1 후속 차이로 기록했다. WASM/UI 노출도 후속 범위다.
- 로컬 `cargo test --workspace`: **41/41 통과**, `--no-default-features` 코어 단위 테스트 20/20 통과. TypeScript strict 검사와 UI 테스트 4/4, CLI 도움말 스모크도 통과했다.

## 2026-07-28 M8 §6.6 성능 목표 달성

- `StateCodec::decode_into`와 청크별 작업 버퍼로 DP/Exact 상태마다 발생하던 control/count `Vec` 할당을 제거했다.
- 지급·흡수 조건이 없는 경로는 컴파일된 제어 전이표와 mixed-radix 스트라이드로 패킹된 `u64`를 직접 갱신한다. 복합 지급·조건 경로는 기존 의미론을 유지한다.
- 확률표가 모든 제어상태에서 동일하면 제어 자릿수를 정규화해 결과에 영향을 주지 않는 180개 제어상태의 중복 전개를 제거한다. 동일 분포 회귀 테스트를 추가했다.
- 병렬 레이어 병합을 고정 이진 트리로 바꿔 스레드 수와 무관한 결정성을 유지하면서 최종 단일 스레드 병합 병목을 줄였다.
- release/4스레드 5회 중앙값(N=1,000, 제어 180상태): 픽업 단독 **3ms**(목표 <300ms), `pickup × star3__self` **178ms**(목표 <8s). 기존 2,107ms/84,254ms 대비 각각 약 702배/473배 개선됐다.
- 위 정식 측정 모델은 선언된 제어상태와 무관하게 확률표가 동일하다. 확률이 실제 pity 상태에 의존하도록 만든 보수적 스트레스 변형의 3회 중앙값은 484ms/18,712ms로, 의미상 필요한 180상태를 제거할 수 없어 목표를 초과했다. 현재 목표 표와의 차이 및 후속 sharded-layer 필요성을 DESIGN §6.6에 함께 기록했다.
- MC jump 스트림을 순차 사전계산해 스트림 준비 복잡도를 O(n²)에서 O(n)으로 낮췄고, snapshot clippy 경고 2건도 정리했다.
- 로컬 `cargo test --workspace`: **42/42 통과**. 결정성 테스트와 프리셋 골든 파일 모두 불변이며 기존 `EntityDef.name` 경고 1건 외 신규 경고 없음.
