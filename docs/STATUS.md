# 구현 상태

## 완료

- Cargo workspace: `gacha-core`, `gacha-cli`, `gacha-wasm`
- 정확 십진/분수/지수 리터럴 파서 (`0.007` → `7/1000`)
- `Prob` 계층과 `F64`, 비트 정규화 기반 `ScaledF64`
- 공통분모 BigInt exact DP (내부 루프 GCD 없음, 선택적 레이어 약분)
- Model IR v1 역직렬화, E001/E003/E004/E006/E007/E008 중심 구조화 진단
- 표현식 AST → 스택 바이트코드 및 유리수 평가
- 엔티티 트리 → 배타적 리프 분할과 `__other__` 생성
- 제어상태·시행별 정확 유리수 확률표 사전계산
- 제어 전이, 확정 지급, 제어상태 기반 동적 확률
- 재현 가능한 xoshiro256++ MC와 alias sampling, Wilson 95% 구간
- 희소 DP, 추적 리프 축소, 프루닝 손실 보고, 최초 달성 PMF/CDF
- 정확 모드 시행/상태/메모리/분모 가드레일과 진행/취소 콜백
- CLI 검증/DP/exact/MC 명령과 WASM JSON API
- 블루 아카이브 및 하드 천장 프리셋
- Blockly 기반 Model IR 편집기, 즉시 검증 패널, 결과/CI 표

## 후속 마일스톤

- Exact DP의 최초 달성 흡수 모드
- 확률표 1천만 제어상태 초과 시 lazy cache
- `consumesTrial=true` 지급의 별도 시행 인덱스 전개
- snapshot의 GCHS/zstd/checkpoint 직렬화
- Rayon/Web Worker 병렬 MC와 자동 CI 폭 정지
- 전체 E002/E005/W003 정적 구간·만족가능성 분석
- Tauri 데스크톱 패키징
- 10개 이상 게임 프리셋 및 골든 파일

## 이 환경에서 실행한 검증

- UI 단위 테스트: 통과
- TypeScript strict 타입 검사: 통과
- Vite 프로덕션 빌드: 통과
- 모든 Rust 파일 tree-sitter 문법 검사: 통과
- Rust `cargo test`: 현재 실행 환경에 Rust 툴체인이 없어 미실행

