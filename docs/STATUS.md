# 구현 상태

## 완료

- Cargo workspace: `gacha-core`, `gacha-cli`, `gacha-wasm`, `gacha-tauri`
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
- Tauri Windows 설치 프로그램 및 3플랫폼 빌드 파이프라인
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
- `StateCodec`의 패킹 상태 고속 갱신 헬퍼에 디버그 범위 검증을 추가해 count 자릿수 carry와 잘못된 control 인덱스를 즉시 검출한다.
- 하드-pity 스트레스 케이스는 PowerShell에서 `$env:RAYON_NUM_THREADS=4; cargo run --release -p gacha-core --example m8_hard_pity_bench`로 재현할 수 있다. 기본 3회 측정의 원시 표본·중앙값·목표 달성 여부를 JSON으로 출력한다.
- 로컬 `cargo test --workspace`: **44/44 통과**. 결정성 테스트와 프리셋 골든 파일 모두 불변이며 기존 `EntityDef.name` 경고 1건 외 신규 경고 없음.

## 2026-07-28 M8 종료

이번 라운드로 M8을 마무리한다. 하드-pity 케이스의 §6.6 목표 미달(픽업 단독 1.6배,
결합 2.3배)은 재현 벤치마크와 함께 유지하되, sharded layer 등 자료구조 변경이 필요한
후속 성능 과제로 이관한다.

## 2026-07-29 E005 정합성 및 Tauri Windows 스켈레톤

- DESIGN §3.4의 E005를 실제 구현인 “`probRule`이 정의되지 않은 엔티티를 대상으로 함”으로 재정의했다. 기존 순환 참조 항목은 현재 IR이 엔티티 확률 참조를 표현하지 못하므로 폐기하고 §13에 조사 근거를 기록했다.
- E005가 오류 심각도와 원래 `blockId`를 보존하는 회귀 테스트를 추가했다.
- `gacha-tauri` 크레이트를 신설하고 Tauri 2 명령이 WASM을 거치지 않고 `gacha-core`의 validate/DP/exact/MC 경로를 직접 호출하도록 연결했다.
- React/Blockly UI는 공용 엔진 어댑터만 추가했다. Tauri에서는 IPC를, 일반 웹에서는 기존 WASM 모듈을 선택하므로 화면 코드는 분기하지 않는다.
- 이번 범위는 `bundle.active=false`인 Windows 실행 파일까지다. 설치 프로그램, dmg/AppImage, 3플랫폼 자동 빌드, 프리셋 팩과 M10 사용자 문서는 후속으로 남긴다.
- `cargo test --workspace`: **47/47 통과**, UI 테스트 **4/4 통과**, Vite 프로덕션 빌드 성공.
- `npm run tauri:build:windows`: 성공. `target/release/gacha-tauri.exe` 생성 확인.

## 2026-07-29 CI 수정 — gacha-tauri Linux 빌드 제외

- `gacha-tauri`를 워크스페이스에 추가한 뒤 GitHub Actions Rust CI(`ubuntu-latest`)가 `glib-sys`/`gobject-sys` 빌드 실패로 깨졌다. Tauri의 Linux 백엔드가 GTK/WebKit 시스템 라이브러리를 요구하는데 러너에 없기 때문이다.
- 이번 라운드 Tauri 범위가 Windows 실행 파일까지였던 것과 일관되게, CI도 `cargo test --workspace --exclude gacha-tauri`로 해당 크레이트를 제외했다. Linux/macOS 패키징에 착수할 때 `apt-get install libgtk-3-dev libwebkit2gtk-4.1-dev libayatana-appindicator3-dev librsvg2-dev` 같은 사전 설치 단계를 추가하고 이 제외를 재검토해야 한다.
- 로컬 재현: `cargo test --workspace --exclude gacha-tauri` 통과 확인.

## 2026-07-29 preset_goldens CRLF/LF 불일치 수정

- 위 수정 뒤에도 CI가 `preset_goldens` 두 테스트에서 실패했다. `resultSha256`(계산 결과)은 완전히 일치했고 `presetSha256`(프리셋 원본 파일 바이트 해시)만 달랐다 — 원인은 Windows `core.autocrlf=true`가 `presets/*.json`을 로컬 체크아웃 시 CRLF로 바꿔서, 그 상태로 생성된 골든 해시가 LF로 체크아웃되는 Linux CI와 어긋난 것이었다.
- `.gitattributes`에 `presets/**/*.json text eol=lf`를 추가해 모든 환경에서 LF 체크아웃을 강제했다. git이 저장한 blob 자체는 이미 LF였으므로 `presets/*.json` 내용은 바뀌지 않았고, `presets/golden/*.json`의 `presetSha256` 필드만 LF 기준 값으로 갱신했다.
- 로컬에서 LF로 재체크아웃한 뒤 재현한 해시가 CI 실패 로그의 `left`(실제 계산값)와 정확히 일치함을 확인하고 반영했다. `cargo test --workspace --exclude gacha-tauri` 전체 통과.

## 2026-07-29 웹(WASM) 실행 경로 크래시 수정 — `Instant::now()`

사용 문서를 작성하려고 실제 브라우저에서 웹 UI를 처음부터 끝까지 구동해보다가 발견했다.
지금까지 Rust 테스트는 전부 네이티브로만 돌았고 UI 테스트는 wasm 호출을 mock 처리하고
있어서, 컴파일된 `.wasm` 바이너리를 실제 브라우저 JS 엔진에서 실행해본 적이 이번이
처음이었다.

- **증상**: `validate_model`(컴파일만)은 정상 동작하지만 `run_dp_json`/`run_exact_json`/`run_mc_json`은
  가장 단순한 모델(상태 없는 동전 던지기)로도 예외 없이 `RuntimeError: unreachable`로 크래시했다.
  즉 웹 UI에서 "DP 실행"/"MC 실행" 버튼은 지금까지 한 번도 정상 동작한 적이 없었을 가능성이 높다.
- **원인**: `engine_dp.rs`/`engine_exact.rs`/`engine_mc.rs`가 실행 시작 시점에 공통으로
  `std::time::Instant::now()`를 호출하는데(경과 시간 계산용), `wasm32-unknown-unknown` 타겟에는
  표준 시계 소스가 없어 이 호출이 트랩을 일으킨다. M8 이전부터 있던 코드라 이번 라운드의
  회귀는 아니다.
- **수정**: `[target.'cfg(target_arch = "wasm32")'.dependencies]`에 `web-time`을 추가하고,
  세 엔진 파일에서 wasm32 타겟일 때만 `web_time::Instant`를 쓰도록 조건부 임포트했다
  (네이티브/Tauri 경로는 그대로 `std::time::Instant`).
- **검증**: `wasm-pack build` 재빌드 후 실제 브라우저(`npm run preview`와 `npm run dev` 양쪽)에서
  블루 아카이브 프리셋으로 DP·MC를 직접 클릭해 실행했다. DP 484개 셀, MC 119개 셀이 정상
  반환되고 MC의 Wilson 95% 구간이 DP 값을 포함함을 확인했다.
- 별개로 `npm run dev`에서 wasm 모듈을 `ui/src/`로 옮겨 import하려는 시도는 Vite 프로덕션
  빌드가 `public/`이 아닌 `@vite-ignore`된 동적 import 대상을 `dist/`에 복사하지 않는 문제를
  일으켜 되돌렸다 — `public/wasm` 배치는 그대로 유지한다. `npm run dev`에서 계산 버튼을
  누르면 Vite 개발 서버가 `public/` 자산의 동적 import를 거부하는 별도의 알려진 제약이
  여전히 남아 있다(§Vite 공식 문서: public 자산은 소스 코드에서 import할 수 없음). 이 경로를
  테스트하려면 `npm run build && npm run preview`를 사용해야 한다 — README/사용 문서에 반영.
- `cargo test --workspace --exclude gacha-tauri` 전체 통과, `ui` `npx tsc --noEmit`/`npm test`
  4/4 통과, `npm run build` 후 `dist/wasm/` 정상 생성 확인.

## 2026-07-29 M9 완료 — 집계 변수, 시행 시리즈, 사용자 결과 뷰

- Model IR v2에 `role: accumulator`, 상한·표시명·리프별 갱신·clamp 정책을 추가하고
  v1 입력은 계속 허용했다. 갱신은 제어/시행 의존성을 접은 사전계산 테이블을 사용하며,
  packed state의 제어·집계·리프 카운트 구획을 분리했다.
- 자동 리프 카운터와 같은 집계 변수는 W008로 상태 없는 파생 축으로 강등한다.
  E009로 사용자의 `role: stat` 선언을 별도 거부하고, accumulator saturate는
  `accumulatorClampEvents`로 MC/Scaled/Exact 결과에 보고한다.
- `trialSeries`의 `none`/`marginal`/`checkpoints`를 세 엔진에 배선했다. 주변분포는
  명시적 시행 번호를 가지며 MC 셀은 반복 수 기준 Wilson 구간을 함께 반환한다.
- UI를 모델/결과/도움말/설정 탭으로 분리했다. 결과 탭은 행·열·집계·필터 피벗,
  1차원 SVG 막대와 2차원 히트맵, DP/MC 비교, 시행별 추이, 첫 달성 분포,
  CSV/JSON 내보내기와 재현 메타데이터를 제공한다.
- Blockly 모델 컨테이너와 집계 변수 블록을 추가하고 JSON→블록 복원에서 확률 규칙,
  전이, 트리거, 조건을 함께 복원한다. 프리셋은 `presets/*.json`을 빌드 타임에 읽고
  출처 배지를 표시한다.
- 설정은 버전 있는 localStorage 스키마로 효과음 볼륨, 기본 엔진 값, 확률 표기와 표
  행 제한을 저장한다. 모델도 자동 저장하고 JSON 열기/저장을 지원한다.
- 집계/시리즈 코어 회귀 5개와 피벗/설정/진단 라벨/첫 달성 UI 단위 테스트 6개를 추가했다. 프리셋은
  v2로 올렸으며 골든의 계산 결과 SHA-256은 그대로이고 원본 프리셋 SHA-256만 갱신했다.
- 남은 차이는 DESIGN §13.1에 기록했다: 일반 파생 accumulator 분류와 WASM
  Web Worker 진행률/취소.
- `cargo test --workspace --exclude gacha-tauri` 전체 통과, UI strict 타입 검사와
  vitest 10/10, Vite 프로덕션 빌드가 통과했다.
- 새 WASM을 빌드해 실제 브라우저 프로덕션 미리보기에서 블루 아카이브 DP
  484셀/시행 시리즈(41ms), MC 10만 회, 2개 비교 히트맵, DP 정확 표기와 MC Wilson
  셀 상세, 도움말 18개 진단, 설정 6개 항목, 800px 반응형 무가로스크롤을 확인했다.
- v1 모델은 시행 시리즈 기본을 `none`으로 보존하고 v2는 `marginal`로 적용해 하위
  호환 성능 회귀를 막았다. 4스레드 하드-pity 벤치 3회 중앙값은 픽업 315ms,
  결합 18,314ms로 기존 STATUS의 484ms/18,712ms 스트레스 측정 범위와 동등 이상이다.

## 2026-07-29 M9 리뷰 후속

- accumulator 사전계산 테이블을 전개 전에 합산한다. 500,000 엔트리 이상은 축별
  크기를 포함한 `W009`, 합계 10,000,000 초과는 OOM 방지용 `E010` error로 처리한다.
- §9.1 MC↔DP 10^6회 Wilson 교차검증에 제어 의존 accumulator packed fast path와
  `consumesTrial` 지급 slow path 모델을 추가했다.
- UI 피벗은 엔진의 십진 `display`를 BigInt 가수·지수로 합산하고 Exact 분수도 같은
  표기로 변환해 `1e-431`급 확률을 0으로 잃지 않는다. 히트맵은 Map 조회를 사용하고
  행·열 모두 표시 상한과 생략 안내를 적용했으며 대형 배열 `Math.max(...values)`
  호출을 제거했다.
- 조건 블록 삭제 시 잔존하던 `run.condition`, 1100px 이하에서 사라지던 파일 작업
  버튼, `EntityDef.name` dead-code 경고도 함께 정리했다.
- 검증: Rust 53/53, UI 11/11, strict 타입 검사와 Vite 빌드 통과. 골든
  `resultSha256` 불변. 최신 WASM 프로덕션 미리보기에서 블루 아카이브 DP 484셀,
  MC 10만 회 119셀을 실행했고, Exact 극소 모델의 201×201 히트맵에서
  `1.04618382913e-431` 표시와 브라우저 콘솔 오류 0건을 확인했다.

## 2026-07-29 Netlify 배포와 모바일 필수 수정

- `main` push와 수동 실행을 지원하는 Netlify 배포 워크플로를 추가했다. Rust/UI
  테스트 뒤 프리빌트 `wasm-pack`으로 WASM을 만들고, Vite 빌드 후
  `dist/wasm/gacha_wasm_bg.wasm` 존재를 확인한 경우에만 프로덕션 배포한다.
- Netlify 자체 빌드는 중단하고 GitHub Actions가 만든 `ui/dist`만 올린다. 해시가
  있는 `/assets/*`는 1년 immutable, 파일명이 고정된 `/wasm/*`와 `/index.html`은
  매 요청 재검증하도록 `_headers`를 배포 산출물에 포함했다.
- 모바일 그리드 아이템의 `min-width: auto` 전파를 끊어 결과 히트맵이 문서 전체를
  밀지 않도록 했다. 620px 이하에서도 프리셋 선택기와 초기화/저장/열기 버튼을
  유지하며, iOS Safari용 `100dvh`와 `100vh` 폴백을 함께 적용했다.
- 검증: Rust 53/53, UI 11/11, strict 타입 검사, Vite/WASM 프로덕션 빌드 통과.
  WASM 디렉터리를 임시 제외한 빌드에서 Vite 자체는 성공하지만 필수 산출물 검사가
  실패하는 것을 확인했다. 골든 `resultSha256`은 그대로다.
- 실제 프로덕션 미리보기를 375×812로 열고 기본 프리셋 Exact DP(201셀)와 MC
  10만 회를 실행했다. 문서 `scrollWidth === clientWidth`, 히트맵 자체 가로 스크롤,
  네 가지 헤더 작업 노출, 동적 뷰포트 높이와 콘솔 오류 0건을 확인했다. 620px에서도
  프리셋 선택기가 표시된다.
- 폰의 Blockly 편집 정책은 소유자 결정 전까지 기존 동작을 유지한다. Blockly 지연
  로딩은 현재 시작 탭 구조에서 즉시 다시 로드돼 효과가 불분명하므로 보류했다.
- `vite.config.ts`에 React 플러그인을 등록했다. 프로덕션 빌드가 통과하고 실제 개발
  서버 응답에 React Refresh 프리앰블과 컴포넌트 변환이 모두 포함되는 것을 확인했다.

## 2026-07-29 Netlify 프로덕션 배포

`.github/workflows/deploy.yml`로 GitHub Actions에서 빌드하고 완성된 `ui/dist`만
Netlify에 올리는 파이프라인을 구성해 첫 배포를 완료했다. 프로덕션 URL은
`https://gacha-simul.netlify.app`이다.

- WASM 산출물은 저장소에 없으므로(`ui/public/wasm/.gitignore`가 `*`) 호스팅 서비스가
  저장소를 클론해 `npm run build`만 돌리면 **빌드는 성공하는데 계산이 안 되는 사이트**가
  나간다. 실제로 `ui/public/wasm`을 지우고 빌드하면 exit 0으로 성공하면서 `dist/`에
  wasm이 빠지는 것을 확인했다. 이를 막기 위해 배포 직전 `dist/wasm/gacha_wasm.js`와
  `gacha_wasm_bg.wasm` 존재를 검사하며, 둘 중 하나만 없어도 배포가 중단되는 것을
  각각 확인했다.
- 첫 실행은 `Type-check UI`에서 실패했다. `npm --prefix ui exec`는 npm의 패키지 해석
  위치만 바꾸고 작업 디렉터리는 저장소 루트로 두므로 `tsc`가 `tsconfig.json`을 찾지
  못해 도움말을 출력하고 종료 코드 1로 끝났다. 같은 워크플로의 `npm --prefix ui ci`와
  `npm --prefix ui run build`는 npm 스크립트가 cwd를 패키지로 재설정해 통과했기 때문에
  `exec` 단계만 깨졌다. UI 단계 전부를 `working-directory: ui`로 고정해 해소했다
  (`033d473`).
- 라이브 사이트 검증: 데스크톱 1280px에서 DP 484셀 36ms, MC 10만 회 3,129ms(seed 42,
  모델 해시 DP와 일치), 모바일 375px에서 DP 484셀 21ms. 두 폭 모두 가로 넘침 없고
  히트맵은 자체 스크롤하며 콘솔 오류 0건이다.
- 응답 헤더 확인: `/wasm/*`는 `application/wasm`과 `max-age=0, must-revalidate`,
  `/assets/*`는 `max-age=31536000, immutable`, `/index.html`은 재검증이다. 이름이
  고정된 wasm에 장기 캐시가 걸리지 않아 재배포가 즉시 반영된다. Netlify가 wasm까지
  brotli로 자동 압축한다.
- Netlify 사이트의 Build status를 `stopped`로 두어도 CLI 프리빌트 배포는 정상
  게시되는 것을 확인했다. Netlify 자체 빌드는 `netlify.toml`의 `ignore = "exit 0"`으로도
  이중 차단한다.
- Node.js 20 지원 종료 경고를 없애기 위해 `actions/checkout@v7.0.1`과
  `actions/setup-node@v7.0.0`으로 갱신했다. 2022년 이후 새 릴리스가 없는
  `jetli/wasm-pack-action`은 `taiki-e/install-action@v2.85.3`과
  `wasm-pack@0.15.0` 조합으로 교체했다.

## 2026-07-29 dev 브랜치 프로세스와 워크플로 갱신

- `dev`에서 작업 브랜치를 분기해 완료 후 `dev`로 합치고, 소유자가 일단락을 판단할
  때만 `dev`를 프로덕션 브랜치인 `main`으로 병합하는 전달 절차를 도입했다.
- CI의 push 대상에 `dev`를 추가했다. push 워크플로는 변경 파일이 전부 `docs/**` 또는
  Markdown일 때 건너뛰지만, 브랜치 보호 필수 체크가 사라지지 않도록
  `pull_request`에는 경로 필터를 적용하지 않았다.
- PR #3의 push와 pull request 이벤트에서 Rust 53개와 UI 11개 테스트가 모두
  통과했다. `main` 병합 뒤 CI와 Netlify 프로덕션 배포도 성공했으며, 새 액션과
  wasm-pack 설치 방식으로 WASM 빌드·필수 산출물 검사·게시까지 완료했다.

## 2026-07-29 B-1 — 웹 계산 Worker와 취소

- 웹의 DP·Exact·MC WASM 호출을 모듈 Worker로 옮겼다. Worker는 초기화한 WASM을
  재사용하며, 취소 시 `terminate()`하고 다음 실행에서 새 Worker를 만든다. Tauri는
  기존 네이티브 `invoke` 경로를 그대로 사용한다.
- 실행 세대 번호를 두어 취소된 요청의 늦은 결과·`finally`가 새 실행 상태를 덮지
  못하게 했다. 실행 중 웹에서만 취소 버튼을 노출한다.
- UI 타입 검사와 13개 테스트, 프로덕션 빌드가 통과했고 별도 Worker 청크 생성을
  확인했다. Vite 개발 서버는 §3.2의 기존 public WASM 동적 import 제약까지 정상
  도달했고, 프로덕션 미리보기에서는 100만 회 MC 중 도움말 탭 전환 0.28초,
  취소 0.27초로 메인 UI가 계속 반응했다.
- 취소 후 10만 회 MC 재실행은 119셀·seed 42·Wilson 95% 구간을 반환했다. 같은
  모델의 Exact DP와 MC 모델 해시 `29887321c2e5`가 일치했고, Worker 재생성 뒤
  실행 시간도 기존 실측과 같은 약 3.2초였다. 375px에서 문서 가로 넘침과 브라우저
  콘솔 오류는 없었다.
- `npm run tauri:dev`는 Vite 서버와 Rust 빌드를 거쳐 `gacha-tauri.exe`를 실제
  기동했다. 프로세스가 기존 네이티브 앱 경로로 시작하는 것을 확인한 뒤 종료했다.

## 2026-07-29 B-2 — 웹 계산 진행률

- 기존 DP·Exact·MC 진행 콜백을 JavaScript 함수로 전달하는 WASM ABI를 추가하고,
  Worker 응답 프로토콜로 완료량과 전체량을 메인 UI에 전달한다. 실행 중에는
  네이티브 `progress` 요소와 완료 수·백분율을 함께 표시한다.
- 동기 WASM 실행 중에는 Worker 이벤트 루프가 취소 메시지를 받을 수 없으므로,
  협조적 취소 대신 B-1의 `terminate()` 방식을 유지했다. 실제 브라우저에서 100만 회
  MC 진행률이 270,336(27%)에서 294,912(29%)로 증가했고 취소는 0.37초에 완료됐다.
  취소 후 새 Worker에서 10만 회 MC를 재실행해 119셀·seed 42·모델 해시
  `29887321c2e5` 결과를 확인했다.
- Rust 워크스페이스 53개 테스트와 골든 해시, UI 타입 검사와 13개 테스트,
  wasm-pack 및 프로덕션 빌드가 모두 통과했다. 이로써 §13.1의 웹
  진행률·취소 차이를 해소했다.

## 2026-07-29 C — 엔진 진단 한국어화와 사전 경고

- 실행 오류의 진단 코드를 줄 단위로 추출해 한국어 제목과 해결 방법을 결과 화면에
  표시한다. 각 항목은 도움말의 같은 코드로 이동하며, 접을 수 있는 상세 영역에는
  영문 원문을 그대로 보존한다. 등록되지 않은 미래 코드도 원문으로 표시한다.
- 로컬 검증기에 코어와 같은 500,000/10,000,000 엔트리 기준의 accumulator 테이블
  추정을 추가했다. 실제 브라우저에서 6천만 엔트리 모델이 실행 전에 E010으로
  차단되고 축별 크기와 해결 방법이 한국어로 표시되는 것을 확인했다.
- 코어는 E010이 발생하면 하위 수준 W009를 함께 내지 않는다. 800,400 엔트리는
  W009만, 60,001,000 엔트리는 E010만 발생하는 코어·UI 테스트를 추가했다.
- 엔진 전용 E007 재현 모델로 복수 진단의 한국어 목록, 영문 원문 상세, 도움말 E007
  이동을 프로덕션 미리보기에서 확인했다. UI 16개 테스트와 타입 검사·빌드가
  통과했고 375px에서 가로 넘침이 없었다.

## 2026-07-29 D 되돌림 — 잘못 모델링한 Arknights 프리셋

- `arknights-guaranteed-ten-roll` 프리셋과 UI 등록을 되돌렸다. 기록한 출처 URL은
  해당 내용을 재현 가능하게 열 수 없었고, 실제 10연 보장은 6★ 확정이 아니라 앞
  9회에 5★/6★이 없을 때 10번째를 5★ 이상으로 만드는 조건부 규칙이다.
- 삭제한 모델은 IR에 조건부·확률적 보장 슬롯을 표현할 방법이 없다는 문제를 무시하고
  일반 확률 9회 뒤 6★를 무조건 지급했다. 그 결과 `P(6★=1)=0.98^9`,
  `E[6★]=1.18`인 잘못된 분포를 만들었다. 핵심 실패 원인은 단순히 출처를 찾지
  못했다는 것이 아니라, 표현할 수 없는 메커니즘을 무조건 지급으로 근사한 것이다.
- 재작업하려면 먼저 조건부 지급 또는 보장 슬롯 재정규화를 정확히 표현할 수 있는지
  IR을 조사·확장해야 한다. 출처·확률·수작업 기대값 검증을 소유자에게 먼저 보고하고
  승인받은 뒤에만 새 프리셋을 추가한다.

## 2026-07-29 FIX A — Worker 배포 형식과 복구 경로

- Vite Worker 출력을 ES 모듈로 고정했다. `npm run build` 산출물의 Worker 생성자가
  `new Worker(..., {type:"module"})`을 유지하며, Worker 청크에서도 WASM 동적
  import와 실패 시 캐시 초기화가 포함된 것을 확인했다.
- 프로덕션 미리보기에서 Exact DP는 진행률 163/200을 거쳐 20,301셀·6,166ms,
  모델 해시 `29887321c2e5`로 완료됐다. MC 100만 회는 1%에서 2%로 진행했고,
  취소는 365ms에 완료됐다. 취소 후 10만 회 재실행은 119셀·2,100ms·seed 42와
  같은 모델 해시를 반환했다.
- 브라우저 개발자 로그는 0건이었고 375px에서 `clientWidth=360`,
  `scrollWidth=360`이었다. Rust 53개와 기존 프리셋 골든, UI 16개 테스트,
  타입 검사와 프로덕션 빌드가 모두 통과했다.

## 2026-07-29 FIX B — Blockly ↔ IR 무손실 왕복

- `loadIr`가 블록으로 표현하지 못하는 엔티티·상태 변수·확률 규칙·전이·트리거·
  최초 달성 조건을 경로와 함께 반환하고, 워크스페이스별로 보관한다.
  `workspaceToIr`는 블록으로 편집한 항목과 이 보관 항목을 원래 순서로 합쳐
  블록 이동이나 편집만으로 일반 IR 규칙이 조용히 삭제되지 않게 했다.
- 미지원 항목은 검증 패널에 개수·IR 경로·설명과 `IR JSON 열기` 버튼으로 계속
  표시한다. `role: "stat"`, 복합 accumulator 갱신, 일반 condition은 편집 범위
  밖에 두되 그대로 보존한다.
- 천장 모델의 기본형을 직접 편집할 수 있도록 `not: {leafOf}` 전이, `변수 + 상수`
  전이 값, 리터럴 `then` 하드 실링과 선형 증가량, 트리거의 `consumesTrial` /
  `appliesTransitions` 체크박스를 추가했다.
- `import.meta.glob("../../presets/*.json")`으로 `presets/`의 모든 파일을 자동
  순회하는 왕복 테스트를 추가했다. `simple-pity`의 하드 실링과 두 전이가
  보존되고, 비기본 트리거 플래그와 미지원 stat/일반 condition 보존까지 포함해
  UI 테스트 20/20이 통과했다.
- 실제 프로덕션 미리보기에서 `simple-pity` 루트 블록을 드래그한 뒤 IR JSON의
  전이가 2개이고 두 번째가 `not leafOf rare`일 때 `pity + 1`인 것을 확인했다.
  미지원 규칙 2개를 넣으면 `stateVars[1]`과 `run.condition` 경고가 표시됐다.
  브라우저 개발자 로그는 0건이었고 375px에서 `clientWidth=360`,
  `scrollWidth=360`이었다.
- `cargo test --workspace --exclude gacha-tauri`는 53/53과 두 프리셋 골든을
  135.7초에 통과했다. `npx tsc --noEmit`, UI 20/20, `npm run build`도 통과했고
  프로덕션 빌드는 1,602개 모듈과 ES module Worker 청크를 생성했다.

## 2026-07-29 FIX C — 조건부 확률 슬롯 조사 결론

- 조건부·확률적 보장 슬롯은 새 트리거 조건 없이 현재 IR의 제어 변수와 시행 조건부
  `probRules`로 표현할 수 있다. `highSeen`을 5★/6★ 결과 전이에서 설정하고,
  `trial == 10 && highSeen == 0`인 확률표 항목만 5★ 이상으로 제한한다.
- 확률 표현식은 제어 상태×1부터 시작하는 시행 번호별로 사전 계산되므로 엔진
  내부 루프 표현식 평가도 추가되지 않는다. 별도 재현 모델의 Exact 결과는
  `controlStates=2`, `P(5★도 6★도 없음)=0`, 모든 셀의 counts 합 10,
  `E[6★]=0.26973568802`였다.
- 따라서 `irVersion`을 올리지 않고 `Trigger.when`도 추가하지 않는다. 조건 충족 시
  고정 리프를 추가 지급해야 하는 실제 프리셋이 생길 때만 별도 기능으로 재검토한다.

## 2026-07-29 FIX B 후속 — 왕복 불변식과 경고 갱신

- 모든 프리셋 테스트는 “전부 블록으로 표현 가능”이 아니라
  `loadIr → workspaceToIr` 구조적 충실도만 단언한다. 일반 시행 조건부 확률 규칙도
  미지원 항목으로 감지하면서 원문 그대로 왕복하는 회귀 테스트를 추가했다.
- 사용자가 미지원 `run.condition`을 지원 조건 블록으로 대체하면 보존 원문과 경고
  항목을 함께 제거한다. 실제 프로덕션 미리보기에서 배너가 2건에서 1건으로 즉시
  줄었고, 새 `nPickup ≥ 2` 조건과 남은 `legacyStat` 보존을 IR에서 확인했다.
- 트리거의 생략 가능한 `amount=1`, `consumesTrial=false`,
  `appliesTransitions=true`를 구조 비교 전에 정규화해 의미가 같은 프리셋이
  표기 차이만으로 실패하지 않게 했다.
- `cargo test --workspace --exclude gacha-tauri`는 53/53과 프리셋 골든 2/2를
  117.5초에 통과했다. `npx tsc --noEmit`, UI 23/23, 1,602개 모듈의
  프로덕션 빌드도 통과했다. 브라우저 개발자 로그는 0건이었고 375px에서
  `clientWidth=360`, `scrollWidth=360`이었다.

## 2026-07-29 FIX D — Arknights 첫 10회 보장 프리셋

- Arknights `Standard Headhunting 첫 10회 보장` 프리셋을 추가했다. 출처는
  `https://arknights.wiki.gg/wiki/Headhunting`이며, 첫 10회 안에 최소 5★가
  나온다는 위키 원문을 notes에 그대로 남겼다. 보장 슬롯의 6★/5★ 20/80은
  기본 2:8 확률의 재정규화 추정이므로 confidence는 `community-estimate`다.
- 이 모델은 `maxTrials=10`에서만 유효하다. 시행 횟수를 더 늘리면 실제와
  달라지고 50회 이후 6★ 소프트 천장도 포함하지 않는다고 notes에 명시했다.
  보장 슬롯을 5★ 확정으로 해석할 때의 대안 기대값 `E[6★]=0.1922516`도 함께
  기록해 가정의 차이가 드러나게 했다.
- 별도 스키마 확장 없이 `highSeen` 제어 변수와 10번째 시행 조건부
  `probRules`로 보장 슬롯을 표현했다. `validate` 결과 blocker 0,
  `controlStates=2`, `statStates=14,641`, `totalStates=29,282`,
  `exactAvailable=true`였다.
- Exact 실행은 275개 셀, 남은 질량 1, `peakStates=275`,
  `clampEvents=0`, `accumulatorClampEvents=0`이었다. 기대값은
  `[0.26973568802, 1.07894275208, 4.806289755500002,
  3.8450318043999987]`이고 합은 10이다. 별도 불변식 테스트에서
  `P(5★도 6★도 없음)=0`, counts 합이 10이 아닌 셀 0개,
  `E[6★]=0.26973568802`를 확인했다.
- 새 골든의 preset SHA-256은
  `77a0251a52462a7362541089dc7b56fad2a3f024f04d9cb5cb17977931a45dd5`,
  result SHA-256은
  `d699c6d19a1384b8ffd5e4c524213829e8a6a09814ba9a65e177238ac6d47588`다.
  기존 `simple-pity`, `blue-archive-pickup` 골든도 변경 없이 통과했다.
- `cargo test --workspace --exclude gacha-tauri`는 Rust 55/55를 107.1초에
  통과했다. UI strict 타입 검사, 8개 파일의 24/24 테스트, 1,603개 모듈의
  프로덕션 빌드도 통과했다.
- 실제 프로덕션 미리보기에서 프리셋을 불러와 Exact 계산이 275개 셀·6ms,
  모델 해시 `944d046c527df830889a72b6dc47ffb5540d279b6d41b09a934950830e53a73f`로
  완료되는 것을 확인했다. 375px에서 `clientWidth=360`,
  `scrollWidth=360`이었고 브라우저 개발자 로그는 0건이었다.

## 2026-07-29 UI 정리 — 미지원 규칙 안내와 출처 배지

- 블록 밖 규칙 안내는 규칙이 그대로 보존된다는 사실을 먼저 알리는 중립 정보
  팔레트로 바꿨다. 4건 이상이면 경로 목록만 닫힌 `details`에 넣고 개수·보존
  설명·IR JSON 이동 버튼은 항상 보이게 했다. `role="status"`는 유지했다.
- 출처 confidence 세 값은 `공식 공시`, `데이터마이닝`, `커뮤니티 추정`으로
  한국어화했다. 배지의 `title`에는 원문을 유지하고 등록되지 않은 미래 값은
  원문 그대로 표시한다. `presets/`의 실제 confidence를 전부 순회해 라벨 누락을
  막는 테스트를 추가했다.
- 실제 프로덕션 미리보기에서 Arknights는 접힌 안내가 5건을 보고하고 펼치면
  `probRules[0..3]`, `transitions[0]`이 모두 보였다. Blue Archive와
  simple-pity에는 안내가 없었다. 배지는 각각 `공식 공시`, `커뮤니티 추정`으로
  표시됐다. 375px에서 `innerWidth=375`, `clientWidth=360`,
  `scrollWidth=360`이었고 브라우저 개발자 로그는 0건이었다.
- `cargo test --workspace --exclude gacha-tauri`는 Rust 55/55를 117.1초에
  통과했고 세 프리셋 골든과 Arknights 불변식도 그대로다. UI strict 타입 검사,
  8개 파일의 26/26 테스트, 1,603개 모듈의 프로덕션 빌드가 통과했다.
- 출처 이탈 추적은 조사만 하고 구현하지 않았다. 현재 `selectedPreset`은 모델과
  독립이라 저장 모델을 복구하면 선택기는 Blue Archive인데 내용은 Arknights인
  상태도 재현된다. 최소 변경안은 `loadPreset`만 provenance를 pristine으로
  설정하고 Blockly 리스너, JSON 적용, 파일 열기, 결과 탭의 run 변경을 하나의
  dirty 경로로 모으는 것이다. 이탈 후에는 배지를 숨기기보다 `수정됨`으로 표시하고,
  저장 시 사실이 아닌 출처가 전파되지 않도록 `$preset`을 제거하는 편이 안전하다.
  프로그램식 `loadIr` 이벤트와 localStorage 복구의 초기 provenance까지 함께
  다뤄야 하므로 별도 작업으로 남겼다.

## 2026-07-29 Arknights 보장 슬롯 가정 정정

- 첫 10회에 5★·6★가 없을 때 10번째 슬롯을 기본 비율로 재분배한 6★ 20% /
  5★ 80%에서, 6★는 기본 2%를 유지하고 3★·4★ 몫을 모두 흡수한 5★가 98%를
  갖는 모델로 정정했다. 조건식·`highSeen` 제어 변수·IR 구조는 바꾸지 않았다.
- 이전 20/80은 출처 없는 2:8 비례 재분배 추측이었다. 새 해석은 실효 6★ 약 3%를
  소프트 천장에 귀속한 위키 서술, 10연 보장이 6★ 확률을 높인다는 언급이 없는 점,
  6★ 확률이 100%이면 남은 5★가 0%가 되는 극단 구조에 더 잘 맞는다. 다만 게임 내
  확률 공시로 확인된 것은 아니므로 confidence는 `community-estimate`를 유지한다.
- 프리셋 notes에는 채택 근거와 함께 기각한 20/80 대안의
  `E[6★]=0.269735688`, 보장이 매 10연인지 최초 10연 1회인지와 이벤트 보상
  형태인지가 미확인이라는 점을 남겼다. `maxTrials=10` 전용과 50회 이후 소프트
  천장 미포함 범위도 유지했다. 단정적이던 Standard Headhunting 이름은
  `10연 5★ 이상 보장`으로 바꿨다.
- Exact 실측은 275셀, `E[6★]=0.20000000000000004`,
  `E[5★]=1.1486784400999994`, 기대 counts 합 10,
  `P(5★도 6★도 없음)=0`, counts 합이 10이 아닌 셀 0개였다. 모델 해시는
  `8728d8938924a9e4655a49cfbdcf341dc17932f3a56927aeb5dc041f65950a20`다.
- 새 골든의 preset SHA-256은
  `babb4755f02a8d31ae9710d9e2e47fd77c5586341dbaeb346890c425a39545ea`,
  result SHA-256은
  `ba04a45e0fd1ac542772f8cc8781442c825e884436fab75a94ad23990334866f`다.
  `jointCells=275`를 유지했고 `simple-pity`, `blue-archive-pickup` 골든은
  변경 없이 통과했다.
- `cargo test --workspace --exclude gacha-tauri`는 Rust 55/55를 105초에
  통과했다. UI strict 타입 검사, 8개 파일의 26/26 테스트, 1,603개 모듈의
  프로덕션 빌드도 통과했다.
- 실제 프로덕션 미리보기에서 새 프리셋 이름과 배지, Exact 275셀·7ms,
  6★ 추이 최댓값 0.2를 확인했다. 375px에서 `innerWidth=375`,
  `clientWidth=360`, `scrollWidth=360`이었고 브라우저 개발자 로그는 0건이었다.

## 2026-07-30 BACKLOG A — 출처 이탈 추적

- 모델 provenance를 `pristine`·`dirty`·`none` 세 값으로 중앙화했다. 프리셋
  로드만 pristine으로 만들고, Blockly 변경·JSON 적용·결과 탭 모델 변경은
  프리셋에서 시작한 모델을 dirty로 만든다. 파일 열기와 localStorage 복구는
  출처 대조 없이 none으로 둔다.
- pristine 배지는 기존 출처 라벨을 유지하고 dirty는 `수정됨`을 함께 표시한다.
  none은 배지를 숨기고 선택기에 `현재 모델 · 출처 없음`을 표시해 기본 Blue Archive
  선택기가 다른 저장 모델의 출처처럼 보이던 문제를 막았다.
- 저장 시 pristine이 아니면 복제본에서 `$preset`을 제거한다. 원본 모델은 변경하지
  않으며 pristine 내보내기만 출처 메타데이터를 유지한다. 여섯 진입·변경 이벤트와
  세 provenance의 내보내기를 순수 함수 테스트로 고정했다.
- 실제 프로덕션 미리보기에서 Arknights 로드 직후에는 `커뮤니티 추정`,
  JSON 적용과 시행 횟수 변경 뒤에는 `커뮤니티 추정 · 수정됨`을 확인했다.
  새로고침 뒤 모델명은 Arknights로 유지되면서 선택기는 `현재 모델 · 출처 없음`,
  출처 배지는 미표시였다. 프로그램식 프리셋 로드는 dirty를 유발하지 않았다.
- `cargo test --workspace --exclude gacha-tauri`는 Rust 55/55를 100.4초에
  통과했고 세 프리셋 골든은 불변이다. UI strict 타입 검사, 9개 파일의
  28/28 테스트, 1,604개 모듈의 프로덕션 빌드가 통과했다. 375px에서
  `innerWidth=375`, `clientWidth=360`, `scrollWidth=360`이었고 브라우저
  개발자 로그는 0건이었다.

## 2026-07-30 BACKLOG B — 블록 커버리지 확장

- `N회이고 상태 변수가 K이면` 형태의 시행 조건부 확률 규칙을 전용 블록으로
  추가했다. 대상·시행 횟수·상태 변수·비교값·조건 참/거짓 확률을 편집할 수 있고,
  IR의 `if.and`와 두 `eq` 표현식으로 손실 없이 왕복한다.
- 두 결과 중 하나가 나왔을 때 상태를 대입하거나 증가시키는 `or` 전이 블록을
  추가했다. 기존 soft-pity와 단일/부정 전이 블록은 그대로 유지했다.
- Arknights 프리셋에서 시행 조건부 확률 블록 4개와 `or` 전이 블록 1개가
  생성되며 `loadIr` 미지원 항목은 5건에서 0건으로 줄었다. 세 프리셋의
  `loadIr → workspaceToIr` 원문 왕복 단언은 유지했다.
- UI strict 타입 검사, 9개 파일의 29/29 테스트, 1,604개 모듈의 프로덕션
  빌드가 통과했다. Rust 55/55와 Arknights를 포함한 세 프리셋 골든이 모두
  통과해 계산 결과는 불변이다.
- 실제 프로덕션 미리보기에서 Arknights 선택 시 미지원 안내가 0건이고,
  `회이고` 블록 4개와 `또는` 전이 블록 1개가 렌더링되는 것을 확인했다.
  375px에서 `innerWidth=375`, `clientWidth=360`, `scrollWidth=360`이었고
  브라우저 개발자 로그는 0건이었다.

## 2026-07-30 BACKLOG C — UI 유지보수

- Vitest를 2.1.9에서 4.1.10으로 올리고 lockfile에서 취약한 중첩
  Vite/esbuild 의존성을 제거했다. clean `npm ci` 뒤 `npm audit`은 취약점
  0건이고, `import.meta.glob`을 쓰는 Blockly·라벨 테스트를 포함해 9개 파일의
  29/29 테스트가 Vitest 4에서 통과했다.
- 모바일 안내 여부를 TS의 별도 `620px` 미디어 쿼리로 판단하지 않고, CSS의
  단일 미디어 쿼리가 설정하는 `--gacha-mobile-layout` 플래그를 읽도록 바꿨다.
  따라서 grid track 높이 `620px`은 그대로 유지하면서 반응형 기준 숫자의
  중복을 없앴다.
- UI strict 타입 검사와 1,604개 모듈의 프로덕션 빌드가 통과했다. 실제
  브라우저에서 데스크톱 안내 0건, 375px의 CSS 플래그 `1`과 안내 1건,
  닫은 뒤 새로고침 시 안내 0건을 확인했다. 375px에서 `innerWidth=375`,
  `clientWidth=360`, `scrollWidth=360`이었고 브라우저 개발자 로그는 0건이었다.

## 2026-07-30 BACKLOG D — 구현 전 조사

### MC 다중 Web Worker

- 결론은 **고정 스트림 단위로 분할하면 착수 가능, UI에서 runs만 나누는 구현은
  불가**다. 현재 코어는 4,096 run마다 스트림 번호를 부여하고 기본 시드에서
  `jump()`한 RNG를 그 번호에 고정한다. 따라서 워커 수가 아니라 스트림 번호가
  결과를 결정하도록 코어에 shard API를 만들면 같은 시드·run 수의 정수
  히스토그램은 워커 수와 무관하게 같아진다.
- 기존 native Rayon 경로도 같은 스트림 배열을 스레드에 나눈다.
  `parallel_mc_is_reproducible_across_thread_counts`를 별도로 실행해 1스레드와
  4스레드의 joint occurrences·first-hit·seed가 일치하는 단언이 통과함을
  확인했다. 반대로 현재 WASM `run_mc_json`을 여러 워커에서 단순 호출하면 각
  워커가 스트림 0부터 다시 시작하므로 중복 샘플이 생기며 이 방식은 금지해야 한다.
- 구현 시 core/WASM이 `totalRuns`, base `seed`, 고정 stream 범위를 받는 partial
  결과와 중앙 finalizer를 제공해야 한다. joint·first-hit·marginal·checkpoint의
  정수 occurrences와 accumulator clamp 횟수는 합산하고, model hash·tracked
  leaf·trial-series mode는 전 shard가 같은지 검증한다. 결과 순서는 기존처럼
  정렬된 키로 확정한다.
- Wilson 구간은 shard별 구간을 합치지 않고, 합산한 occurrences와 전역
  `actualRuns`로 마지막에 한 번 계산한다. 결과의 seed는 base seed를 그대로
  기록하므로 절대 규칙 6을 유지할 수 있다.
- MC에는 프루닝 자체와 `prunedMass` 필드가 없어 프루닝 손실 합산은 해당 없음이다.
  `clampEvents`는 모델 확률표의 정적 값이라 shard별로 같은지 확인한 뒤 한 번만
  취하고, `accumulatorClampEvents`만 합산해야 한다. 둘을 모두 합산하면 정적
  clamp가 워커 수만큼 부풀어 절대 규칙 8을 어긴다.
- 진행률은 shard 완료 run의 합으로 보고하고, 취소 시 모든 워커를 종료하면서
  partial 결과를 폐기해야 한다. DP·Exact는 대상에서 제외한다. SharedArrayBuffer와
  COOP/COEP는 필요 없다. 권고는 core partial/finalizer와 재현성 테스트를 먼저,
  WASM/프로토콜을 다음, 워커 풀·취소·실브라우저 성능 측정을 마지막 PR로 나누는
  것이다. 이 조사에서는 구현하지 않았다.

### Tauri 인스톨러

- 현재 `bundle.active=false`이고 Windows x64의 단일 exe만 로컬 빌드가 확인된
  상태다. 첫 대상은 **Windows x64 NSIS 설치 프로그램**을 권고한다. 기존
  `icon.ico`와 Windows 실행 검증을 재사용할 수 있고, MSI에만 필요한 VBSCRIPT
  선택 기능을 피할 수 있다. macOS와 Linux용 표준 아이콘 자산과 설치 실측은
  아직 없다.
- GitHub-hosted `windows-latest`에서 Rust·Node를 설치하고 `npm ci` 후 Tauri
  build를 실행하는 build-only CI는 가능하다. 공식 `tauri-action`은 Windows,
  macOS, Linux 빌드와 workflow artifact 업로드를 지원하며, release 관련 입력을
  생략하면 GitHub Release를 만들지 않는다. Linux x64는 WebKitGTK 등 현재 CI에
  없는 패키지를 설치해야 하고, macOS Intel/ARM은 macOS runner와 각 target이
  필요하다.
- Windows 서명은 실행 자체에는 필수가 아니지만 브라우저 다운로드의 SmartScreen
  경고를 피하고 Microsoft Store에 올리려면 필요하다. 공개 배포 전 인증서 또는
  Azure Artifact Signing 방식을 소유자가 선택해야 한다. macOS 공개 배포는 Apple
  코드 서명이 필요하고, App Store 밖 DMG도 notarization이 필요하다. Linux
  AppImage 서명은 별도 GPG 절차다.
- 1단계는 unsigned Windows NSIS를 **workflow artifact로만** 만들어 설치 smoke
  test에 쓰고, 2단계 공개 배포는 서명 방식을 정한 뒤 별도 승인하는 것이 안전하다.
  현재 CI의 `contents: read`를 유지할 수 있다. GitHub Release 생성·asset 업로드는
  `contents: write`, 태그/버전 정책, 공개·draft 여부가 필요하므로 이번 범위에서는
  만들지 않는다.
- 구현 전 소유자 결정 사항은 (1) Windows x64 NSIS 우선 여부, (2) 공개 전에 쓸
  Windows 서명 수단, (3) workflow artifact만 만들지 draft GitHub Release까지
  만들지, (4) macOS/Linux를 같은 M10에 포함할지다.
- 확인한 공식 자료:
  [Tauri GitHub 파이프라인](https://v2.tauri.app/distribute/pipelines/github/),
  [tauri-action](https://github.com/tauri-apps/tauri-action),
  [Windows 설치 프로그램](https://v2.tauri.app/distribute/windows-installer/),
  [Windows 코드 서명](https://v2.tauri.app/distribute/sign/windows/),
  [macOS 코드 서명](https://v2.tauri.app/distribute/sign/macos/),
  [Tauri 사전 요구사항](https://v2.tauri.app/start/prerequisites/).

## 2026-07-30 LIVE A — 저장소 실패 복구

- 모델과 설정의 `localStorage.setItem` 예외를 흡수해 저장소 차단·용량 초과가
  현재 편집과 계산을 중단하지 않게 했다. 모델·설정 읽기 경로도 예외와 손상 JSON
  모두 기본값으로 복구함을 테스트로 고정했다.
- 최상위 React error boundary를 추가했다. 오류가 발생하면 한국어 안내,
  새로고침, `gacha-lab.model.v2` 저장 모델 삭제 후 재시작, 접이식 원본 오류
  메시지를 제공한다. 저장소 삭제 자체가 실패해도 새로고침은 수행한다.
- 실제 프로덕션 미리보기에서 `Storage.prototype.setItem`이 항상
  `QuotaExceededError`를 던지도록 한 뒤 Arknights 프리셋 전환(루트
  57,719자), 블록 필드 수정(57,798자), IR JSON 적용(62,624자)을 순서대로
  수행했고 앱이 계속 동작했다. 정상 저장소에서는 simple-pity 모델을 저장한 뒤
  새로고침해 `90회 하드 천장` 모델이 복구됐다.
- 임시 오류 컴포넌트로 경계 화면과 원본 메시지를 확인하고 임시 코드를 제거했다.
  `저장된 모델 지우고 다시 시작`을 누른 뒤 기본 Blue Archive 모델로 복귀하는
  것까지 확인했다.
- `npx tsc --noEmit`, 10개 파일의 UI 33/33 테스트, 1,606개 모듈의 프로덕션
  빌드가 통과했다. `cargo test --workspace`는 core 55/55와 Tauri 어댑터 2/2를 포함해
  전부 통과했고 세 프리셋 골든도 불변이다. 실제 브라우저의 정상 경로 콘솔 오류는
  0건이었으며 375px에서 `innerWidth=375`, `body.scrollWidth=360`이었다.

## 2026-07-30 LIVE B — 근사 DP 런타임 상한

- 상한을 정하기 전에 `$env:GACHA_REPORT_DP_PEAKS='1'; cargo test -p
  gacha-core -- --nocapture`로 기존 approximate DP를 계측했다. 최대는 지급 전파·중첩 검증의
  20,301 상태였고 simple-pity 프리셋은 1 상태였다. 기존 exact 프리셋 peak는
  Blue Archive 20,301, Arknights 275다.
- `DpOptions.max_layer_states`의 웹 기본값을 1,000,000으로 정했다. 기존 최대의
  약 49배다. `u64 + ScaledF64` 원시 키·값이 24바이트이고 해시맵 여유 공간,
  source 벡터, 병렬 부분 레이어와 다음 레이어가 겹치는 점을 반영하면 대략
  100~200MB 범위에서 중단하므로 브라우저 OOM 전 방어선이 되면서 정상 모델에는
  충분한 여유가 있다. CLI와 스냅샷 명령은 `--max-layer-states`로 높일 수 있다.
- 근사 DP는 프루닝 뒤 실제 레이어 상태 수가 상한을 넘으면 부분 결과를 만들지
  않고 `E011`을 반환한다. 진행 콜백으로 우회하지 않는 회귀 테스트는 10개
  상한에서 실제 11개가 된 시점에 마지막 진행률 9를 남기고 오류가 반환됨을
  확인한다. 성공 결과에는 `peakStates`를 기록하고 UI 결과 헤더에도 표시한다.
- UI의 50,000,000 추정 상태 하드 한도를 코어
  `DP_ESTIMATED_STATE_LIMIT`와 상호 참조했다. 초과한 `W004`를 error로 올려
  실행을 막고, W004/E011 도움말에는 시행 횟수·추적 대상 축소와 MC 전환을
  명시했다.
- release CLI의 500회·4축 균등 모델은 `validate`에서
  `dpAvailable=false`, `totalStates=63,001,502,001`이었다. `dp`는
  43,793ms 뒤 `E011: actual=1,001,236, limit=1,000,000`으로 종료했고,
  `--max-layer-states 100` 재실행은 실제 147개에서 즉시 사용자 상한을 적용했다.
- 실제 프로덕션 브라우저에서 같은 500회 모델은 W004와 50,000,000 한도를
  표시하며 실행 전에 차단됐다. 별도로 UI 추정 22,698,100으로 사전 검사를
  통과하는 60회·제어상태 100 모델은 WASM 실행 2,910ms 뒤
  `E011: actual=1,040,230, limit=1,000,000`과 한국어 해결 방법을 표시했고
  탭은 정상 동작했다. 실행 중 진행률과 취소 버튼, 취소 뒤 오류가 남지 않는
  것도 확인했다.
- `cargo test --workspace --exclude gacha-tauri`는 core 56/56을 통과했고
  세 프리셋 `resultSha256`는 모두 불변이다. `npx tsc --noEmit`, 10개 파일의
  UI 34/34 테스트, 1,606개 모듈의 프로덕션 빌드도 통과했다. 최종 번들의
  브라우저 콘솔 오류는 0건이었고 375px에서 `innerWidth=375`,
  `clientWidth=360`, `scrollWidth=360`이었다.

## 2026-07-30 LIVE C — Worker 사망 감지 판단

- 구현하지 않았다. B의 사전·런타임 방어선으로 확인된 OOM 경로가 차단됐고
  실제 진행률·취소도 정상이다. Worker 무응답 판정은 정당하게 오래 걸리는
  계산과 구분할 heartbeat 설계가 필요하며 계획서도 별도 PR을 요구한다.
  B와 섞지 않고 실제 무응답 사례가 남을 때 독립 작업으로 진행한다.

## 2026-07-30 DP 상한 후속 — 확률표 사전계산 방어

- UI 추정 상태 수가 1,000,000을 넘으면 `W004` warning, 50,000,000을 넘으면
  `E011` error가 되도록 진단 규약과 임계값을 맞췄다. 세 번들 프리셋은 모두
  `W004`/`E011` 없이 기존 범위에 남는다.
- 확률표는 500,000 엔트리부터 `W010`, 10,000,000 초과 시 `E012`를 내며,
  하드 오류에서는 하위 경고나 표 할당으로 진행하지 않는다. 엔트리 수는
  제어 상태 × 시행 × 리프 축을 함께 보고한다.
- 초기 제어값에서 실제 시행 순서의 모든 리프 전이와 트리거 대입·지급 전이를
  따라 도달 가능한 제어 상태를 구하고, 선언 상태 인덱스를 조밀한 확률·전이
  테이블 인덱스로 매핑한다. 수렴한 시행 비의존 전이는 조기 종료한다. lazy
  평가는 추가하지 않아 엔진 내부 루프가 계속 완전 사전계산 표만 조회한다.
- 지시서의 12,007,001 선언 제어 상태(`max` 3000 × 4000,
  `transitions: []`, 두 제어 변수를 참조하는 확률식) 모델은 1차 하드 가드에서
  `target/release/gacha-cli validate .tmp-prob-table.json` 147ms 만에
  24,014,002 엔트리 `E012`로 종료했다. 도달 제한 후 같은 명령의 워밍 측정
  5회는 26.7/7.8/7.4/7.0/7.1ms(최소 7.0ms, 평균 11.2ms)였고,
  `controlStates=1`, `totalStates=3`, `dpAvailable=true`였다.
  `gacha-cli dp`는 peak 3, 확률 0.25/0.5/0.25로 완료했다. 기존 보고의
  무방어 기준값은 26,040ms였다.
- `cargo test --workspace --exclude gacha-tauri`는 core 61/61을 포함해
  전부 통과했다. Blue Archive, Arknights, simple-pity의 `resultSha256`
  3개가 모두 불변이며, 전 구간을 도는 simple-pity도 같은 골든을 유지했다.
  `npx tsc --noEmit`, 10개 파일의 UI 37/37 테스트와 `npm run build`
  (1,606 모듈)도 통과했다.
- 현재 코어로 `wasm-pack build crates/gacha-wasm --target web --out-dir
  ../../ui/public/wasm`을 실행한 뒤 프로덕션 프리뷰를 확인했다. UI에서
  12,007,001 선언 상태의 동등한 두-제어 확률 규칙 모델은 1ms, 3개 결과 셀,
  peak 3으로 완료됐다. 별도 11리프 × 1,000,001시행 모델은 탭 중단 없이
  `E012 · 확률표 한도 초과`와 모든 축을 포함한 한국어 해결 방법을 표시했다.
  콘솔 오류는 0건이었다. 375px에서 `innerWidth=375`,
  `clientWidth=360`, `scrollWidth=360`, `body.scrollWidth=360`으로
  가로 넘침이 없었다.

## 2026-07-30 DP 도달 상태 회귀 수정

- 시행 비의존 전이가 수렴한 뒤 다음 트리거 시행으로 점프할 때 프런티어만
  유지해, 자기 루프가 없는 순환 전이의 일부 상태가 트리거 입력에서 빠지는
  회귀를 수정했다. 점프 시 누적 도달 집합 전체를 트리거 입력 상위집합으로
  전개해 확률표가 런타임 제어 상태를 빠뜨리지 않는다.
- 기존 프리셋에는 `transitions`와 `triggers`를 함께 쓰는 모델이 없었다.
  simple-pity와 Arknights는 전이만 있고, Blue Archive는 트리거만 있으며
  제어 변수가 없다. 기존 도달 회귀 테스트도 시행이 2회이거나 전이가 없어
  수렴 뒤 트리거 점프 분기를 통과하지 않았다.
- parity 2주기와 6번째 시행 트리거를 결합한 회귀 테스트를 추가했다.
  수정 한 줄을 제거한 변이에서는 `controlStates=2`로 컴파일된 뒤
  `compile.rs:222`의 `runtime control state must be present in the probability
  table` 패닉으로 실패했다. 수정 후 `controlStates=4`, 확률표 키
  `[0, 1, 2, 3]`이며 DP는 `P(a=0)=0.0263671875`,
  `P(a=7)=0.0009765625`로 완료했다.
- 도달 탐색이 확률표 하드 상한에서 조기 중단된 `E012`는 발견된 제어 상태와
  엔트리 수를 `at least`, `control>=...`, `entries>=...`로 표시한다.
  끝까지 계산한 `W010`/`E012`는 기존처럼 정확한 개수를 표시한다.
- `cargo test --workspace --exclude gacha-tauri`는 core 62/62를 포함해
  전부 통과했다. Blue Archive, Arknights, simple-pity의 `resultSha256`
  3개와 Arknights 불변식이 그대로이며, `npx tsc --noEmit`, UI 37/37
  테스트와 1,606개 모듈의 프로덕션 빌드도 통과했다.
- 현재 코어로 WASM을 다시 빌드한 프로덕션 프리뷰에서 같은 전이·트리거와
  동등한 `probRules` 확률식을 사용한 모델은 `controlStates=4`, 8개 결과 셀,
  peak 8 상태로 3ms에 완료됐다. 양끝 확률은 CLI와 같고 브라우저 콘솔 오류는
  0건이었다.

### 리뷰 검증 (Claude)

- 수정 한 줄을 직접 제거한 변이에서 새 회귀 테스트가 같은 패닉으로 실패하는
  것을 재현했다. 되돌린 뒤 `controlStates=4`, 해석해 일치를 확인했다.
- `ui/public/wasm`의 바이너리가 수정 커밋보다 4분 앞선 시각이어서 수정 전
  코어일 가능성이 있었다. 현재 코어로 `wasm-pack`을 다시 빌드해 프리뷰에서
  직접 호출한 결과 `controlStates=4`, 8셀, peak 8, `P(a=0)=0.0263671875`,
  `P(a=7)=0.0009765625`였다. 보고값과 일치한다.
- `E012`/`W010` 두 경로를 각각 재현했다. 도달 1,667 × 3,000시행 × 2리프는
  71ms에 하한 표기(`control>=1667`, `entries>=10002000`)로 거부되고,
  도달 501 × 500시행 × 2리프 = 501,000은 정확한 개수의 `W010`만 낸다.
- 성능 영향 없음. 12,007,001 선언 상태 모델 `validate` 57~61ms,
  simple-pity 55~56ms.
- `git diff fd19dbb origin/dev -- presets/`가 공집합이다. 골든 파일이
  한 바이트도 바뀌지 않았다.
- `push_probability_table_size_diagnostic`의 `W010` 분기에 있는
  `is_lower_bound` 처리는 도달 불가능하다. 하한 표기가 붙는 호출은 항상
  하드 상한을 초과해 `E012`로 반환한다. 방어적 중복이라 해롭지 않으며,
  이 함수를 다시 만질 때 정리한다.

## 2026-07-30 프로덕션 배포 (`main` `51b5f2e`)

- 소유자 판단으로 `dev` `0a8d761`을 `main`에 `--no-ff` 병합했다. PR #13~#22가
  한 번에 올라갔다 — UI polish, Arknights 2/98 정정, 출처 추적, Blockly 보장
  규칙, 저장소 실패 복구, DP 런타임 상한, 확률표 방어, 도달 회귀 수정.
- 병합 후 `git diff main dev`가 공집합이다.
- GitHub Actions **CI**와 **Deploy to Netlify** 모두 success다.
  `Verify Netlify credentials`와 `Deploy production site`가 통과했고
  시크릿은 이름으로만 참조한다. 프로덕션 URL은
  `https://gacha-simul.netlify.app`이다.
- 배포된 사이트에서 이번 회귀 모델을 직접 실행해 `controlStates=4`, 8셀,
  `P(a=0)=0.0263671875`, `P(a=7)=0.0009765625`를 확인했다. 수정 전이라면
  wasm 패닉으로 탭이 죽는 모델이므로, 수정이 프로덕션에 반영된 증거다.
  `#root` 57,626자, 콘솔 오류 0건.

## 2026-07-31 블록 에디터 A — 일반 Expr 값 블록

- `expr.rs`가 지원하는 숫자·불리언 식을 재귀 값 블록으로 왕복한다. 리터럴은
  문자열을 그대로 보존하고, `pow` 지수는 i32 필드로 제한하며, Blockly
  Number/Boolean 연결 검사로 잘못된 소켓 연결을 막는다. 코어의 실제 연산자
  23개(`xor` 포함)와 중첩 `if`, `1/3`·`3e-5` 리터럴을 모두 테스트했다.
- 엔티티 확률과 확률 규칙은 임의 숫자 식을 받는다. 기존
  `soft_pity_rule`/`trial_state_prob_rule` 고정 블록 타입은 제거하고 같은
  완성형 트리를 toolbox 프리필로 제공해 사용성은 유지했다. 미래 확장 식은
  기존처럼 `unsupported`로 감지하면서 원문 그대로 왕복한다.
- `ui/src/types.ts`의 `Expr`를 코어 문법에 맞는 재귀 판별 유니온으로 좁혔다.
  일반 확률식을 초기 상태·첫 시행 기준으로 미리 계산하는 검증기 프리뷰도
  추가해, 블록에서 만든 유효한 식이 단순히 비리터럴이라는 이유로 실행
  차단되지 않는다.
- 세 프리셋 모두 블록 로드 시 `unsupported=[]`이고 기존 골든 결과는
  불변이다. 프로덕션 프리뷰의 Exact 결과는 Blue Archive 20,301셀
  (`29887321c2e5`), Arknights 275셀(`8728d8938924`), simple-pity 0셀
  (`a37d60079a6c`)로 CLI 골든의 셀 수와 일치했다. simple-pity의 0셀은
  `run.condition` 필터 뒤 남는 질량이 0인 기존 골든 결과다.
- 일반 `if/and/eq/ge/clamp/add/mul/pow` 조합 모델은 블록 트리로 표시되고
  Exact에서 3셀, peak 3, 확률 0.25/0.5/0.25로 계산됐다.
  브라우저 콘솔 오류는 0건이었다. 375px에서 결과·블록 화면 모두
  `innerWidth=375`, `clientWidth=360`, `scrollWidth=360`으로 가로 넘침이
  없었다.
- `cargo test --workspace --exclude gacha-tauri`는 core 62/62를 포함해
  전부 통과했고 프리셋 `resultSha256` 3개와 Arknights 불변식이 그대로다.
  `npx tsc -b --pretty false`, 10개 파일의 UI 73/73 테스트,
  `wasm-pack build` 뒤 1,606개 모듈의 프로덕션 빌드도 통과했다.

## 2026-07-31 블록 에디터 B — Blockly 변수 메뉴와 문맥 검사

- `control_variable`·`accumulator_variable` 선언 블록을 제거하고 Blockly 변수
  메뉴와 관리 대화상자로 제어·집계 변수의 생성, ID 이름 변경, 삭제,
  `init`·`max`·표시 이름·`clampPolicy` 편집을 옮겼다. 시행 횟수는 삭제할 수
  없는 고정 참조로 보이지만 `stateVars`에는 넣지 않는다. 집계 `update[]`는
  결과 조건과 일반 숫자 식을 받는 별도 집계 갱신 블록으로 왕복한다.
- 확률, 전이, 트리거, 집계 갱신, 최초 달성 조건의 다섯 문맥을 연결 검사기에
  구분했다. 확률·전이·트리거는 제어 변수와 시행만, 집계 갱신은 자기 자신과
  제어 변수·시행만, 조건은 엔티티 카운트와 시행만 허용한다. 직접·중첩·완성
  트리 이동·연결된 식 내부 삽입뿐 아니라 연결 뒤 변수 드롭다운이나 집계
  대상을 바꾸는 우회도 막는다.
- accumulator를 전이 우변에 둔 B2 재현 모델은 새로 만들 수 없고, 기존 JSON을
  열면 `transitions[0]` 미지원 항목으로 표시하면서 원문 `spent + 1`을 그대로
  보존한다. 직렬화 붙여넣기와 실제 연결 undo/redo 경로도 회귀 테스트로
  고정했다. 대문자로 시작하는 엔티티 ID는 손실되는 `n` 접두 인코딩 대신
  원본 ID로 저장하고 경고하며 조건 왕복이 유지된다.
- Blockly 변수 ID 변경은 엔티티 확률식·전이 대입 대상과 우변·트리거 참조를
  함께 바꾼다. 기존 변수의 역할 변경은 참조 의미가 조용히 달라지지 않도록
  금지했다. `role: stat` 원문 보존과 `clampPolicy: error` 편집 및 E004 안내도
  유지한다. 변수 의존 toolbox 프리필은 기존 제어 변수가 있을 때만 활성화하며,
  블록 로드 중 기본 필드명(`guarantee` 등)이 가짜 `stateVars`로 새는 회귀를
  별도 테스트로 막았다.
- A 후속으로 검증 패널의 확률 미리보기 변수는 코어와 같은 control만 사용한다.
  E003 안내는 미해석 변수·음수·합계 초과·리프 질량 불일치를 열거하고, 미리보기
  캡션은 시행 1회와 변수 초기값 기준임을 명시한다.
- `cargo test --workspace --exclude gacha-tauri`는 core 62/62를 포함해 전부
  통과했고 프리셋 골든 3건과 Arknights 불변식은 그대로다.
  `npx tsc --noEmit`, UI 88/88 테스트, 현재 WASM을 사용한 1,607개 모듈의
  프로덕션 빌드도 통과했다.
- 프로덕션 프리뷰에서 변수 생성과 `pity → renamedPity` 변경이 모든 지원 참조에
  반영되고, B2 금지 모델이 예외 없이 열려 원문을 보존하는 것을 확인했다.
  Blue Archive 20,301셀(`29887321c2e5`), Arknights 275셀
  (`8728d8938924`), simple-pity 0셀(`a37d60079a6c`) Exact 결과가 기존
  골든과 일치했다. 375px에서 `innerWidth=375`, `clientWidth=360`,
  `scrollWidth=360`, 변수 대화상자는 좌우 10px 안에 들어왔고, 1280px에서도
  가로 넘침과 브라우저 콘솔 오류가 없었다.

## 2026-07-31 블록 에디터 B 후속 — 연결 검사 인자 순서 독립성

- `GachaConnectionChecker`가 첫 번째 인자의 블록만 검사해, 이미 전이 문맥에
  연결된 산술식 내부 소켓을 부모 우선 순서로 검사하면 accumulator 참조가
  통과하는 결함을 수정했다. 연결 양쪽의 출력 트리를 모두 검사하므로 호출
  순서와 무관하게 같은 안전 속성이 성립한다.
- 수정 전 새 회귀 테스트는
  `rooted insertion parentFirst: expected true to be false`로 실패했다.
  수정 후 직접 연결·중첩 연결·완성 식 이동·연결된 식 내부 삽입 네 경우를
  자식 우선과 부모 우선 양쪽에서 모두 거부한다. 실제 부모 우선 `connect()`도
  accumulator를 붙이지 않고 기존 `pity + 1` 식을 보존한다.
- `npx tsc --noEmit --pretty false`와 UI 88/88 테스트가 통과했다.
