# 4단계 — 기본 게임 수직 단면

1. 현재 연결된 플레이 흐름

현재 수직 단면은 다음 흐름까지 실제 런타임에 연결되어 있다.

```text
키보드 입력
    → PlayerInputIntent
    → PhysicsSchedule 수평 제어
    → Avian 충돌 해결
    → BB_FME 바닥·천장·벽 반응
    → PlayerBall Transform
    → PlayerCameraPlugin
    → 맵 범위에 제한된 카메라 추적
```

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
