# 실행 및 사용 방법

이 문서는 Windows 데스크톱 앱(Tauri), CLI, 웹 환경 세 가지로 이 프로젝트를 실행하고
사용하는 방법을 다룬다. 아키텍처/설계 결정은 [`docs/DESIGN.md`](DESIGN.md), 구현
현황은 [`docs/STATUS.md`](STATUS.md)를 참고한다.

---

## 0. 사전 준비

| 도구 | 용도 | 확인 |
|---|---|---|
| Rust (stable) | 코어/CLI/Tauri 빌드 | `cargo --version` |
| Node.js 22 | 웹 UI, Tauri 프런트엔드 | `node --version` |
| `wasm-pack` | 웹 빌드용 WASM 패키징 | `cargo install wasm-pack` |
| `wasm32-unknown-unknown` 타깃 | WASM 빌드 | `rustup target add wasm32-unknown-unknown` |

Tauri 데스크톱 앱만 쓸 거라면 `wasm-pack`/wasm 타깃은 필요 없다 (Tauri는 WASM을 거치지
않고 `gacha-core`를 네이티브로 직접 호출한다 — `docs/STATUS.md` 2026-07-29 항목 참고).

리포지토리 루트에서:

```bash
cargo build --workspace
```

---

## 1. Windows 데스크톱 앱 (Tauri)

### 1.1 개발 모드

```bash
cd ui
npm install
npm run tauri:dev
```

Vite 개발 서버(`127.0.0.1:5173`)를 띄운 뒤 Tauri 창이 그 위에서 열린다. 코드 변경 시
핫 리로드된다.

### 1.2 배포용 실행 파일 빌드

```bash
cd ui
npm run tauri:build:windows
```

빌드가 끝나면 `target/release/gacha-tauri.exe`가 생성된다. 현재 범위는 **설치 프로그램
없이 실행 파일만** 만드는 것까지다(`crates/gacha-tauri/tauri.conf.json`의
`bundle.active: false`). exe를 더블클릭하거나 터미널에서 직접 실행하면 된다. 인스톨러
(msi/nsis), 코드 서명, 다른 OS 빌드는 아직 후속 작업이다 (`docs/STATUS.md` 참고).

### 1.3 사용법

앱을 열면 웹 UI와 동일한 화면이 뜬다 — §3 "UI 사용법"을 그대로 따르면 된다. 차이는
계산이 WASM이 아니라 Rust 네이티브 코드로 직접 실행된다는 점뿐이다(더 빠르고, 브라우저
제약이 없다).

---

## 2. CLI (`gacha-cli`)

모든 플랫폼(Windows 포함)에서 동일하게 동작한다. 리포지토리 루트에서 실행한다.

```bash
cargo run -p gacha-cli -- <명령> <모델.json> [옵션]
```

| 명령 | 설명 | 주요 옵션 |
|---|---|---|
| `validate <model>` | Model IR을 검증하고 진단·리프 확률표·마르코프 분석 결과를 JSON으로 출력 | — |
| `dp <model>` | 근사(F64/ScaledF64) 또는 정확(BigInt) 마르코프 DP 실행 — `numeric` 필드에 따라 자동 분기 | `--no-prune` (프루닝 비활성화) |
| `exact <model>` | `numeric` 설정과 무관하게 강제로 BigInt 정확 DP 실행 | `--reduce` (레이어 공통분모 약분) |
| `mc <model>` | 몬테카를로 시뮬레이션 | `--runs <N>` (기본 100000), `--seed <N>` (기본 42) |
| `snapshot <model> <output>` | DP를 실행하며 레이어를 GCHS 포맷으로 디스크에 저장 | `--policy aggregate\|checkpoint\|full` (기본 `aggregate`), `--pin <layer>` (반복 가능), `--confirm-full`, `--no-prune` |

예시:

```bash
cargo run -p gacha-cli -- validate presets/blue-archive-pickup.json
cargo run -p gacha-cli -- dp presets/blue-archive-pickup.json
cargo run -p gacha-cli -- mc presets/blue-archive-pickup.json --runs 100000 --seed 42
cargo run -p gacha-cli -- exact presets/simple-pity.json
cargo run -p gacha-cli -- snapshot presets/blue-archive-pickup.json ./out --policy checkpoint
```

`--policy full`은 절대 규칙(§CLAUDE.md)에 따라 `--confirm-full` 없이는 거부된다 — 용량이
크므로 (§DESIGN.md §8.3) 먼저 `validate`나 기본 `dp` 실행으로 상태 공간을 확인하고 쓰는
것을 권장한다.

빌드된 바이너리로 직접 실행하려면 `cargo build --release -p gacha-cli` 후
`target/release/gacha-cli.exe <명령> ...`을 쓴다 (`cargo run`보다 매번 빠르다).

---

## 3. 웹 환경

### 3.1 WASM 패키지 빌드 (최초 1회, 또는 `crates/gacha-core`/`crates/gacha-wasm` 변경 후)

```bash
wasm-pack build crates/gacha-wasm --target web --out-dir ../../ui/public/wasm
```

이 출력물은 `ui/public/wasm`에 생기며 git에 커밋되지 않는다(빌드 산출물).

### 3.2 개발 서버

```bash
cd ui
npm install
npm run dev
```

`http://localhost:5173`에서 블록 에디터·JSON 편집·실시간 검증 패널을 바로 쓸 수 있다.

> **알려진 제약**: Vite 개발 서버는 `public/` 아래 자산을 소스 코드에서 동적
> `import()`로 불러오는 것을 허용하지 않는다(Vite 자체 정책). 이 프로젝트는 WASM
> 글루 코드를 `public/wasm`에 두므로, **`npm run dev`에서는 "DP 실행"/"MC 실행"
> 버튼이 동작하지 않는다** — 블록 에디터, IR JSON 편집, 실시간 검증 패널(리프 확률·
> 마르코프 분석·진단)까지만 확인할 수 있다. 실제 계산까지 확인하려면 §3.3의 프로덕션
> 미리보기를 쓴다.

### 3.3 프로덕션 빌드 및 미리보기 (계산까지 확인하려면 이 경로를 쓴다)

```bash
cd ui
npm run build      # tsc -b && vite build → dist/
npm run preview     # http://localhost:4173 에서 dist/ 를 그대로 서빙
```

이 경로는 실제 배포 시 나가는 산출물과 동일하며, DP/MC 실행까지 전부 정상 동작한다
(`docs/STATUS.md` 2026-07-29 항목에서 실제 브라우저로 검증됨). 정적 호스팅(GitHub
Pages, S3 등)에 배포할 때도 `dist/`를 그대로 올리면 된다.

### 3.4 UI 사용법

1. **모델 편집**: 상단 탭에서 "블록"(Blockly 비주얼 에디터) 또는 "IR JSON"(직접 텍스트
   편집) 중 선택. 기본값은 블루 아카이브 픽업 프리셋. 상단 "프리셋 초기화"로 언제든
   되돌릴 수 있다.
2. **실시간 검증 패널** (우측): 모델을 바꾸는 즉시 갱신된다.
   - 리프별 확률표와 합계(항상 1.000000이어야 정상)
   - 마르코프 분석: DP 가능 여부, 제어 상태 수, 예상 결합 상태 수
   - 진단 메시지(E/W 코드) — 클릭하면 원인이 된 블록으로 포커스 이동
3. **고급 설정**: 수치 백엔드 선택 — 표준(ScaledF64, 권장), 고속(F64), 정확(BigInt
   유리수). 정확 모드는 `run.numeric: "exact"`와 동일하며 결과가 느리지만 반올림 오차가
   없다.
4. **실행**: 진단에 오류(error)가 없어야 "DP 실행"/"MC 실행" 버튼이 활성화된다.
   - DP 실행: 마르코프 DP로 정확한(수치 백엔드 기준) 결합 확률분포 계산
   - MC 실행: 몬테카를로 10만 회 시뮬레이션, Wilson 95% 신뢰구간 함께 표시
5. **결과 패널**: 확률 상위 30개 셀을 막대 그래프와 함께 표시. 확률 보정(clamp) 이벤트가
   있었다면 함께 표시된다.

---

## 4. 문제 해결

| 증상 | 원인 | 해결 |
|---|---|---|
| 웹 UI에서 "WASM 패키지가 아직 배치되지 않았습니다" | `ui/public/wasm`에 빌드 산출물이 없음 | §3.1 실행 |
| `npm run dev`에서 계산 버튼이 반응 없음/에러 | §3.2의 알려진 Vite 제약 | §3.3 프로덕션 미리보기 사용 |
| `gacha snapshot ... --policy full`이 거부됨 | 절대 규칙 — `full`은 명시적 확인 필수 | `--confirm-full` 추가 (용량 먼저 확인 권장) |
| `cargo test --workspace`가 Linux/CI에서 `gacha-tauri` 단계에서 실패 | Tauri Linux 백엔드가 GTK/WebKit 시스템 라이브러리 요구 (현재 미설치) | 로컬에서는 `--exclude gacha-tauri`로 우회 가능. CI는 이미 이렇게 설정됨 (`docs/STATUS.md` 참고) |
