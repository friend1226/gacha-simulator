# Gacha Simulator

블록 기반 모델 IR로 가챠 규칙을 정의하고 몬테카를로 및 마르코프 DP로 확률을 계산하는 프로젝트입니다.

## 빠른 시작

```bash
cargo test --workspace --exclude gacha-tauri
cargo run -p gacha-cli -- validate presets/blue-archive-pickup.json
cargo run -p gacha-cli -- dp presets/blue-archive-pickup.json
cargo run -p gacha-cli -- mc presets/blue-archive-pickup.json --runs 100000 --seed 42
```

**Windows 데스크톱 앱(Tauri), CLI 전체 명령, 웹 UI(개발 서버/프로덕션 미리보기) 실행 및
사용 방법은 [`docs/USAGE.md`](docs/USAGE.md)를 참고하세요.**

현재 구현은 Rust 코어/CLI/WASM API/Tauri 데스크톱 셸, 정확 리터럴 파서, 엔티티→배타적
리프 컴파일, 동적 제어상태·전이·트리거, MC/DP/공통분모 BigInt exact DP(모두 Rayon
병렬화), Wilson 구간, GCHS 스냅샷, Blockly 기반 IR 편집기를 포함합니다. Web Worker
병렬 실행과 Tauri 인스톨러/다중 플랫폼 빌드는 후속 마일스톤입니다 (`docs/STATUS.md`).

