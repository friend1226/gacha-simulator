# Gacha Simulator

블록 기반 모델 IR로 가챠 규칙을 정의하고 몬테카를로 및 마르코프 DP로 확률을 계산하는 프로젝트입니다.

현재 구현은 Rust 코어/CLI/WASM API/Tauri 데스크톱 셸, 정확 리터럴 파서, 엔티티→배타적
리프 컴파일, 동적 제어상태·전이·트리거, MC/DP/공통분모 BigInt exact DP(모두 Rayon
병렬화), Wilson 구간, GCHS 스냅샷, Blockly 기반 IR 편집기를 포함합니다. Web Worker
병렬 실행과 Tauri 인스톨러/다중 플랫폼 빌드는 후속 마일스톤입니다 (`docs/STATUS.md`).

## 사전 준비

| 도구 | 용도 |
|---|---|
| Rust (stable) | 코어/CLI/Tauri 빌드 |
| Node.js 22 | 웹 UI, Tauri 프런트엔드 |
| `wasm-pack` (`cargo install wasm-pack`) + `wasm32-unknown-unknown` 타깃 | 웹 빌드용 WASM 패키징. Tauri만 쓸 거라면 불필요 |

```bash
cargo build --workspace
```

## 빠른 시작 — CLI

```bash
cargo test --workspace --exclude gacha-tauri
cargo run -p gacha-cli -- validate presets/blue-archive-pickup.json
cargo run -p gacha-cli -- dp presets/blue-archive-pickup.json
cargo run -p gacha-cli -- mc presets/blue-archive-pickup.json --runs 100000 --seed 42
cargo run -p gacha-cli -- exact presets/simple-pity.json
```

`validate` / `dp` / `exact` / `mc` / `snapshot` 다섯 명령과 전체 옵션은
[`docs/USAGE.md`](docs/USAGE.md#2-cli-gacha-cli)에 정리돼 있습니다.

## 빠른 시작 — Windows 데스크톱 앱 (Tauri)

```bash
cd ui
npm install
npm run tauri:dev              # 개발 모드
npm run tauri:build:windows    # target/release/gacha-tauri.exe 생성
```

현재 범위는 설치 프로그램 없이 실행 파일만 만드는 것까지입니다.

## 빠른 시작 — 웹

```bash
wasm-pack build crates/gacha-wasm --target web --out-dir ../../ui/public/wasm
cd ui
npm install
npm run dev
```

`http://localhost:5173`에서 모델/결과/도움말/설정 탭, 컨테이너형 Blockly 편집기,
피벗 히트맵과 시행·첫 달성 차트를 쓸 수 있습니다.

> **주의**: Vite 개발 서버는 `public/` 자산을 소스 코드에서 동적 import하는 것을
> 허용하지 않아서, `npm run dev`에서는 "DP 실행"/"MC 실행" 버튼이 동작하지
> 않습니다. 계산까지 확인하려면 `npm run build && npm run preview`를 쓰세요.

## 더 알아보기

- **[`docs/USAGE.md`](docs/USAGE.md)** — Windows/CLI/웹 실행 방법, UI 사용법(블록
  에디터·검증 패널·결과 해석), 문제 해결
- **[`docs/DESIGN.md`](docs/DESIGN.md)** — 설계 명세, 아키텍처 결정
- **[`docs/STATUS.md`](docs/STATUS.md)** — 구현 현황, 검증 이력
