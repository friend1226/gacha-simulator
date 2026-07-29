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

## 2026-07-29 D — 공식 출처 프리셋 팩

- Arknights 글로벌 운영팀의 공식 공지
  `https://www.arknights.global/news/1326`에서 Half Year Anniv. Special
  Ten-roll 모집의 6★ 1명 보장과 일반 슬롯 확률(6★ 2%, 5★ 8%, 4★ 50%,
  3★ 40%)을 2026-07-29에 확인해 프리셋으로 추가했다.
- 보장 슬롯은 `trialCount: 9` 뒤 `consumesTrial: true`인 6★ 지급으로 모델링해
  9개 일반 확률 슬롯과 1개 보장 슬롯이 총 10회를 정확히 차지한다. CLI 검증과 Exact
  DP가 통과했고, 결과 220셀 모두 총 카운트가 10이며 6★ 최솟값이 1,
  확률 질량이 부동소수점 오차 범위에서 1임을 확인했다.
- 공식 공시까지 확인하지 못한 검색 후보는 개수를 채우기 위해 추가하지 않았다.
  기존 두 골든의 유지 비용을 늘리지 않기 위해 이번 프리셋 골든은 추가하지 않고,
  CLI 실행과 실제 브라우저 계산으로 검증했다. 프로덕션 미리보기에서 공식 배지,
  Exact 220셀·모델 해시 `b762fded8e5f`, 375px 무가로스크롤을 확인했다.
