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
| `dp <model>` | 근사(F64/ScaledF64) 또는 정확(BigInt) 마르코프 DP 실행 — `numeric` 필드에 따라 자동 분기 | `--no-prune` (프루닝 비활성화), `--max-layer-states <N>` (기본 1000000) |
| `exact <model>` | `numeric` 설정과 무관하게 강제로 BigInt 정확 DP 실행 | `--reduce` (레이어 공통분모 약분) |
| `mc <model>` | 몬테카를로 시뮬레이션 | `--runs <N>` (기본 100000), `--seed <N>` (기본 42) |
| `snapshot <model> <output>` | DP를 실행하며 레이어를 GCHS 포맷으로 디스크에 저장 | `--policy aggregate\|checkpoint\|full` (기본 `aggregate`), `--pin <layer>` (반복 가능), `--confirm-full`, `--no-prune`, `--max-layer-states <N>` |

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

`--max-layer-states`는 근사 DP 한 레이어의 **실제** 상태 수 상한이다. 초과하면 부분
결과를 완료로 보고하지 않고 `E011`로 중단한다. `--no-prune`은 이 상한을 해제하지 않으며
(오히려 상태가 늘어 더 쉽게 걸린다), 의도적으로 큰 계산을 밀어붙일 때만 값을 올린다.
정확(BigInt) 모드는 이 상한 대상이 아니다.

`dp` 결과의 `peakStates`는 실행 중 관측된 최대 레이어 상태 수다. 상한을 조정하기 전에
이 값으로 실제 규모를 확인할 수 있다.

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

### 3.4 Netlify 프로덕션 배포

배포는 Netlify 빌드 환경이 아니라 GitHub Actions의
`.github/workflows/deploy.yml`에서 수행한다. `main` push 또는 수동 실행 시 다음
게이트를 모두 통과해야 프로덕션 배포 단계가 실행된다.

1. `cargo test --workspace --exclude gacha-tauri`
2. UI 의존성 설치, TypeScript 검사, UI 단위 테스트
3. `wasm-pack`으로 `ui/public/wasm` 생성
4. Vite 프로덕션 빌드
5. `ui/dist/wasm/gacha_wasm_bg.wasm` 존재 확인
6. `ui/dist`를 Netlify 프로덕션 사이트에 업로드

WASM과 `dist`는 빌드 산출물이므로 git에 커밋하지 않는다. 캐시는 해시가 붙는
`/assets/*`만 1년 `immutable`이며, 이름이 고정된 `/wasm/*`와 `/index.html`은
재배포를 즉시 확인하도록 매 요청 재검증한다.

최초 배포 전에 저장소 소유자가 다음 세 가지를 직접 준비해야 한다.

1. Netlify에서 **빌드 명령 없이** 수동 배포 사이트 생성
2. Netlify 개인 액세스 토큰 발급과 사이트 ID 확인
3. GitHub Actions secrets에 `NETLIFY_AUTH_TOKEN`, `NETLIFY_SITE_ID` 등록

시크릿 등록 후 `main`에 push하거나 GitHub Actions의 **Deploy to Netlify**에서
수동 실행한다.

문서만 바뀐 push에서는 워크플로가 실행되지 않는다. CI와 Deploy 모두
`paths-ignore: ["docs/**", "**/*.md"]`를 쓰기 때문이다. 문서 변경은 배포 산출물에
들어가지 않으므로 이때 배포가 돌지 않는 것이 정상이고, 라이브 사이트는 직전 코드
커밋의 빌드를 계속 서비스한다.

#### 저장소를 Netlify에 직접 연동하지 않는다

이 사이트는 **빌드 명령이 없는 수동 배포 사이트**여야 한다. GitHub Actions가 이미
빌드한 `ui/dist`만 올리므로 Netlify 자체 빌드는 필요 없다. 저장소를 Netlify에
연동하면 Netlify가 스스로 빌드를 시도하다가 실패한다. 증상은 "프레임워크나 빌드
단계를 감지할 수 없다"는 오류다.

`netlify.toml`의 `ignore = "exit 0"`이 그 빌드를 취소하도록 넣어 뒀지만, **Netlify가
루트 `netlify.toml`을 읽을 때만** 동작한다. 사이트 설정에 base directory가 잡혀
있으면 Netlify는 `<base>/netlify.toml`을 찾고, 이 저장소에는 그 파일이 없어서 설정이
통째로 무시된다. 즉 ignore 설정만 믿으면 안 된다.

연동돼 있다면 Netlify 사이트 설정의 **Build & deploy → Continuous deployment**에서
저장소 연결을 해제한다. 연결을 유지해야 한다면 base directory를 비워 루트
`netlify.toml`이 읽히게 한다.

**`publish = "."`로 바꾸지 않는다.** 루트에는 `index.html`이 없고(`ui/index.html`에
있다) Rust 소스와 문서가 그대로 공개된다. 자체 빌드 실패의 해결책으로 이 설정이
제안되는 경우가 있는데 오답이다.

Netlify 자체 빌드가 실패해도 라이브 사이트는 영향받지 않는다. Netlify는 마지막 성공
배포를 유지하고, GitHub Actions는 `netlify deploy --dir=ui/dist --prod`로 이미 빌드된
결과를 직접 올리므로 사이트의 빌드 설정과 무관하게 동작한다.

### 3.5 UI 사용법

화면 상단에는 **모델 · 결과 · 도움말 · 설정** 탭이 있다. 900px보다 좁은 화면에서는
모델 편집기와 검증 패널이 세로로 배치된다.

1. **프리셋/모델 파일**: 상단 목록은 `presets/*.json`의 세 프리셋(Blue Archive 픽업 ·
   Arknights 10연 보장 · 일반 하드 천장)을 읽는다. 모델은 브라우저에 자동 저장되며
   JSON 파일로 열기/저장할 수도 있다.
   - 목록 옆 배지는 출처 신뢰도를 한국어로 표시한다: **공식 공시 · 데이터마이닝 ·
     커뮤니티 추정**. 마우스를 올리면 원문 값이 보인다.
   - 프리셋을 불러온 뒤 편집하면 배지에 **· 수정됨**이 붙는다. 이때 JSON으로 저장하면
     `$preset` 출처 메타데이터를 **제거하고** 내보낸다 — 편집된 모델이 원본 출처를
     주장하지 않게 하기 위한 것이다.
   - 파일에서 열거나 브라우저에 저장된 모델을 복구한 경우에는 출처를 알 수 없으므로
     목록이 **현재 모델 · 출처 없음**으로 바뀌고 배지가 표시되지 않는다.
2. **모델 편집**: 모델 탭에서 "블록"(Blockly 비주얼 에디터) 또는 "IR JSON"(직접 텍스트
   편집)을 선택한다. 블록은 최상위 모델 컨테이너의 뽑기 결과·상태와 집계·확률 규칙·
   결과 변화·시행 이벤트·조건 슬롯에 넣는다.
   - 블록 편집기는 IR의 일부만 표현한다. 표현할 수 없는 규칙이 있으면 검증 패널에
     안내가 뜨고 **해당 규칙은 블록을 편집해도 그대로 보존된다.** 내용은 IR JSON
     탭에서 확인한다. 안내가 뜬 상태도 정상 동작이며 오류가 아니다.
3. **실시간 검증 패널** (모델 탭 우측): 모델을 바꾸는 즉시 갱신된다.
   - 리프별 확률표와 합계(항상 1.000000이어야 정상)
   - 정확 계산 가능 여부, 가챠 규칙 상태 수, 집계 변수까지 포함한 계산 규모
   - 진단 메시지(E/W 코드) — 블록 포커스 또는 도움말의 한국어 해결 방법으로 이동
4. **실행**: 결과 탭 실행 바에서 시행 횟수, 추적 대상, 수치 모드, 시행 시리즈,
   시뮬레이션 반복 수와 시드를 지정한다.
   - **정확 계산 (DP)**: 가능한 상태를 합산해 같은 입력에 항상 같은 결과를 반환
   - **시뮬레이션 (MC)**: 지정 반복 수만큼 무작위 실행하고 각 셀에 Wilson 95% 오차 범위 표시
   - 웹에서는 계산이 Worker에서 돌기 때문에 실행 중에도 화면이 멈추지 않는다. 진행률
     막대와 **취소** 버튼이 표시된다. 데스크톱 앱(Tauri)은 네이티브 경로를 쓰며 진행률과
     취소가 없다.
   - 실행이 진단으로 실패하면 코드별 한국어 제목과 해결 방법이 표시되고, 도움말 탭의
     같은 코드로 이동할 수 있다. 영문 원문은 접이식 영역에 그대로 남는다.
5. **결과 읽기**: 추적 축마다 행/열/집계/필터 역할을 선택한다. 1차원 결과는 SVG
   막대그래프, 2차원은 확률 텍스트를 병기한 히트맵으로 표시된다. DP와 MC를 모두
   실행하면 같은 피벗 배치로 나란히 비교한다.
6. **시행/첫 달성**: `trialSeries: marginal` 결과는 시행별 기대 개수 추이로,
   `condition` 결과는 첫 달성 PMF/CDF·중앙값·90백분위·실패 확률로 표시된다.
7. **재현과 내보내기**: 결과 카드에 엔진, 수치 모드, 시드, 반복 수, 모델 해시,
   소요 시간, 프루닝 손실과 두 종류의 clamp 이벤트가 표시된다. 현재 피벗 CSV와 원본
   JSON을 복사/저장할 수 있다.
8. **설정**: 블록 효과음 볼륨, 기본 수치 모드, MC 반복 수/시드, 확률 표기와 최대 표
   행 수를 조정한다. 설정은 `localStorage`의 버전 있는 스키마로 저장된다.

---

## 4. 문제 해결

| 증상 | 원인 | 해결 |
|---|---|---|
| 웹 UI에서 "WASM 패키지가 없습니다" | `ui/public/wasm`에 빌드 산출물이 없음 | §3.1 실행 |
| WASM을 이미 빌드했는데도 `npm run dev`에서 같은 "WASM 패키지가 없습니다" 메시지 | §3.2의 알려진 Vite 제약. 동적 import 실패를 미배치와 구분하지 못해 같은 문구가 나온다 | §3.3 프로덕션 미리보기 사용 |
| `gacha snapshot ... --policy full`이 거부됨 | 절대 규칙 — `full`은 명시적 확인 필수 | `--confirm-full` 추가 (용량 먼저 확인 권장) |
| `E011: approximate DP layer state count ... exceeds limit` | 근사 DP 한 레이어의 실제 상태 수가 기본 상한 1,000,000 초과 | 시행 수·추적 대상을 줄이거나, 규모를 알고 밀어붙일 때만 `--max-layer-states`를 올린다. MC로 바꾸는 것도 방법 |
| `E012: probability precompute table requires ... entries` | 확률표 사전계산이 1천만 엔트리 초과. 메시지의 `control`·`trials`·`leaves` 축으로 원인을 알 수 있다 | 확률식의 제어 변수 참조를 줄여 도달 제어 상태를 줄이거나, `maxTrials`·리프 수를 줄인다. 상한 초과로 탐색이 중단된 경우 개수는 `>=` 하한으로 표시된다 |
| UI 검증 패널에 `W004` 경고 | 추정 상태 수가 기본 DP 레이어 상한 1,000,000을 넘을 수 있음 (계산은 가능) | 그대로 실행해도 되며, `E011`로 중단되면 위 항목을 따른다 |
| 웹 UI가 오류 복구 화면을 표시 | 렌더 예외 또는 저장된 모델이 현재 스키마와 맞지 않음 | 새로고침을 먼저 시도하고, 반복되면 "저장된 모델 지우고 다시 시작"을 쓴다 |
| Netlify 대시보드에 "프레임워크·빌드 단계를 감지할 수 없다"는 빌드 실패 | 저장소가 Netlify에 연동돼 Netlify 자체 빌드가 돌았다. 배포는 GitHub Actions가 담당한다 | §3.4의 "저장소를 Netlify에 직접 연동하지 않는다" 참고. 라이브 사이트는 영향받지 않으므로 재배포는 필요 없다 |
| `main`에 push했는데 배포 워크플로가 실행되지 않음 | 문서만 바뀌었고 두 워크플로 모두 `paths-ignore`로 `docs/**`·`**/*.md`를 제외한다 | 정상이다. 코드가 그대로이므로 재배포할 이유가 없다. 굳이 돌리려면 **Deploy to Netlify**를 `workflow_dispatch`로 실행한다 |
| `cargo test --workspace`가 Linux/CI에서 `gacha-tauri` 단계에서 실패 | Tauri Linux 백엔드가 GTK/WebKit 시스템 라이브러리 요구 (현재 미설치) | 로컬에서는 `--exclude gacha-tauri`로 우회 가능. CI는 이미 이렇게 설정됨 (`docs/STATUS.md` 참고) |
