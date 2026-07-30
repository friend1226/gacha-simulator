# Gacha Simulator

블록 기반 모델 IR로 가챠 규칙을 정의하고 몬테카를로 및 마르코프 DP로 확률을 계산하는 프로젝트입니다.

현재 구현은 Rust 코어/CLI/WASM API/Tauri 데스크톱 셸, 정확 리터럴 파서, 엔티티→배타적
리프 컴파일, 동적 제어상태·전이·트리거, 집계 변수, MC/DP/공통분모 BigInt exact
DP(네이티브는 Rayon 병렬화), Wilson 구간, GCHS 스냅샷, Blockly 기반 IR 편집기,
결과 피벗·차트를 포함합니다. 웹 계산은 모듈 Worker에서 실행되며 진행률과 취소를
제공하고, 엔진 진단은 한국어 해결 방법과 연결됩니다.

큰 모델이 브라우저를 멈추게 하지 않도록 자원 상한이 걸려 있습니다. 확률표와 집계 변수
테이블은 할당 전에 엔트리 수를 세서 경고(`W009`/`W010`)하거나 거부(`E010`/`E012`)하고,
확률표는 도달 가능한 제어 상태만 사전계산합니다. 근사 DP는 레이어 상태 수가 상한을
넘으면 부분 결과를 완료로 보고하지 않고 `E011`로 중단합니다. 브라우저 저장소 실패나
렌더 예외는 백화면 대신 복구 화면으로 처리됩니다.

후속 마일스톤은 **MC 다중 워커 병렬화**(현재 웹은 워커 1개라 단일 스레드)와
**Tauri 인스톨러/다중 플랫폼 빌드**입니다. 둘 다 사전 조사가 끝나 있습니다
(`docs/STATUS.md`, `docs/DESIGN.md` §13.1).

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
프리셋 3종(Blue Archive 픽업 · Arknights 10연 보장 · 일반 하드 천장), 피벗 히트맵과
시행·첫 달성 차트를 쓸 수 있습니다.

> **주의**: Vite 개발 서버는 `public/` 자산을 소스 코드에서 동적 import하는 것을
> 허용하지 않아서, `npm run dev`에서는 계산 버튼("정확 계산"/"시뮬레이션")이 동작하지
> 않습니다. WASM을 이미 빌드해 뒀더라도 "WASM 패키지가 없습니다"라고 표시됩니다.
> 계산까지 확인하려면 `npm run build && npm run preview`를 쓰세요.

## Netlify 배포

`main`에 push하면 GitHub Actions가 Rust/UI 테스트, WASM과 UI 프로덕션 빌드,
`dist/wasm/gacha_wasm_bg.wasm` 존재 검사를 순서대로 통과한 뒤 `ui/dist`만 Netlify
프로덕션 사이트에 배포합니다. WASM 산출물은 저장소에 커밋하지 않습니다.

최초 1회 저장소 소유자가 다음 설정을 완료해야 실제 배포가 동작합니다.

1. Netlify에서 빌드 명령이 없는 수동 배포 사이트를 생성합니다.
2. Netlify 개인 액세스 토큰과 사이트 ID를 확인합니다.
3. GitHub 저장소 Actions secrets에 `NETLIFY_AUTH_TOKEN`과 `NETLIFY_SITE_ID`를
   등록합니다.

수동 재배포는 GitHub Actions의 **Deploy to Netlify** 워크플로에서
`workflow_dispatch`로 실행할 수 있습니다. 자세한 운영 방법은
[`docs/USAGE.md`](docs/USAGE.md#34-netlify-프로덕션-배포)를 참고하세요.

## 더 알아보기

- **[`docs/USAGE.md`](docs/USAGE.md)** — Windows/CLI/웹 실행 방법, UI 사용법(블록
  에디터·검증 패널·결과 해석), 문제 해결
- **[`docs/DESIGN.md`](docs/DESIGN.md)** — 설계 명세, 아키텍처 결정
- **[`docs/STATUS.md`](docs/STATUS.md)** — 구현 현황, 검증 이력
