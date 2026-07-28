# M8 구현 계획 (성능/인프라)

Codex 작업자 대상. `docs/DESIGN.md`가 정본이며 이 문서는 그 §6.1/§5.2/§8/§4.2, §11 M8 항목을
실행 가능한 작업 단위로 쪼갠 것이다. 충돌 시 `docs/DESIGN.md`가 우선한다.

## 0. 전제 조건 (착수 전 확인)

- `CLAUDE.md` 절대 규칙 9번: "성능 최적화는 §13.3 핵심 검증이 통과한 뒤, M8에서 진행한다."
  → §13.3 전체 및 E002/W003/exact 흡수까지 완료, `cargo test --workspace` 33/33 통과 확인됨
  (`docs/STATUS.md` 2026-07-27 항목들). **이 전제가 깨졌으면(즉 실패하는 기존 테스트가 있으면)
  M8 작업을 시작하지 말고 먼저 보고할 것.**
- 시작 전 `cargo build --workspace && cargo test --workspace` 를 한 번 돌려 baseline을 확인한다.
- 각 Phase는 절대 규칙을 위반하지 않아야 한다. 특히:
  - 규칙 1: DP/Exact 내부 루프에 `Fraction`/`BigRational`·GCD 금지 (패킹 작업이 이 규칙을 어기기 쉬우므로 §1 참고)
  - 규칙 3: 엔진을 백엔드별로 복제하지 않는다. `run_generic::<P: Prob>` 한 벌 유지
  - 규칙 6: MC 결과에는 항상 Wilson 구간 + 시드 동반 (병렬화 후에도 유지)

## 1. 작업 순서와 근거

아래 순서로 진행한다. 뒤 단계가 앞 단계에 의존하므로 순서를 바꾸지 말 것.

1. **상태 u64 패킹** — DP/Exact 성능의 근본 병목이자, 병렬화(2)의 전제조건 (락 없는 값 복사가 쉬워짐)
2. **MC 병렬화** — 상태 간 의존성이 없어 가장 리스크가 낮다. u64 패킹과 독립적이지만 먼저 해도 되고 1과 병행 가능
3. **DP 레이어 병렬 전개** — 1의 패킹된 상태 위에서 진행. rayon 도입
4. **스냅샷 직렬화 (GCHS)** — 1의 상태 인덱싱에 의존 (인덱스 delta 인코딩)
5. **(후순위/조건부) lazy 확률표** — 실제로 제어상태 10^7 초과 모델이 필요할 때 착수. 이번 라운드는 스펙만 재확인하고 코드 착수는 보류

각 Phase는 별도 커밋(또는 별도 PR)으로 분리한다. Phase마다 `cargo test --workspace` 통과를
전제로 다음 Phase로 넘어간다.

---

## Phase 1 — 상태 u64 패킹 (`state.rs` 신설)

### 현재 문제

- `crates/gacha-core/src/engine_dp.rs:19-22`와 `crates/gacha-core/src/engine_exact.rs:25-28`에
  **동일한 목적의 `State { control: Vec<u32>, counts: Vec<u32> }`가 중복 정의**되어 있다.
  절대 규칙 3(엔진 복제 금지)의 정신에 어긋나는 부분이므로 패킹 작업과 함께 통합한다.
- 전이(`transition`)마다 `State`를 clone하면서 `Vec` 힙 할당이 최소 2회(`control`, `counts`) 발생하고,
  `HashMap<State, P>`의 해시 계산이 `Vec` 전체를 순회한다. `docs/DESIGN.md:900`이 이를
  "DP 성능의 주 병목"으로 명시.

### 목표

`docs/DESIGN.md` §6.1 사양대로 혼합 진법(mixed-radix) 정수 패킹으로 교체:

```rust
index = c0 + m0*(c1 + m1*(c2 + m2*(...)))
```

### 구체적 작업

1. `crates/gacha-core/src/state.rs` 신설. `CompiledModel`이 이미 갖고 있는 control 변수 상한과
   추적 리프 카운트 상한(각 카운트의 max)으로부터 각 차원의 modulus(`m_i`)를 계산하는
   `StateCodec` (또는 유사 이름) 구조체를 만든다.
   - `encode(control: &[u32], counts: &[u32]) -> u64`
   - `decode(index: u64) -> (Vec<u32>, Vec<u32>)` — 결과 리포팅(`DpCell.counts` 등)에 필요하므로 유지
   - modulus 곱이 `u64::MAX`를 넘는 경우 컴파일 단계에서 이미 `dp_available=false`로 걸러지는지
     확인하고, 안 걸러지면 여기서 명시적으로 에러 반환 (조용한 오버플로 금지 — 절대 규칙 8과 같은 정신)
2. `engine_dp.rs`와 `engine_exact.rs`의 `State` 구조체와 관련 로직을 이 `StateCodec` 기반으로
   교체한다. `HashMap<u64, P>`로 레이어를 저장 (§6.2 스펙과 일치).
3. **주의**: 패킹/언패킹 자체는 정수 산술(u64 곱셈·나눗셈·모듈로)만 사용해야 한다.
   유리수 연산이 아니므로 절대 규칙 1과 무관하지만, 실수로 `Rational`을 이 경로에 끌어들이지 않는다.
4. `control_index(state)` 같은 헬퍼가 패킹된 u64에서 control 부분만 빠르게 추출할 수 있어야
   한다 (확률표 조회에 매 트라이얼마다 쓰임 — `engine_dp.rs:596` 참고). 비트 시프트/모듈로 연산
   O(1)로 가능하도록 control 차원을 counts 차원보다 하위 비트에 두는 등 레이아웃을 설계한다.

### 완료 기준

- `cargo test --workspace` 전부 통과, 기존 골든 파일(SHA-256) 결과 불변
- `docs/DESIGN.md` §6.6 성능 목표 표의 첫 두 행(N=1,000 픽업만 추적 / 결합 추적)을 벤치마크로
  측정해 목표(<300ms / <8s) 대비 개선 폭을 STATUS.md에 기록
- `engine_dp.rs`/`engine_exact.rs`에 있던 중복 `State` 정의 제거 확인

---

## Phase 2 — MC 병렬화

### 목표

`docs/DESIGN.md` §5.2: "스레드별로 RNG jump 함수로 분리된 스트림 사용. 각자 로컬 히스토그램을
만들고 마지막에 머지."

### 구체적 작업

1. `crates/gacha-core/Cargo.toml:10`에 이미 선언만 된 `parallel = []` feature에 실제 `rayon`
   의존성을 연결한다 (native 빌드만; `gacha-wasm`은 제외 — WASM은 Web Worker 몫이며 이번
   Phase 범위 밖).
2. `engine_mc.rs:82` `run_mc`의 run 루프를 청크로 나눠 `rayon::scope` 또는
   `par_iter`로 분배. xoshiro256++의 jump 함수(라이브러리가 제공하면 사용, 없으면
   `long_jump`/시드 분기)로 스레드별 독립 스트림을 만든다 — **스레드마다 단순히 시드에 스레드
   인덱스를 더하는 방식은 스트림 간 상관관계를 유발할 수 있으므로 금지**, jump 함수를 써야 함.
3. 스레드 로컬 히스토그램(`HashMap<StatKey, u64>`)을 만들고 마지막에 머지한다. 개별 run
   이력은 원래도 저장하지 않으므로 (§5.2) 이 부분은 변경 없음.
4. `#[cfg(feature = "parallel")]` 게이트를 실제로 코드에 심는다 (`docs/DESIGN.md:902`가
   "feature flag는 선언만 있고 게이트가 전혀 없는 빈 스텁"이라고 지적한 부분 해소).
   feature 미활성 시 기존 단일 스레드 경로가 그대로 동작해야 한다.
5. `McResult.seed`는 사용자가 입력한 최상위 시드를 그대로 기록한다 (절대 규칙 6). 병렬화가
   시드-재현성을 깨서는 안 된다 — 동일 시드 + 동일 스레드 수로 실행하면 항상 동일 히스토그램이
   나와야 한다는 회귀 테스트를 추가한다.

### 완료 기준

- 신규 테스트: 같은 시드로 단일 스레드 실행 결과와 병렬 실행 결과가 히스토그램상 완전히 일치
  (스레드 수를 바꿔도 일치 — 순서 의존성이 없어야 함)
- 코어 수에 비례한 처리량 개선을 STATUS.md에 기록 (정확한 배수 목표는 없으나 스케일링 확인용)

---

## Phase 3 — DP 레이어 병렬 전개

### 목표

`engine_dp.rs:88` 이하 `run_generic::<P: Prob>`의 레이어 전개 루프(`engine_dp.rs:~592-608`
해당, `docs/DESIGN.md` §6.3)를 병렬화한다.

### 구체적 작업

1. Phase 1에서 u64로 패킹된 셀 순회(`layer.cells.iter()`)를 `rayon`의 `par_iter`로 분배하되,
   여러 스레드가 같은 `next_state`에 쓸 수 있으므로 단순 `HashMap::insert`는 데이터 레이스다.
   - 권장 접근: 스레드별 로컬 `HashMap<u64, P>`를 만들고 최종적으로 합산(reduce)한다
     (MC와 동일한 map-reduce 패턴). `P: Prob`의 덧셈이 결합법칙을 만족하는 한 (정확 모드도
     BigInt 덧셈이므로 문제 없음) 순서 비의존적으로 병합 가능.
2. `apply_pruning`, `snapshot_policy.on_layer`, `progress_callback`은 병합 이후 단일 스레드
   구간에서 호출 (레이어 단위 배리어).
3. **정확 모드(ExactInt)의 질량 보존 단언(§6.5)은 병렬화 후에도 "정확히" 성립해야 한다** —
   BigInt 덧셈 순서가 바뀌어도 정확 모드는 결과가 완전히 동일해야 하므로, 기존 질량 보존
   테스트가 그대로 회귀 검증 역할을 한다. 부동소수점 모드는 합산 순서가 바뀌면 마지막 비트가
   달라질 수 있으므로 허용 오차(`1e-9`, §6.5)로 비교.
4. 이 Phase는 `parallel` feature 아래 게이트하고, Phase 2와 동일하게 미활성 시 단일 스레드
   경로를 유지한다.

### 완료 기준

- `docs/DESIGN.md` §6.6 성능 목표 재측정 (특히 결합 추적 <8s 목표)
- 정확 모드 골든 테스트가 병렬/비병렬에서 바이트 단위로 동일한 결과를 냄을 확인하는 회귀 테스트

---

## Phase 4 — 스냅샷 직렬화 (GCHS 포맷)

### 목표

`docs/DESIGN.md` §8 전체 (현재 미구현 — 관련 파일/심볼 없음, `docs/DESIGN.md:901`).

### 구체적 작업

1. `snapshot_policy` 3종 구현: `aggregate`(기본), `checkpoint`(로그 간격 1,2,5,10,20,50,100,
   200,500,1000 + 사용자 핀 레이어), `full`.
   - **절대 규칙 7: 기본값은 반드시 `aggregate`. `full`은 명시적 확인 없이 실행되지 않아야 한다.**
     CLI/WASM API에 `full` 선택 시 확인 플래그(예: `--confirm-full` 또는 JSON 옵션의 명시적
     boolean)를 요구하도록 설계한다.
2. `[Header]`/`[Body]` 포맷을 §8.2 그대로 구현: magic `"GCHS"`, version, numeric_backend,
   model_hash(IR canonical JSON의 SHA-256), n_trials, layer_index, state_dims, cell_count,
   본문은 zstd 압축, 상태 인덱스는 정렬 후 delta+varint 인코딩.
   - 상태 인덱스가 Phase 1의 u64 패킹 결과와 동일한 정렬 기준을 써야 delta 인코딩 효율이
     의도대로 나온다 — Phase 1 완료 후 착수해야 하는 이유.
3. zstd crate 의존성 추가 (레벨 3).
4. §8.3 용량 사전 경고: 실행 전 예상 용량 계산, 200MB 경고 임계값, "사용 가능 메모리의 50%"
   거부 임계값. 경고 시 대안 제시 문구("aggregate로 전환 시 5MB" 등)를 결과 텍스트에 포함.
5. 재계산 기반 복원(체크포인트 사이 임의 레이어를 최대 K/2 스텝 재계산)을 CLI 명령 또는
   내부 API로 제공. 목표: 100ms 이내 (§8.1).
6. WASM 쪽 노출 여부는 이번 Phase 범위에서 CLI만 우선 구현하고, UI 연동은 별도 후속 작업으로
   `docs/STATUS.md`에 남긴다 (범위 과확장 방지).

### 완료 기준

- 스냅샷 저장 → 다른 프로세스에서 로드 → model_hash 불일치 시 명확한 에러로 거부하는 테스트
- `aggregate`/`checkpoint`/`full` 각각 용량이 §8.1 표의 대략적 크기(N=1000, 결합추적 기준
  ~5MB/~50MB/~4GB 근방)와 같은 자릿수인지 확인하는 스모크 테스트
- `full` 미확인 상태에서 호출 시 실행되지 않고 에러/경고를 반환하는 테스트

---

## Phase 5 — lazy 확률표 (보류, 조건부 착수)

`docs/DESIGN.md` §4.2, `docs/DESIGN.md:903`: 제어상태 10^7 초과 시 사전계산 대신 lazy 평가
+ 캐시로 전환하는 기능. 현재는 10,000,000 초과 시 `W004` 경고만 내고 그대로 즉시 계산을
강행한다 (`compile.rs:280-305`).

**이번 라운드 범위 아님.** `docs/STATUS.md`가 이미 "한계 초과 모델이 나오면 착수"라고
못박아 두었고, 현재 프리셋(블루 아카이브, 하드 천장) 어느 것도 10^7 상태에 근접하지 않는다.
Phase 1~4를 완료한 뒤, 실제로 이 한계에 부딪히는 프리셋/사용 사례가 생기면 별도 계획 문서로
분리해 착수한다. 지금 손대면 검증되지 않은 추상화를 미리 만드는 셈이라 CLAUDE.md의
"과설계 금지" 원칙에 어긋난다.

---

## 공통 규약

- Phase별로 별도 커밋. 커밋 메시지는 영어, PR 설명/코드 리뷰 논의는 이 문서 기준 한국어 가능.
- 각 Phase 완료 시 `docs/STATUS.md`에 실행 검증 기록을 추가한다 (기존 항목들과 같은 형식:
  날짜, 무엇을 확인했는지, `cargo test --workspace` 결과 N/N).
- `docs/DESIGN.md` §13.1 표에서 이번에 해소한 항목(§6.1 상태 인코딩, §8 스냅샷, §5.2 병렬)을
  "구현됨"으로 갱신하고, 새로 발견한 스펙-코드 불일치가 있으면 §13.5 이후 번호로 추가한다.
- 성능 수치는 반드시 실측으로 남긴다 (추정치·예상치를 STATUS.md에 "확인됨"으로 적지 않는다 —
  이 프로젝트의 기존 STATUS.md 항목들이 전부 실행 검증 기반이므로 그 관례를 따른다).
- 각 Phase는 이전 Phase가 `cargo test --workspace` 전부 통과한 상태에서 시작한다. 실패한
  채로 다음 Phase로 넘어가지 않는다.
