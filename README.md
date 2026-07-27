# Gacha Simulator

블록 기반 모델 IR로 가챠 규칙을 정의하고 몬테카를로 및 마르코프 DP로 확률을 계산하는 프로젝트입니다.

## 빠른 시작

```bash
cargo test --workspace
cargo run -p gacha-cli -- validate presets/blue-archive-pickup.json
cargo run -p gacha-cli -- dp presets/blue-archive-pickup.json
cargo run -p gacha-cli -- mc presets/blue-archive-pickup.json --runs 100000 --seed 42
```

웹 UI:

```bash
cd ui
npm install
npm run dev
```

현재 구현은 Rust 코어/CLI/WASM API, 정확 리터럴 파서, 엔티티→배타적 리프 컴파일,
동적 제어상태·전이·트리거, MC/DP/공통분모 BigInt exact DP, Wilson 구간 및 Blockly 기반
IR 편집기를 포함합니다. 스냅샷 압축, Web Worker 병렬 실행과 Tauri 패키징은 후속 마일스톤입니다.

