# CLAUDE.md

가챠 시뮬레이터. 가챠 규칙을 Model IR로 정의하고 **몬테카를로(근사 + 신뢰구간)** 와
**마르코프 DP(정확값)** 두 엔진으로 확률 분포를 계산한다.

- **설계 명세: [`docs/DESIGN.md`](docs/DESIGN.md)** — 작업 전 반드시 읽는다. 아키텍처 결정은 확정 사항이다.
- 구현 현황과 스펙 차이: `docs/DESIGN.md` §13
- 진행 로그: `docs/STATUS.md`

---

## 명령어

```bash
# Rust
cargo build --workspace
cargo test --workspace
cargo run -p gacha-cli -- validate presets/blue-archive-pickup.json
cargo run -p gacha-cli -- dp      presets/blue-archive-pickup.json
cargo run -p gacha-cli -- exact   presets/simple-pity.json
cargo run -p gacha-cli -- mc      presets/blue-archive-pickup.json --runs 100000 --seed 42

# UI
cd ui && npm install && npm run dev
cd ui && npx tsc --noEmit && npm test
```

---

## 구조

```
crates/gacha-core/src/
  rational.rs    정확 리터럴 파서 ("0.007" → 7/1000). f64 경유 금지
  numeric.rs     Prob trait + F64 / ScaledF64 (mantissa f64 + exponent i64)
  ir.rs          Model IR v1 역직렬화
  expr.rs        표현식 AST → 스택 바이트코드, 유리수 평가
  compile.rs     엔티티 트리 → 배타적 리프, 확률표 사전계산, 진단, 전이/트리거
  engine_mc.rs   xoshiro256++ + alias sampling + Wilson 구간
  engine_dp.rs   희소 DP, Prob에 대해 제네릭
  engine_exact.rs 레이어 공통분모 + BigInt 분자
  report.rs      집계
crates/gacha-cli/   validate / dp / exact / mc
crates/gacha-wasm/  JSON in/out API
ui/src/             Blockly IR 편집기 + 검증 패널 + 결과 표
presets/            게임별 Model IR (코드 하드코딩 금지)
```

---

## 절대 규칙

이 8개는 설계의 근간이라 위반 시 조용히 잘못된 결과가 나온다. 리뷰에서 가장 먼저 본다.
(`AGENTS.md`에 같은 내용이 영문으로 있다. 둘을 함께 갱신할 것.)

1. **`Fraction`/`BigRational`을 DP 내부 루프에 넣지 않는다.** 정확 모드는 레이어 공통분모 + BigInt 분자만 사용한다. 내부 루프에 GCD가 등장하면 설계 위반이다 (약분은 `reduce_layers` 옵션에서 레이어당 1회만).
2. **리터럴을 f64로 파싱하지 않는다.** `"0.007".parse::<f64>()` 후 유리수 변환 금지. 십진 문자열 → 유리수 직행.
3. **엔진을 백엔드별로 복제하지 않는다.** `run_generic::<P: Prob>` 한 벌만 유지한다.
4. **엔티티 카운트를 상태에 저장하지 않는다.** 저장은 리프 카운트뿐이고 엔티티 카운트는 파생합이다. 확정 지급은 대상 리프 카운터만 올린다 — 상위를 따로 올리면 이중 계산이다.
5. **확률 표현식을 엔진 내부 루프에서 평가하지 않는다.** `prob_table` 사전계산을 반드시 거친다.
6. **MC 출력에 Wilson 신뢰구간과 시드가 항상 동반된다.**
7. **프루닝 손실과 확률 clamp 이벤트를 조용히 버리지 않는다.** 누적해서 결과에 보고한다.
8. **성능 최적화는 §13.3 테스트가 통과한 뒤에 한다.** 정확성 검증이 먼저다.

---

## 지금 할 일 (순서 고정)

### 1. 빌드·테스트 통과 — 완료 (2026-07-27 확인)

최초 검증에서 `cargo build --workspace && cargo test --workspace` 7/7 통과를 확인했다. exact 경로 수정 후 8/8, §13.3의 전체 검증과 `W007` 지급 예산 경고를 구현한 최신 로컬 실행은 Rust 27/27 통과다. 작업 브랜치의 GitHub Actions에서도 핵심 검증 병합 전 Rust 13/13과 UI 4/4가 통과했다 (`docs/DESIGN.md` §13.4, `docs/STATUS.md`). 회귀 방지를 위해 앞으로도 변경 시 계속 돌린다.

```bash
cargo build --workspace && cargo test --workspace
```

### 2. exact 모드 오작동 버그 — 완료 (`docs/DESIGN.md` §13.4)

2026-07-27 감사에서 발견한 다음 세 문제는 PR #2에서 수정하고 실행 검증했다.

- `numeric: "exact"` DP 경로가 BigInt exact 엔진으로 정상 위임된다.
- UI "정확" 옵션이 `run_exact_json`을 호출한다.
- exact 결과가 `numeric: "exact"`와 `clamp_events`를 보고한다.

`cargo test --workspace`와 수동 이항분포 exact 스모크 테스트로 확인했다.

### 3. §13.3 검증 테스트 1~7 — 완료 (`docs/DESIGN.md` §13.3)

2026-07-27 다음 검증을 함께 유지하고 `cargo test --workspace` 27/27 통과를 확인했다.

- **MC ↔ DP 교차 검증** — 10개 모델에서 각각 MC 10^6회 및 실제 희귀 비율 모델, Wilson 이탈 셀 5% 미만
- **exact ↔ ScaledF64 일치** — 복합 모델의 모든 셀과 `0.007^200` 극소 셀에서 상대오차 ≤ 1e-10
- **지급 전파** — 200회 모델의 전체 `nStar3` 분포가 정확히 +1 이동
- **지급 의미론** — `consumesTrial × appliesTransitions` 4조합을 MC·ScaledF64 DP·ExactInt DP에서 비교
- **해석해·골든·퍼징** — 기하/음이항 closed-form, 프리셋 2종 SHA-256 골든, 경계 IR 컴파일·실행 퍼징

교차검증이 MC alias-table의 bucket 원소 소실 버그와 Wilson 경계 반올림 버그를 발견했고, 두 문제의 수정 및 직접 회귀 테스트를 완료했다.

### 4. 이후

`docs/DESIGN.md` §13.2의 미구현 코어 진단 E002/W003를 우선 해소하고, §13.1의 exact 최초 달성 흡수 상태와 lazy 확률표를 이어서 검토한다.
스냅샷·병렬화·`u64` 상태 패킹은 M8이므로 위 정확성 작업보다 먼저 손대지 않는다.

---

## 작업 규약

- 문서는 한국어, 코드 식별자와 커밋 메시지는 영어
- Model IR 스키마를 바꾸면 `irVersion`을 올리고 `presets/`와 `ui/src/types.ts`를 함께 갱신한다
- 새 진단 코드를 추가하면 `docs/DESIGN.md` §3.4 표에 등재한다
- 스펙과 다르게 구현할 이유가 생기면 코드만 바꾸지 말고 §13에 기록한다
- `upload/`는 초기 명세를 붙여넣은 임시 디렉터리다. `docs/DESIGN.md`가 정본이므로 삭제 대상이다
