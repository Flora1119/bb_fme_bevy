# 블록·아이템 카탈로그

## 읽는 법

- ID와 분류는 `block_assets_config.json`을 기준으로 한다.
- `dir`은 0 위, 1 오른쪽, 2 아래, 3 왼쪽이다.
- “밟기”는 BallBase의 충돌 법선이 현재 중력 기준 바닥으로 판정될 때다.
- config에는 9개 카테고리, 총 82개 ID가 있다. 일부 ID는 상태별 sprite 이름이거나 발사체 이름이라 독립 배치 prefab이 아니다.

## item (17)

| ID | 현행 동작 |
|---|---|
| `ball` | 플레이어 시작점. 맵당 하나로 추적 |
| `star` | 수집 시 별 +1, 비활성화. 옵션 `Scale` |
| `star_empty` | 별 스위치가 눌린 동안만 collider가 켜지고 일반 별 sprite로 변함 |
| `star_jump` | 수집 시 점프 후 별 +1. 옵션 `Scale` |
| `i_dash` | 큐형; 더블 좌/우로 수평 대시 |
| `i_jump` | 큐형; 중력 반대 방향 점프 |
| `i_on` | 즉시형; 투명 상태 해제 |
| `i_off` | 즉시형; 반투명 상태 진입, 해당 중에는 플레이어의 별 수집 억제 |
| `i_wall` | 큐형; 선택 방향 벽 통과/최대 2칸 이동 |
| `i_tp` | 획득 위치를 체크포인트로 지정하는 큐형 복귀 아이템 |
| `i_circle` | 큐형; 회전 조준 후 자유 방향 대시 |
| `i_st` | 큐형; 선택 방향 수평 직진 |
| `i_clone` | 큐형; 점프하고 클론 공 생성 |
| `i_swing` | 큐형; 조준 후 블록에 로프 부착 |
| `i_ginvert` | 큐형; 전역 중력 방향 반전 |
| `i_gdown` | 즉시형; gravityScale +1.5(상한 경로 6, 최종 setter clamp 8) |
| `i_gup` | 즉시형; gravityScale -1.5(하한 경로 0, 최종 setter clamp 0.5) |

아이템 우선순위 딕셔너리(`i_swing` 1 … `i_clone` 9)가 존재하지만 현재 `ItemAbilityManager`는 이 값을 사용하지 않고 획득 순서 FIFO로 처리한다.

## block (4)

| ID | 현행 의미 |
|---|---|
| `b_normal` | 일반 고체 블록 |
| `b_o` | 시각/형상 변형 일반 블록 |
| `b_o_half` | 반 블록 형상 |
| `b_o_quarter` | 1/4 블록 형상 |

별도 스크립트 동작은 없고 prefab collider, sprite, rotation이 의미를 결정한다.

## spike (6)

`s_normal`, `s_half`, `s_b_normal`, `s_b_two`, `s_b_o_half`, `s_b_o_two`는 collider와 `Damage` 태그/프리팹 구성으로 공을 사망시키는 가시 블록 변형이다. 모양과 회전별 collider 형상은 Rust 이전 시 prefab YAML 또는 렌더 비교로 수치화해야 한다.

## funcblock (17)

| ID | 현행 동작 |
|---|---|
| `fb_jump` | 밟으면 점프 속도 14 |
| `fb_gd` | 밟으면 Rigidbody2D Dynamic으로 바뀌어 낙하하는 블록 |
| `fb_ds_jump` | 밟으면 점프 속도 16 후 블록 비활성화 |
| `fb_stone` | 활성 시 Dynamic인 낙하 돌 |
| `fb_fr` | 보이지 않는 낙석 spawner; Delay 뒤 Interval 주기로 FallRock 생성 |
| `fb_st_hv` | dir의 상하좌우로 속도 12 직진 |
| `fb_st_dg` | dir을 대각선으로 바꿔 속도 12 직진 |
| `fb_ds_st_hv` | 상하좌우 속도 15 직진 후 블록 비활성화 |
| `fb_ds_st_dg` | 대각선 속도 15 직진 후 블록 비활성화 |
| `fb_clock_d4` | 플레이어를 숨기고 고정, 0.3초마다 4방향 회전; press 시 속도 15 발사 |
| `fb_clock_d8` | 위와 같되 8방향 |
| `fb_tp1_in` | 텔레포트 1 입구; 설정된 출구 좌표로 Rigidbody 이동 |
| `fb_tp1_out` | 텔레포트 1 출구/목표 좌표 |
| `fb_tp2_in` | 텔레포트 2 입구 |
| `fb_tp2_out` | 텔레포트 2 출구/목표 좌표 |
| `fb_portal1` | 포탈 쌍 1의 A/B; 진입 속도를 입·출구 방향 좌표계로 변환 |
| `fb_portal2` | 포탈 쌍 2의 A/B |

포탈 구현은 Rigidbody2D가 있다고 가정하며, 진입 object/collider를 0.2초 동안 재진입 방지한다. 코드 주석상 transport/레이저/고정 중력 오브젝트와 포탈의 완전한 상호작용은 아직 보류 상태다.

## switch (12)

| ID군 | 현행 동작 |
|---|---|
| `sw_el_on`, `sw_el_off` | 에디터의 전기 스위치 상태별 선택 ID; 런타임 prefab은 `sw_el` |
| `el`, `el_off` | 전기 블록/문 상태 sprite 또는 collider를 `sw_el` 상태로 전환 |
| `el_b`, `el_b_off` | 전기 문 변형; 꺼질 때 dir 반대 한 칸으로 0.3초 이동 |
| `sw_b1_on`, `sw_b1_off`, `b1` | 블록 스위치 군 1; 같은 type 전부 동기화, 비문 블록은 alpha/collider 변화 |
| `sw_b2_on`, `sw_b2_off`, `b2` | 블록 스위치 군 2 |

스위치를 밟으면 해당 `switchType`의 전역 표시/문 상태가 갱신된다. 런타임 상태는 각 SwitchBlock의 `isEnable`과 `PlayState.current_sw_*`가 함께 존재하지만, `OnBlockStepped`는 PlayState의 `current_sw_*` 필드를 직접 갱신하지 않는다.

## whiteblock (6)

| ID | 현행 동작 |
|---|---|
| `wb_ds` | 밟으면 비활성화; 파란 포탄에도 파괴됨 |
| `wb_star_sw` | 공/발사체 등이 trigger 안에 있는 동안 모든 `star_empty` 활성화 |
| `wb_pp` | trigger 진입 시 dir 축 속도를 12로 설정하는 추진 블록 |
| `wb_change` | trigger 후 2초간 투명→빨강→파랑; 중간부터 Damage trigger, 끝에는 고체 Block |
| `wb_lift` | 같은 그룹을 이동시키는 lift 변형; 옵션 Speed/Direction |
| `wb_tpoint` | transport/lift 그룹 탐색용 위치 인덱스에 등록 |

## transport (7)

| ID | 현행 동작 |
|---|---|
| `tp_pull_a` | dir 앞에서 처음 만난 블록을 transport 앞까지 당김 |
| `tp_pull_o` | 처음 만난 블록을 한 칸 뒤로 당김 |
| `tp_push_a` | 처음 만난 블록을 다음 장애물/경계 직전까지 밀기 |
| `tp_push_o` | 처음 만난 블록을 한 칸 밀기 |
| `tp_st_a` | 자기 자신을 dir로 0.5초마다 빈 칸 끝까지 이동 |
| `tp_st_o` | 자기 자신을 dir로 한 칸 이동 |
| `tp_sticky` | 좌/우 면에 공을 붙이고 중력 방향으로 최대 1.2 속도로 미끄러뜨림; 입력 재개 시 벽점프 |

transport 검색은 최대 100칸 안전 제한을 두며 현재 `IsBlockMovable`은 항상 true다. 위치 검사는 `0 <= x <= width`, `0 <= y <= height`로 양 끝을 포함한다.

## laser (6)

| ID | 현행 동작 |
|---|---|
| `ls_start_red` | dir로 ray path를 생성하는 빨간 레이저; EdgeCollider trigger + Damage tag |
| `ls_start_blue` | 파란 레이저; EdgeCollider 비-trigger + Block layer, 즉 물리 장벽 |
| `ls_mirror_1` | 열린 면 2개(기본 0,2), 회전 반영 |
| `ls_mirror_2` | 열린 면 2개(기본 0,1), 회전 반영 |
| `ls_mirror_3` | 열린 면 3개(기본 0,1,2), 회전 반영 |
| `ls_mirror_4` | 네 방향 모두 열림 |

레이저는 격자 단위로 경로를 추적하고 LineRenderer+EdgeCollider2D를 동적 생성한다. 블록 이동 시 모든 beam을 지우고 반경 20 내 laser source만 다시 생성한다. 선 폭은 기본 0.2다.

## obstacle (7)

| ID | 현행 동작 |
|---|---|
| `ob_jumper` | dir 방향으로 주기 점프하는 Dynamic 블록; Delay/Interval/Value 옵션 |
| `ob_cannon` | Delay 뒤 Interval마다 빨간 포탄 자동 발사; 밟히거나 hit되면 파란 포탄 발사 |
| `ob_lift` | 개별 왕복 lift; Speed/Direction 옵션 |
| `ob_dart` | lift 계열 이동 + 회전 다트; Speed/Direction 옵션 |
| `ob_gear` | 인접 블록 외곽선을 손 법칙으로 따라 이동/회전; Speed/회전 방향 옵션 의도 |
| `ob_shell_b` | 파란 포탄 sprite/ID; 독립 배치 목록에서는 exception |
| `ob_shell_r` | 빨간 포탄 sprite/ID; 독립 배치 목록에서는 exception |

빨간/파란 포탄 모두 Damage tag로 공을 죽이고 블록 또는 Border 접촉 시 제거된다. 파란 포탄은 추가로 `fb_ds_jump`, `fb_ds_st_dg`, `fb_ds_st_hv`, `wb_ds`를 비활성화한다. 대포, 낙석, lift 간 일부 상호작용은 prefab trigger 구성과 component callback에 의존한다.

## 옵션 계약

| Block | Options (min..max, default) |
|---|---|
| `star`, `star_jump` | `Scale` 0.5..1, 1 |
| `fb_fr` | `Delay` 0..10, 0.5; `Interval` 0.3..10, 1 |
| `wb_lift` | `Speed` 0..10, 3; `Direction` 0..3, 0 |
| `ob_jumper` | `Delay` 0..10, 0.5; `Interval` 0..10, 2; `Value` 0..20, 12 |
| `ob_cannon` | `Delay` 0..10, 0.5; `Interval` 0..10, 1.5; `Value` 0..15, 8 |
| `ob_dart`, `ob_lift` | `Speed` 0..10, 3; `Direction` 0..3, 0 |
| `ob_gear` | `Speed` 0..10, 3; `CounterClockWise` 0..1, 0 |

옵션은 float로 저장되며 Direction/CounterClockWise만 적용 시 int/bool로 변환한다. 현재 option applier와 config의 이름 불일치는 별도 findings 문서에 기록했다.

## Bevy에서 보존할 상호작용 우선순위

한 물리 tick에 여러 접촉이 생길 수 있으므로 다음 우선순위를 골든 테스트로 확정해야 한다.

1. Damage/Border 사망
2. 바닥/벽/천장 bounce 계산
3. Steppable block 효과
4. Trigger item/white/prefab 효과
5. 별 수집과 클리어 판정
6. 이동 블록/레이저 재생성 command 적용

Unity callback 호출 순서는 명시적이지 않으므로 Rust에서는 event 수집 → deterministic sorting → command 적용의 두 단계로 만드는 것을 권장한다.
