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
  ir.rs          Model IR v1/v2 역직렬화
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

이 9개는 설계의 근간이라 위반 시 조용히 잘못된 결과가 나온다. 리뷰에서 가장 먼저 본다.
(`AGENTS.md`에 같은 내용이 영문으로 있다. 둘을 함께 갱신할 것. 원문은 `docs/DESIGN.md` §12.)

1. **`Fraction`/`BigRational`을 DP 내부 루프에 넣지 않는다.** 정확 모드는 레이어 공통분모 + BigInt 분자만 사용한다. 내부 루프에 GCD가 등장하면 설계 위반이다 (약분은 `reduce_layers` 옵션에서 레이어당 1회만).
2. **리터럴을 f64로 파싱하지 않는다.** `"0.007".parse::<f64>()` 후 유리수 변환 금지. 십진 문자열 → 유리수 직행.
3. **엔진을 백엔드별로 복제하지 않는다.** `run_generic::<P: Prob>` 한 벌만 유지한다.
4. **엔티티 카운트를 상태에 저장하지 않는다.** 저장은 리프 카운트뿐이고 엔티티 카운트는 파생합이다. 확정 지급은 대상 리프 카운터만 올린다 — 상위를 따로 올리면 이중 계산이다.
5. **확률 표현식을 엔진 내부 루프에서 평가하지 않는다.** `prob_table` 사전계산을 반드시 거친다.
6. **MC 출력에 Wilson 신뢰구간과 시드가 항상 동반된다.**
7. **스냅샷 기본값은 `aggregate`다.** `full`은 명시적 확인 없이 실행되어서는 안 된다.
8. **프루닝 손실과 확률 clamp 이벤트를 조용히 버리지 않는다.** 누적해서 결과에 보고한다.
9. **성능 최적화는 §13.3 핵심 검증이 통과한 뒤에만 진행하고, 최적화 후에도 해당 테스트를 계속 통과시킨다.** 정확성 검증이 먼저다.

---

## 지금 할 일 (순서 고정)

M8 성능·스냅샷 작업과 M9 집계 변수·결과 UI·Netlify 배포까지 완료됐다. 현재 기준
회귀선은 Rust 53개와 UI 11개 테스트이며, 프리셋 골든 `resultSha256`은 유지해야 한다.
다음 작업은 아래 순서로 진행한다.

1. 웹 계산을 Web Worker로 옮기고 취소·진행률을 제공한다. Tauri 네이티브 IPC 경로와
   `ui/public/wasm` 배치는 유지한다.
2. 엔진 진단을 한국어 안내와 연결하고, `W009`/`E010` 중복 및 실시간 집계 테이블
   사전 경고를 정리한다.
3. 공식 출처가 확인된 서로 다른 메커니즘의 프리셋을 추가한다. 확인할 수 없는 확률이나
   규칙은 만들지 않는다.

각 단계는 `docs/DESIGN.md` §13과 `docs/STATUS.md`에 검증 결과를 기록한다.

---

## 작업 규약

- 문서는 한국어, 코드 식별자와 커밋 메시지는 영어
- `main`은 프로덕션 배포 브랜치이므로 직접 커밋하지 않는다
- 작업은 `dev`에서 분기한 브랜치에서 진행하고 완료 후 `dev`로 머지한다
- `dev` → `main` 머지는 소유자가 판단하며 에이전트가 임의로 수행하지 않는다
- 자동 프로덕션 배포는 `main` push에서만 실행된다
- Model IR 스키마를 바꾸면 `irVersion`을 올리고 `presets/`와 `ui/src/types.ts`를 함께 갱신한다
- 새 진단 코드를 추가하면 `docs/DESIGN.md` §3.4 표에 등재한다
- 스펙과 다르게 구현할 이유가 생기면 코드만 바꾸지 말고 §13에 기록한다
- `upload/`는 초기 명세를 붙여넣은 임시 디렉터리다. `docs/DESIGN.md`가 정본이므로 삭제 대상이다
