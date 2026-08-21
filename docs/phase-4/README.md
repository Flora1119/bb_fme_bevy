# 4단계 — 기본 게임 수직 단면

1. 현재 연결된 플레이 흐름

현재 수직 단면은 다음 흐름까지 실제 런타임에 연결되어 있다.

키보드 입력
→ PlayerInputIntent
→ PhysicsSchedule 수평 제어
→ Avian 충돌 해결
→ BB_FME 바닥·천장·벽 반응
→ PlayerBall Transform
→ PlayerCameraPlugin
→ 맵 범위에 제한된 카메라 추적

현재 입력은 `A`/`←`, `D`/`→`를 지원한다.

| 항목                  |           값 |
| --------------------- | -----------: |
| 일반 제어 목표 속도   |          5.0 |
| 입력 가속도           | 20.0 unit/s² |
| 무입력 감속도         |  8.0 unit/s² |
| 정지 임계값           |          0.5 |
| 물리 주기             |        50 Hz |
| 바닥 최소 바운스 속도 |          9.5 |
| 벽 최소 반동 속도     |          3.0 |
| 벽 반동 감쇠율        |          0.3 |
| 카메라 최소 표시 폭   |         25.0 |
| 카메라 최소 표시 높이 |         15.0 |
| 카메라 수직 dead zone |          3.0 |

일반 제어 목표 속도 5.0은 전역 속도 제한이 아니다.

벽 반동이나 이후 능력으로 일반 제어 범위를 넘는 외부 속도를 얻은 경우,
같은 방향 입력이 그 속도를 즉시 5.0으로 잘라내지 않는다.

현재 벽 반동은 충돌 직전 벽 방향 입사 속도에 0.3을 곱하고,
그 결과와 최소 반동 속도 3.0 중 큰 값을 사용한다.

2. 벽 반동과 일반 입력의 자동 검증

tests/player_control_wall_response.rs는 실제 Avian 물리 스케줄에서
왼쪽과 오른쪽 벽 충돌을 발생시킨 뒤 일반 입력과 벽 반동의 합성을 검증한다.

현재 테스트의 벽 입사 속도는 5.0이므로 반동 직후 속도는 3.0이다.

50 Hz에서 각 제어량은 다음과 같다.

| 상태              |     틱당 변화 |              10틱 후 |
| ----------------- | ------------: | -------------------: |
| 반동 방향 입력    |      0.4 가속 |   제어 목표 속도 5.0 |
| 무입력            |     0.16 감속 |     반동 속력 약 1.4 |
| 다시 벽 방향 입력 | 0.4 방향 전환 | 반대 방향으로 약 1.0 |

개별 검증:

cargo test --test player_control_wall_response

3. 카메라 규칙

카메라는 PlayerCameraPlugin이 PostUpdate에서 플레이어 위치를 읽어 이동시킨다.

수평축은 플레이어를 직접 추적하지만 화면이 맵 바깥으로 나가지 않도록
현재 viewport와 맵 크기를 기준으로 위치를 제한한다.

수직축에는 3.0 unit의 dead zone이 있다.

플레이어가 dead zone 안에 있는 동안에는 카메라 Y 위치를 유지하고,
범위를 벗어났을 때만 dead zone의 가장자리에 플레이어가 위치하도록 이동한다.

맵이 현재 viewport보다 작으면 해당 축의 카메라는 맵 중앙에 고정된다.

4. 카메라 ECS 통합 검증

tests/camera_follow.rs는 계산 함수만 호출하지 않고 실제 ECS 흐름을 구성한다.

SpawnValidatedMap
→ PlayWorld 생성
→ PlayerBall 생성
→ MapCamera 생성
→ PlayerBall Transform 변경
→ app.update()
→ PlayerCameraPlugin 실행
→ MapCamera Transform 검증

현재 통합 테스트는 다음 동작을 검증한다.

큰 맵의 좌우·상하 가장자리 clamp
수평 플레이어 추적
수직 dead zone 내부에서 카메라 정지
수직 dead zone을 벗어났을 때 카메라 이동

개별 검증:

cargo test --test camera_follow

5. 수동 체감 검증 맵

DevelopmentMapPlugin은 현재
assets/maps/phase5_camera_follow_sandbox.json을 사용한다.

cargo run으로 다음 항목을 직접 확인한다.

A/D 입력이 정상적으로 동작한다.
←/→ 입력이 A/D와 동일하게 동작한다.
좌우 벽에서 반동 방향과 세기가 대칭적이다.
벽 반동 뒤 같은 방향 입력에서 자연스럽게 제어 속도로 이어진다.
벽 반동 뒤 반대 방향 입력에서 순간 반전하지 않고 점진적으로 방향이 바뀐다.
입력을 놓으면 수평 속도가 자연스럽게 감속한다.
카메라가 플레이어의 수평 이동을 추적한다.
맵 왼쪽과 오른쪽 끝에서 화면이 맵 바깥으로 넘어가지 않는다.
수직 dead zone 안에서는 카메라가 불필요하게 흔들리지 않는다.
수직 dead zone을 벗어나면 카메라가 플레이어를 따라간다. 6. 체감 조정 판단 기준

현재 이동과 반동 값은 서로 다른 역할을 가진다.

반동 자체가 너무 강하거나 약하다면 일반 이동값이 아니라
MIN_WALL_BOUNCE_SPEED 또는 WALL_BOUNCE_DAMPING_RATIO를 조정한다.

평상시 좌우 이동이 둔하다면 PLAYER_HORIZONTAL_ACCELERATION을 조정한다.

키를 놓았을 때 너무 오래 미끄러진다면
PLAYER_HORIZONTAL_DECELERATION을 조정한다.

반동 후 반대 방향 전환만 별도로 조정할 필요가 생긴다면
일반 가속도를 무작정 변경하지 않고 별도의 방향 전환 규칙 도입을 검토한다.

카메라가 수직 바운스마다 지나치게 움직인다면
CAMERA_VERTICAL_DEAD_ZONE을 조정한다.

7. 체크포인트 완료 검증

카메라·입력 체크포인트를 닫기 전에 다음 검증을 모두 통과시킨다.

cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test --quiet
cargo test -- --ignored

수동 검증 결과도 이 문서에 기록한 뒤 다음 단계인
실제 Unity JSON 골든 fixture 확보로 이동한다.

## 8. Unity JSON 골든 fixture

Unity 원본 JSON 스키마와 Rust `MapDocument`의 실제 호환성을 검증하기 위해
Unity Editor에서 생성한 골든 fixture를 유지한다.

fixture: assets/maps/unity_phase4_vertical_slice.json

호환성 테스트: tests/map_document_compat.rs

fixture의 정확한 출처, 생성 방법, 검증 범위와 부동소수점 round-trip 정책은
다음 문서에 기록한다.

## 9. 최소 PlaySession과 결정적 상호작용 처리

게임 플레이 상태와 물리 상호작용의 실행 순서를 관리하기 위한
최소 PlaySession 구조를 추가했다.

현재 세션 상태는 다음 세 가지다.

Playing
Dead
Cleared

PlaySession은 현재 다음 플레이 진행 데이터를 관리한다.

state
collected_stars
elapsed_seconds

elapsed_seconds는 세션이 Playing일 때만 증가한다.

세션이 Dead 또는 Cleared 상태가 되면
플레이어 입력 수집과 수평 제어도 더 이상 진행되지 않는다.

상호작용 수집과 적용 분리

물리 이벤트가 도착하는 즉시 게임 상태를 변경하지 않는다.

게임 규칙에 영향을 주는 상호작용은 먼저
PendingPlayInteractions에 수집한 뒤,
같은 물리 틱 안에서 한 번에 정규화하고 처리한다.

현재 상호작용 종류는 다음과 같다.

Death
Movement
Collection
Switch

현재 결정적 우선순위는 다음과 같다.

Death
↓
Movement / Portal
↓
Collection
↓
Switch
↓
기존 물리 Bounce

동일 우선순위의 이벤트는 Entity 식별자를 기준으로 정렬한다.

동일한 Entity에서 같은 상호작용이 한 물리 틱에 여러 번 발생한 경우
한 번만 처리하도록 중복을 제거한다.

이를 통해 이벤트가 물리 엔진에서 들어오는 순서가 달라져도
게임 규칙의 결과가 달라지지 않도록 한다.

Death 우선 규칙

같은 물리 틱에 죽음과 수집이 동시에 발생하면 죽음이 우선한다.

예를 들어 플레이어가 같은 틱에 별과 가시에 동시에 접촉하면
Death를 먼저 처리하고 해당 틱의 나머지 게임 규칙 처리를 중단한다.

따라서 별은 증가하지 않는다.

PhysicsSchedule 실행 순서

현재 관련 시스템의 실행 순서는 다음과 같다.

PhysicsStepSystems::First
↓
PlaySessionSet::AdvanceTime
↓
Player horizontal control
↓
BroadPhase
↓
NarrowPhase
↓
PlayInteractionSet::Collect
↓
Solver
↓
PlayInteractionSet::Resolve
↓
기존 solid contact / bounce response
↓
Sleeping

PlaySessionSet::AdvanceTime을 별도 SystemSet으로 두어
세션 시간 갱신과 플레이어 제어의 데이터 접근 순서도 명시적으로 고정했다.

PlayInteractionSet::Resolve는 Solver 이후에 실행되고,
기존 바닥·천장·벽 반동 처리는 Resolve 이후에 실행된다.

이 체크포인트 당시 범위

이번 체크포인트에서는 게임 규칙을 처리할 기반만 만들었다.

아직 실제 별 Sensor 또는 가시 Sensor가
PendingPlayInteractions를 생성하도록 연결하지 않았다.

또한 Movement와 Switch는 향후 포탈 및 스위치 구현을 위한
우선순위 슬롯만 존재하며 실제 게임 규칙은 아직 적용하지 않는다.

다음 작업부터 실제 물리 접촉을 이 구조에 연결한다.

자동 검증

tests/play_session.rs는 실제 ECS 및 PhysicsSchedule에서 다음을 검증한다.

이벤트 입력 순서와 관계없이 Death가 Collection보다 우선한다.
같은 source의 Collection이 한 틱에 중복되면 한 번만 처리된다.
Playing 상태에서는 타이머가 진행된다.
Dead 상태에서는 타이머가 멈춘다.
Playing 상태에서는 수평 입력과 제어가 동작한다.
Dead 상태에서는 입력 intent가 0으로 돌아가고 수평 제어가 중지된다.

기존 수평 제어와 벽 반동의 회귀 검증은
tests/player_control_wall_response.rs에서 계속 수행한다.

검증 명령:

cargo test --test play_session
cargo test --test player_control_wall_response
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test --quiet

위 검증이 모두 통과하면
Phase 4의 최소 PlaySession 및 결정적 상호작용 처리 체크포인트를
완료한 것으로 본다.

## 10. 실제 별 수집

최소 PlaySession과 결정적 상호작용 처리 구조 위에
일반 별의 실제 물리 수집 흐름을 연결했다.

현재 일반 `star`는 MapSpawn 단계에서
`CollectibleStar` 역할을 부여받는다.

StarCollectionPlugin은 새로 생성된 CollectibleStar에
다음 물리 구성요소를 연결한다.

Sensor
CollisionEventsEnabled
Collider
StarSensorCollider

별 Collider는 Sensor이므로
플레이어와 물리적으로 충돌해 이동을 막지 않고,
접촉 시작 이벤트만 생성한다.

실제 수집 흐름은 다음과 같다.

PlayerBall
↓
CollectibleStar Sensor 접촉
↓
CollisionStart
↓
PlayInteraction::Collection
↓
PendingPlayInteractions
↓
결정적 Resolve
↓
PlaySession.collected_stars 증가
↓
CollectedStar
↓
Visibility::Hidden
↓
ColliderDisabled

Collection 이벤트는 물리 이벤트가 발생하는 즉시
PlaySession을 직접 변경하지 않는다.

기존 PlayInteractionSet::Collect에서 먼저 수집한 뒤,
PlayInteractionSet::Resolve에서 기존 우선순위 규칙에 따라 처리한다.

따라서 별 수집도 이전 체크포인트에서 정의한
Death → Movement → Collection → Switch 순서를 그대로 따른다.

수집된 별은 Entity 자체를 despawn하지 않는다.

대신 `CollectedStar`를 추가하고,
`Visibility::Hidden`으로 Sprite를 숨기며,
`ColliderDisabled`로 이후 충돌 감지를 중지한다.

이 방식은 GridIndex가 이미 제거된 Entity를 가리키는 문제를 피하고,
향후 재시작 시 기존 Entity를 개별 복원하지 않고
PlayWorld 전체를 재생성하는 구조와도 맞는다.

같은 별에서 Collection 이벤트가 중복 발생해도
한 물리 틱 안에서는 기존 정규화 단계에서 한 번만 처리된다.

또한 이미 CollectedStar 상태인 별은
후속 Collection 처리 대상에서 제외되므로
여러 물리 틱에 걸쳐 같은 별이 다시 증가하지 않는다.

현재 범위에서는 일반 `star`만 실제 수집 대상으로 연결한다.

`star_jump`, `star_empty` 등의 추가 별 종류는
이후 관련 블록 이식 단계에서 별도 동작과 함께 확장한다.

별 개수가 목표치에 도달했을 때의 Cleared 전환,
클리어 UI,
타이머 종료 처리는 아직 이번 체크포인트의 범위가 아니다.

### 자동 검증

`tests/star_collection.rs`는 실제 Avian 물리 충돌을 발생시켜
다음 동작을 검증한다.

- PlayerBall과 별 Sensor가 접촉하면 별이 한 번 수집된다.
- 수집 후 CollectedStar가 추가된다.
- 수집 후 Visibility가 Hidden이 된다.
- 수집 후 ColliderDisabled가 추가된다.
- 같은 위치에 계속 머물러도 같은 별은 다시 증가하지 않는다.
- 같은 물리 틱에 여러 별과 접촉하면 각각 한 번씩 수집된다.

기존 `tests/play_session.rs`도 함께 강화하여
Death가 Collection보다 우선할 경우
별의 카운트뿐 아니라 실제 별 Entity도 수집 상태로 전환되지 않는 것을 검증한다.

수동 검증에서는 development map에서 `cargo run`을 실행하고
플레이어가 일반 별에 접촉하는 순간 별 Sprite가 사라지는 것을 확인했다.

검증 명령:

cargo test --test play_session
cargo test --test star_collection
cargo test --test player_control_wall_response
cargo test --test block_colliders
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test --quiet

위 자동 검증과 수동 검증이 모두 통과했으므로
Phase 4의 실제 별 수집 체크포인트를 완료한 것으로 본다.

### 알려진 제한

- Dead 상태에서는 입력과 타이머가 정지하지만
  Avian 물리 자체는 계속 진행되므로 공은 계속 움직일 수 있다.
  최종 사망 연출과 물리 정지는 후속 UI/상태 작업에서 처리한다.

- HUD는 개발 검증용 최소 표시이며 최종 게임 UI가 아니다.

- 바닥/모서리 접촉은 원형 플레이어의 실제 접촉점을 기준으로
  중력 방향과 45도 이하를 바닥으로 분류한다.

- 45도 초과의 모서리 접촉은 커스텀 바운스를 적용하지 않고
  Avian 충돌 해결과 중력에 맡겨 옆으로 빠져나가도록 한다.

- 지속 접촉은 매 물리 틱 다시 분류하여
  드물게 바닥에 붙는 현상을 완화했다.
  극단적인 접촉점 조합의 추가 미세 조정은 후속 물리 튜닝으로 남긴다.

- Unity 골든 fixture는 실제 BB_FME C# DTO와 Newtonsoft.Json으로
  생성하지만, 만료된 서버 환경 때문에 전체
  MapEditor -> 서버 저장 -> 다운로드 흐름의 end-to-end fixture는 아니다.

- Movement/Portal 및 Switch 상호작용 슬롯은 우선순위만 존재하며
  실제 규칙은 후속 블록 구현 단계에서 연결한다.
