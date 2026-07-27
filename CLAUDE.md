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

`cargo build --workspace && cargo test --workspace`를 이 환경에서 처음 실행해 빌드 성공, 테스트 7/7 통과를 확인했다 (`docs/DESIGN.md` §13.4, `docs/STATUS.md`). 회귀 방지를 위해 앞으로도 변경 시 계속 돌린다.

```bash
cargo build --workspace && cargo test --workspace
```

### 2. exact 모드 오작동 버그 — 신규 최우선 (`docs/DESIGN.md` §13.5)

2026-07-27 감사에서 발견. **코드 수정은 아직 하지 않았다.**

- `numeric: "exact"`를 선택해도 `dp` 커맨드/`run_dp_json`이 조용히 `ScaledF64`로 강등되어 실행되고, 출력에도 `"numeric": "scaled"`라고 잘못 표시된다 (`engine_dp.rs:60-64`). 에러·경고 없음
- `ui/src/App.tsx`의 "정확" 드롭다운은 `run_exact_json`을 아예 호출하지 않아 웹에서 이 문제가 완전히 은폐된다
- `engine_exact.rs`의 `ExactResult`에 `clamp_events` 필드가 없어 exact 모드의 확률 clamp 이벤트가 보고되지 않음 (절대 규칙 7 부분 위반)

3번(핵심 검증 테스트)에 착수하기 전에 먼저 고친다 — exact↔ScaledF64 일치 테스트가 이 배선 버그 위에서는 애초에 무의미하다.

### 3. 핵심 검증 테스트 3개 (`docs/DESIGN.md` §13.3)

이 셋이 없으면 이후 모든 변경이 회귀를 감지하지 못한다.

- **MC ↔ DP 교차 검증** — 같은 IR로 DP와 MC(10^6회)를 돌려, DP 값이 각 셀의 Wilson 95% 구간 안에 드는지. 벗어난 셀이 5% 이상이면 실패
- **exact ↔ ScaledF64 일치** — 상대오차 ≤ 1e-10
- **지급 전파** — 확정 픽업 1회가 있는 모델의 `nStar3` 분포가 없는 모델 대비 정확히 +1 이동 (이중 계산이면 +2, 미전파면 +0으로 잡힌다)

### 4. 이후

`docs/DESIGN.md` §13.1(스펙 차이)과 §13.2(미구현 진단 E002/W003) 순으로 해소한다.
스냅샷·병렬화·`u64` 상태 패킹은 M8이므로 3번이 끝나기 전에 손대지 않는다.

---

## 작업 규약

- 문서는 한국어, 코드 식별자와 커밋 메시지는 영어
- Model IR 스키마를 바꾸면 `irVersion`을 올리고 `presets/`와 `ui/src/types.ts`를 함께 갱신한다
- 새 진단 코드를 추가하면 `docs/DESIGN.md` §3.4 표에 등재한다
- 스펙과 다르게 구현할 이유가 생기면 코드만 바꾸지 말고 §13에 기록한다
- `upload/`는 초기 명세를 붙여넣은 임시 디렉터리다. `docs/DESIGN.md`가 정본이므로 삭제 대상이다
