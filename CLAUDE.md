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
cargo test --workspace --exclude gacha-tauri   # gacha-tauri는 Linux에서 GTK/WebKit 필요. CI도 이 형태
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
  state.rs       StateCodec 혼합기수 u64 패킹 (제어 / 집계 / 리프 카운트 3구획)
  compile.rs     엔티티 트리 → 배타적 리프, 확률표 사전계산, 진단, 전이/트리거
  engine_mc.rs   xoshiro256++ + alias sampling + Wilson 구간
  engine_dp.rs   희소 DP, Prob에 대해 제네릭
  engine_exact.rs 레이어 공통분모 + BigInt 분자
  snapshot.rs    GCHS 스냅샷 (aggregate 기본, full은 명시 확인)
  report.rs      집계
crates/gacha-cli/   validate / dp / exact / mc / snapshot
crates/gacha-wasm/  JSON in/out API (진행 콜백 변형 포함)
crates/gacha-tauri/ 데스크톱 셸. 네이티브 IPC로 코어 직접 호출
ui/src/             Blockly IR 편집기 + 검증 패널 + 결과 피벗 + 도움말/설정
  engine.worker.ts  웹 계산 Worker (진행률·취소). Tauri는 이 경로를 쓰지 않는다
  provenance.ts     모델 출처 pristine / dirty / none 추적
  pivot.ts          결과 피벗·집계 (BigInt 10진 합산으로 극소 확률 보존)
  AppErrorBoundary.tsx  렌더 예외 복구 화면 (새로고침 / 저장 모델 삭제)
  validator.ts      실시간 사전 검증. 임계값은 compile.rs·engine_dp.rs와 동기 유지
presets/            게임별 Model IR (코드 하드코딩 금지)
presets/golden/     프리셋별 결과 SHA-256 골든
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

## 현재 상태 (2026-07-30)

M8 성능·스냅샷, M9 집계 변수·결과 UI를 끝내고 라이브 서비스 방어까지 넣은 상태가
`main` `51b5f2e`로 프로덕션(https://gacha-simul.netlify.app)에 배포돼 있다. 세부 검증
기록은 `docs/STATUS.md`에 있다.

- 웹 계산이 모듈 Worker에서 돌고 진행률·취소를 제공한다. Tauri는 네이티브 IPC 경로를
  그대로 쓰며 진행률·취소가 없다
- 엔진 진단이 한국어 제목·해결 방법과 도움말 탭으로 연결된다. 영문 원문은 접이식으로
  보존된다
- 저장소 실패로 백화면이 되지 않는다. `localStorage` 쓰기는 모두 보호되고 렌더 예외는
  오류 경계가 새로고침·저장 모델 삭제를 제공한다
- 자원 상한이 세 층으로 걸려 있다. 근사 DP 레이어 실제 상태 수는 1,000,000(`E011`),
  확률표 사전계산은 500,000 엔트리부터 `W010`·10,000,000 초과 시 `E012`, 집계 변수
  테이블은 `W009` / `E010`. 확률표·집계 테이블은 **할당 전에** 엔트리 수를 세서 판정한다
- 확률표는 초기 제어값에서 리프 전이와 트리거를 따라 **도달 가능한 제어 상태만**
  완전 사전계산한다. 선언 상태 수가 아니라 도달 수가 기준이다 (절대 규칙 5 유지)
- 실시간 검증 패널이 추정 상태 수를 1,000,000에서 `W004` 경고, 50,000,000에서 `E011`
  차단으로 사전 안내한다. `E` = error, `W` = warning 규약을 지킨다
- Blockly 편집기가 표현하지 못하는 IR 규칙을 삭제하지 않고 보존하며 안내한다.
  `presets/` 전체에 대한 왕복 충실도 회귀 테스트가 이를 고정한다
- 모델 출처를 `pristine` / `dirty` / `none`으로 추적한다. 편집된 모델을 내보낼 때
  `$preset` 메타데이터를 제거한다
- 프리셋 3종(Blue Archive 픽업, Arknights 10연 보장, 일반 하드 천장)과 골든 3종

**회귀선: Rust 62개, UI 37개.** 프리셋 골든 `resultSha256` 3개는 유지해야 한다.
`irVersion`은 2다.

프리셋 3종 중 `transitions`와 `triggers`를 **함께 쓰는 모델이 없다.** 도달 계산 회귀가
골든을 통과한 원인이므로, 그 조합을 건드리는 작업은 `core_diagnostics.rs`의
`cyclic_transition_frontier_keeps_late_trigger_states_reachable`에 의존한다.

## 다음 후보 (소유자 판단 대기)

- **MC 다중 워커.** 조사 완료. 고정 RNG 스트림 단위 shard + 중앙 finalizer면 워커 수와
  무관한 재현성이 보장된다. 단순 run 분할은 스트림 중복으로 금지. `clampEvents`는
  정적 값이라 합산 대상이 아니고 `accumulatorClampEvents`만 합산한다. DP·Exact는 제외
- **Tauri 인스톨러 (M10).** 조사 완료. Windows x64 NSIS를 workflow artifact로만 만드는
  1단계를 권고. 서명·Release 게시는 소유자 결정 사항
- **Arknights 확률 공시 확인.** 보장 슬롯 6★/5★ 비율(현재 2/98 해석)과 보장 범위가
  미확정. 확정되면 프리셋과 불변식 테스트를 갱신한다
- **프리셋 추가.** 블록 편집기로 보장 규칙까지 표현되므로 이전보다 비용이 낮다.
  출처·메커니즘 사전 보고와 소유자 승인 절차는 유지한다
- **Worker 무응답 감지.** 정당하게 오래 걸리는 계산과 구분할 heartbeat 설계가 필요하다.
  실제 무응답 사례가 관측되면 착수한다 (`docs/STATUS.md` 2026-07-30 LIVE C)
- **lazy 확률표.** `docs/DESIGN.md` §4.2의 스펙. 도달 제한과 `E012`로 무제한 사전계산은
  해소됐고, lazy는 표현식을 엔진 경로에서 평가해 **절대 규칙 5와 충돌**한다. 필요가
  생기면 §13에 근거를 기록하고 소유자 판단을 받는다

각 작업은 `docs/DESIGN.md` §13과 `docs/STATUS.md`에 검증 결과를 기록한다.

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
