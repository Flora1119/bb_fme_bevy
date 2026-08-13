# BB_FME Unity → Rust/Bevy 이전: 0단계 구현 명세

작성일: 2026-08-12  
분석 대상: `/home/flora1119/unity_projects/BB_FME`  
분석 방식: 소스 코드, Unity YAML 씬/프리팹, Resources 설정, ProjectSettings를 읽기 전용으로 정적 분석

## 목적

이 디렉터리는 Rust/Bevy 재구현 전에 현재 Unity 클라이언트를 동결 명세로 남긴다. “현재 코드가 실제로 하는 일”을 우선 기록하며, 의도는 코드와 에셋에서 확인되는 범위만 별도로 표시한다.

## 문서 구성

- [01-current-implementation.md](./01-current-implementation.md): 제품 범위, 씬 흐름, 런타임 상태, 입력, 물리, 에디터, 플레이
- [02-map-and-server-contracts.md](./02-map-and-server-contracts.md): 맵 JSON 스키마, 로컬 영속 상태, HTTP API 계약
- [03-block-catalog.md](./03-block-catalog.md): config에 등록된 블록/아이템 ID 82종의 분류와 동작
- [04-bevy-migration-baseline.md](./04-bevy-migration-baseline.md): Bevy 대응 설계, 단계별 이전 순서, 수용 기준
- [05-findings-and-open-questions.md](./05-findings-and-open-questions.md): 결함 후보, 불명확한 계약, 실기기 확인 항목
- [evidence-inventory.md](./evidence-inventory.md): 분석 범위와 근거 파일 목록

## 0단계 완료 정의

- 5개 빌드 씬과 전환 조건이 기록되어 있다.
- 플레이어 입력, 2D 물리, 아이템 큐, 블록 상호작용, 재시작/클리어 흐름이 기록되어 있다.
- 에디터의 배치 제약, 맵 직렬화 형식, 썸네일 생성 방식이 기록되어 있다.
- 서버 엔드포인트, 전송 필드, 주요 응답 DTO가 기록되어 있다.
- Unity 전역 정적 상태를 Bevy Resource/Component/Event로 옮길 기준이 제시되어 있다.
- 코드에서 확정할 수 없는 내용은 추측으로 확정하지 않고 확인 항목으로 분리되어 있다.

## 범위 경계

이번 단계에는 Rust 프로젝트 생성, Bevy 버전 선택, 에셋 변환, 서버 변경, Unity 원본 수정이 포함되지 않는다. 런타임 실행/플레이테스트도 수행하지 않았으므로 프레임 단위 물리 감각과 UI 픽셀 일치는 다음 단계의 골든 테스트로 확인해야 한다.
