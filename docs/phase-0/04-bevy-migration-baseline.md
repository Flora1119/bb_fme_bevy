# Bevy 이전 기준선

## 1. 이전 원칙

1. 서버와 저장 맵의 wire contract를 먼저 고정한다.
2. Unity GameObject 계층을 그대로 흉내 내지 않고 데이터·시스템·표현을 분리한다.
3. 물리 감각은 “비슷함”이 아니라 골든 시나리오의 위치/속도 허용 오차로 검증한다.
4. 에디터 문서 상태와 플레이 시뮬레이션 상태를 명시적으로 분리한다.
5. 모든 block ID를 문자열 match로 흩뿌리지 않고 typed ID/registry로 집중시킨다.
6. Unity에서 확인된 결함은 호환 모드와 수정 모드를 구분해 결정한다.

## 2. 권장 크레이트/모듈 경계

Bevy 버전과 외부 physics crate는 1단계 착수 시 최신 호환성을 확인해 결정한다. 현재 단계에서는 다음 논리 경계를 고정한다.

bb_fme_app 앱 상태, 씬/화면 전환, 공용 카메라/로딩
bb_fme_domain BlockId, MapDocument, 옵션, 방향, 게임 규칙
bb_fme_assets sprite/prefab registry, config 로드, asset validation
bb_fme_physics 공 이동, 접촉 분류, 중력 반전, joint/rope adapter
bb_fme_gameplay 아이템 큐, 스위치, 포탈, 레이저, 장애물 시스템
bb_fme_editor 그리드 편집, 배치 제약, 옵션 UI, undo 후보
bb_fme_net HTTP form API, DTO, session/token
bb_fme_persistence map JSON serde, 로컬 설정, 썸네일 encode/decode
bb_fme_ui Main/Hub/Editor/Play 화면과 dialog stack
bb_fme_tests fixture, 골든 시뮬레이션, contract test

초기에는 하나의 Cargo workspace 안에 `app` + 소수 library crate로 시작해도 되지만, 위 의존 방향을 코드 module로라도 유지한다. `domain`은 Bevy/렌더러/HTTP에 의존하지 않는 것이 핵심이다.

## 3. Unity → Bevy 개념 대응

| Unity                                    | Bevy 기준                                                             |
| ---------------------------------------- | --------------------------------------------------------------------- |
| SceneManager scene                       | `States`/`SubStates` + 화면 root entity                               |
| additive MapPlay                         | Editor state 위 PlayTest substate 또는 별도 simulation world snapshot |
| MonoBehaviour `Start/Update/FixedUpdate` | `OnEnter`, `Update`, `FixedUpdate` schedule system                    |
| static singleton/state                   | 작은 typed `Resource`                                                 |
| GameObject prefab                        | data-driven block definition + spawn bundle/system                    |
| `CurrentBlockData` dictionary            | `GridIndex: HashMap<IVec2, Entity>` + components                      |
| interface `IActivatable` 등              | marker/component + event-driven systems                               |
| Coroutine/WaitForSeconds                 | Timer component/resource + systems                                    |
| UnityEvent/Button callback               | UI interaction events                                                 |
| PlayerPrefs                              | platform persistence adapter                                          |
| Resources.LoadAll                        | asset manifest + Bevy AssetServer/loader                              |
| Rigidbody2D/Collider2D                   | 선택한 2D physics backend components                                  |
| DistanceJoint2D                          | distance/rope joint adapter 또는 deterministic custom constraint      |
| LineRenderer+EdgeCollider                | laser path resource + mesh/line rendering + segment colliders         |

## 4. 데이터 모델 초안

```rust
struct MapDocument {
    map_name: String,
    author: String,
    settings: MapSettings,
    blocks: Vec<BlockEntry>,
    block_options: Vec<BlockOptionEntry>,
}

struct BlockEntry {
    position: IVec2,
    kind: BlockKind,
    id: BlockId,
    direction: CardinalDirection,
}

#[derive(Component)] struct GridPosition(IVec2);
#[derive(Component)] struct OriginGridPosition(IVec2);
#[derive(Component)] struct BlockIdComponent(BlockId);
#[derive(Component)] struct Inactive;
#[derive(Component)] struct PlayerBall;
#[derive(Component)] struct CloneBall;

#[derive(Resource)] struct GridIndex(HashMap<IVec2, Entity>);
#[derive(Resource)] struct GravityState { direction: GravityDirection, scale: f32 }
#[derive(Resource)] struct PlaySession { stars: u32, cleared: bool, checkpoint: Option<IVec2> }
```

Serde layer에서는 현행 문자열/숫자를 그대로 받고 domain 변환에서 validation한다. 알 수 없는 block ID는 `Unknown(String)`으로 보존해 round-trip 손실을 막고, spawn 단계에서 명시적 경고/대체 표시를 제공하는 편이 현행의 조용한 skip보다 안전하다.

## 5. 시스템 순서 기준

FixedUpdate의 권장 system set:

CollectInputIntent
→ ApplyPlayerForces
→ PhysicsStep
→ CollectContacts
→ ClassifyContacts
→ ApplyDamageAndBounds
→ ApplyBounce
→ ApplySteppableEffects
→ ApplyTriggerEffects
→ ResolveGridMoveCommands
→ RefreshDirtyLasers
→ EvaluateClearCondition

Update schedule:

UI interaction / dialog state
Timers and coroutine-equivalent state machines
Camera follow
Aim and rope visuals
Network task polling
Presentation sync

명시적 ordering과 deferred command 경계를 두어 Unity callback 순서 의존을 줄인다.

## 6. 단계별 구현 순서

### 1단계: 호환 코어

- Cargo workspace와 CI 생성
- 현행 맵/응답 fixture 수집
- MapDocument serde round-trip
- block config parser와 모든 ID/옵션 validation
- 방향/좌표/시간 formatting unit test
- 기존 PNG sprite 로드 PoC

완료 기준: 실제 Unity 맵 JSON을 읽고 재직렬화해 의미 필드가 보존되며, 82개 config ID를 모두 registry가 인식한다.

### 2단계: 최소 플레이 수직 슬라이스

- 단일 화면, orthographic camera, 25×15 맵
- 일반 블록/스파이크/ball/star
- 좌우 입력, 중력, bounce, 사망, 재시작, 클리어
- Border와 카메라 dead zone

완료 기준: 골든 맵에서 시작→이동→별 수집→클리어와 spike 사망→0.5초 재시작을 반복 재현한다.

### 3단계: 기능 블록과 능력

- 아이템 FIFO와 즉시 아이템
- jump/straight/gravity/clone/wall/free-dash/swing
- 기능/화이트/스위치/텔레포트/포탈
- deterministic timer components

완료 기준: 각 ID당 최소 1개 독립 테스트 맵과 재시작 후 상태 복원 테스트를 통과한다.

### 4단계: 동적 블록

- cannon/shell/fall rock/jumper/lift/dart/gear
- transport grid moves
- laser tracing, mirror, dirty refresh
- 복합 충돌/포탈 상호작용 정책 확정

완료 기준: 이동 중 grid index 불변식, projectile cleanup, laser collider/sprite 일치가 검증된다.

### 5단계: 에디터

- 카테고리/블록 선택, 회전, 배치/삭제, 옵션
- 배치 제약과 맵 크기/카메라
- JSON import/export
- 썸네일 렌더
- 플레이 테스트 snapshot/restore

완료 기준: Unity에서 저장한 맵을 Bevy에서 편집/저장 후 Unity가 다시 읽을 수 있고, 반대 방향도 성립한다.

### 6단계: 온라인/UI 완성

- Loading/Main/UserMapHub
- login/token/profile
- map list/info/reviews/records/like/rating/upload/save slots
- 모든 dialog/오류/로딩 상태

완료 기준: staging 서버에 대한 form contract test와 주요 사용자 여정 E2E가 통과한다.

### 7단계: 패리티·출시

- 시각 비교, physics tolerance 조정, 모바일/데스크톱 입력
- 로컬 설정/token migration
- 성능/메모리/네트워크 실패 복구
- Unity/Bevy 병행 비교 후 전환

## 7. 테스트 전략

### 순수 domain test

- 방향 변환 4/8방향
- map bounds와 grid occupancy
- option min/max/default 및 이름 mapping
- 포탈 속도 변환
- mirror outgoing direction
- transport pull/push/straight 결과
- item FIFO와 즉시형 분기

### 고정 시간 시뮬레이션

각 tick에서 `position`, `linear_velocity`, `gravity_direction`, `stars`, `active block IDs`를 기록한다. Unity 측 동일 시나리오의 로그를 golden trace로 얻고 Bevy 결과에 허용 오차를 둔다.

우선 시나리오:

- 무입력 낙하와 바닥 bounce 10회
- 좌/우 가속 및 release 감속
- 위 중력에서 바닥/천장 판정
- dash 중 단일 탭 제동
- free-dash 720° timeout
- swing attach/release/벽 모서리 anchor
- clone의 item 전달과 전체 정리
- 이동 블록 후 사망 재시작
- 스위치/문/투명별 초기화
- mirror 연쇄 laser

### 직렬화/서버 contract

- serde snapshot과 round-trip
- form key 정확성(`mapSeq` 포함)
- wire field rename(`map_Count`, `clear_time_ms`)
- Base64 PNG decode
- 빈/null/누락 data 대응

### 시각 회귀

- 5개 화면의 기준 해상도 screenshot
- 각 block ID의 방향 0..3 atlas
- editor grid/cursor/option dialog
- map thumbnail pixel dimensions와 aspect ratio
- red/cyan laser 및 death particle

## 8. 패리티 수용 기준 초안

- 맵 JSON: 알려진 필드 100% 보존, unknown field 정책 결정 및 테스트
- 블록 registry: config ID 100% 분류, 누락 asset 0
- 물리: 10초 골든 trace에서 위치/속도 오차 허용치를 시나리오별 합의
- 게임 규칙: 독립 block 테스트와 재시작 테스트 전부 통과
- 서버: staging의 성공/오류 응답 모두 contract test 통과
- UI: 핵심 사용자 여정에 막힌 버튼/닫히지 않는 dialog 없음
- 성능: 목표 기기 60 FPS, physics fixed tick 유지
- 원본 보전: Unity 프로젝트는 fixture/export 도구 추가가 별도 승인되기 전까지 변경하지 않음

## 9. 다음 착수 권장안

1단계 첫 작업은 Bevy 화면을 띄우는 것이 아니라 `MapDocument + block config` 호환 crate와 fixture test를 만드는 것이다. 이것이 서버 저장 데이터와 에디터 자산을 보호하는 가장 작은 안전 경계다. 그 다음 일반 블록/ball/star만 포함한 수직 슬라이스로 physics backend의 적합성을 검증한다.
