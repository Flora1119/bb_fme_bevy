# 현재 구현 명세

## 1. 제품 개요

BB_FME(BounceBall FanMadeEdition)는 2D 그리드 기반 물리 퍼즐/플랫포머와 유저맵 제작·공유 기능을 결합한 Unity 클라이언트다. 현재 공식맵 플레이 진입은 UI에서 “준비 중”으로 막혀 있으며, 실사용 핵심은 다음 두 경로다.

1. 로그인 → 유저맵 허브 → 맵 선택 → 플레이 → 평가/기록 제출
2. 로그인 → 맵 에디터 → 제작/저장 → 애디티브 테스트 플레이 → 검증 후 업로드

정적 규모는 C# 56개(약 10,456행), 빌드 씬 5개, 프리팹 89개(그중 Resources 맵 프리팹 82개), Resources 스프라이트 이미지 91개다. 오디오 파일과 `.anim`/Animator Controller는 발견되지 않았고, 사망 이펙트는 ParticleSystem 프리팹이다.

## 2. 기술 기준선

- Unity Editor: `6000.3.9f1`
- 목표 프레임률: 60 FPS
- 고정 물리 스텝: 0.02초(50 Hz)
- 2D 중력 기본값: `(0, -9.81)`; 런타임에서 위/아래 반전
- Physics2D solver: velocity iterations 8, position iterations 3
- UI: uGUI (`UnityEngine.UI`)
- 입력: Unity Input System 1.18.0
- JSON: Newtonsoft.Json 3.2.2
- 핵심 렌더링: SpriteRenderer, LineRenderer, Orthographic Camera
- 물리: Rigidbody2D, Collider2D, DistanceJoint2D, PhysicsMaterial2D

Unity 패키지 manifest에는 Addressables가 직접 선언되어 있지 않지만 EditorBuildSettings의 config object에는 addressable asset 참조가 남아 있다. 실제 게임 리소스 로딩은 `Resources.Load/LoadAll` 기반이다.

## 3. 씬과 애플리케이션 흐름

빌드 순서는 `Loading → Main → UserMapHub → MapEditor → MapPlay`이다.

### Loading

- 영속 `PlayingManager` 초기화를 기다린다.
- 서버 `version.txt`와 클라이언트 `v1.0.3`을 문자열로 비교한다. 불일치하면 업데이트 다이얼로그를 띄우고 진행을 중단한다.
- `PlayerPrefs`에서 사운드/이펙트 토글과 토큰을 읽는다.
- 토큰이 있으면 서버 검증 후 로그인 전역 상태를 복구한다.
- 업데이트 노트와 크레딧을 내려받는다.
- `block_assets_config.json`, 모든 `Resources/sprites`, 모든 `Resources/mapdata`를 이름 기반 딕셔너리에 적재한다.
- Main을 비동기 로드하고 0.9 진행률에서 활성화한다.

네트워크 오류 콜백이 없는 노트/버전 요청은 실패해도 별도 복구 UI가 없다. 버전 요청 실패 시 `isNewUpdateAvailable`은 false로 유지되어 다음 단계로 진행한다.

### Main

- 로그인/로그아웃, 회원가입, 프로필, 설정, 업데이트 노트, 크레딧, 종료 UI를 제공한다.
- 유저맵 허브는 로그인 상태에서만 진입 가능하다.
- 공식 맵 버튼은 현재 미구현 안내만 표시한다.
- 배경 RawImage UV를 매 프레임 스크롤한다.

### UserMapHub

- 서버에서 20개 단위로 맵 목록을 페이징한다.
- 최신/평점/플레이 수 정렬 및 작성자/맵 이름 검색 진입점이 있다.
- 선택 맵의 썸네일, 좋아요, 제한 시간, 랭킹 여부를 조회한다.
- 댓글 목록과 타임어택 목록, 세계/개인 최고 기록을 조회한다.
- 선택 정보는 `SelectedMapInfo` 정적 상태에 저장한 뒤 MapPlay로 넘어간다.

### MapEditor

- 기본 맵 크기는 25×15, 최대 45×45다.
- 블록 카테고리/하위 블록 선택, 회전, 그리기, 지우기, 옵션 편집을 지원한다.
- 서버 슬롯에 JSON+PNG 썸네일을 저장하고 다시 불러온다.
- 플레이 조건을 통과하면 MapPlay 씬을 additive로 로드하여 같은 블록 오브젝트로 테스트한다.
- 테스트 종료 시 MapPlay만 unload하고 에디터 UI와 카메라를 복구한다.
- 테스트 클리어 후에만 업로드 버튼이 활성화된다.

### MapPlay

- Editor 모드는 이미 구성된 `CurrentBlockData`를 사용한다.
- UserCreated 모드는 선택한 map sequence로 JSON을 받은 뒤 프리팹을 생성한다.
- 맵 경계, 블록 옵션, 스위치 초기 상태, 플레이어, 카메라, 타이머를 설정하고 게임을 시작한다.
- 사망 시 0.5초 후 전체 상태를 재시작한다.
- 필요한 별 수를 모두 모으면 클리어하며, 모드에 따라 업로드 허용 또는 평가/기록 UI로 간다.

## 4. 영속 객체와 전역 상태

`PlayingManager`는 Loading 씬의 카메라 GameObject에 붙어 singleton으로 유지되며 자신, 로딩 Canvas, EventSystem을 `DontDestroyOnLoad` 처리한다. 다음을 소유한다.

- 메인 카메라
- 공용 로딩 스피너/오버레이
- 공용 EventSystem
- `PlayerInputHandler`
- 현재 맵 블록 부모 Transform
- 씬별 ESC 콜백 한 개

주요 정적 상태는 다음과 같다.

| Unity 상태 | 역할 | Bevy 이전 후보 |
|---|---|---|
| `GameSettings` | 로그인/토글/모드/현재 씬 | `AppSettings`, `GameFlow` Resource |
| `PlayerInfo` | 사용자 식별 정보 | `Session` Resource |
| `SelectedMapInfo` | 허브→플레이 전달값 | `SelectedMap` Resource |
| `ResourceCache` | 이름→Sprite/Prefab/옵션 | typed asset registry Resource |
| `EditorSettings` | 도구/선택/배치 제약 | `EditorState` Resource/State |
| `OriginSettings` | 맵의 불변 시작 설정 | `LoadedMap` Resource |
| `CurrentBlockData` | 좌표→런타임 블록 및 인덱스 | ECS entity + grid index Resource |
| `BallState` | 중력, 입력 상태 | `GravityState`, input Resource/Event |
| `PlayState` | 스위치/별/포탈/레이저/플레이어 | 여러 작은 Resource + Events |
| `SaveNLoadMapBuffer` | JSON/썸네일 씬간 버퍼 | explicit transfer Resource |

## 5. 입력 의미론

Input Action Map은 `GamePlay` 하나이며 다음 액션을 갖는다.

- `MoveLeft`: Left Arrow, A, gamepad left/right stick의 left
- `MoveRight`: Right Arrow, D, gamepad left/right stick의 right
- `Touch`: touchscreen press, mouse press
- `DoubleTap`: touchscreen/mouse MultiTap
- `SpaceBar`: keyboard Space
- `ESC`: keyboard Escape 및 플랫폼 back 입력

포인터는 화면 중앙 X를 기준으로 좌/우 입력으로 변환된다. 단일 press/release는 이동 유지와 특수 상태 확정/제동에 쓰이고, 같은 쪽의 0.3초 이내 재입력은 더블 입력으로 취급되어 현재 아이템을 사용한다. Space는 별도 press 경로로 연결된다. ESC는 0.5초 쿨다운을 가진다.

플레이어 의미론:

- 좌/우 유지: FixedUpdate에서 수평 힘 적용, 최대 수평 속도 5
- 무입력: 속도 0.5 이하는 즉시 0, 그 외 반대 방향 감속력 적용
- 단일 입력: 대시 제동, 자유대시/스윙 조준 확정, 직진 상태 중지
- 더블 좌/우: 큐의 현재 능력을 해당 방향으로 사용

## 6. 플레이어 및 공 물리

`BallBase`가 플레이어와 클론의 공통 충돌 동작을 제공한다.

- 바닥 판정: 접촉 법선과 현재 바닥 방향 dot가 0.7 초과
- 천장 판정: dot가 0.7 초과
- 벽 판정: 수평 dot 절댓값이 0.9 초과
- 최소 바운스 속도: 9.5
- 벽 최소 바운스 속도: 12, 감쇠 비율 0.8
- 런타임 Rigidbody gravityScale 기본값: 3
- 전역 중력 방향 변경 이벤트를 구독하여 각 공에 반영
- `Damage` 태그 trigger 접촉 시 사망
- `Border` 태그 접촉 시 사망/제거
- 밟은 블록은 그리드 좌표로 `CurrentBlockData`를 조회한 뒤 기능/화이트/PrefabBlock 인터페이스로 분배

사망 시 플레이어 sprite와 물리를 끄고 DeathEffect를 생성한다. 모든 클론을 제거하고 0.5초 뒤 재시작한다. 클론은 동일한 BallBase 물리를 사용하고, 획득 아이템을 원 플레이어 능력 큐로 전달하며, 클론 사망은 클론만 제거한다.

## 7. 능력 큐

일반 아이템은 FIFO 큐에 들어가며, 현재 능력이 없을 때 큐 첫 항목을 활성 슬롯으로 이동한다. 사용 시 다음 항목으로 넘어간다. 즉시형은 큐를 사용하지 않는다.

| ID | 동작 |
|---|---|
| `i_jump` | 중력 반대 방향으로 속도 12 설정 |
| `i_dash` | 선택한 수평 방향 속도 15, 위쪽 보정 최소 3; 0.15초 상태, 0.4초 쿨다운 |
| `i_circle` | 물리를 Kinematic으로 두고 초당 180° 조준 후 임의 방향 대시 |
| `i_st` | 선택한 수평 방향으로 속도 10의 직진 상태 |
| `i_ginvert` | 전역 중력 Up/Down 반전 |
| `i_tp` | 획득 위치를 체크포인트로 저장하고 사용 시 복귀 |
| `i_clone` | 점프 후 클론 공 생성 |
| `i_wall` | 선택 방향 최대 2블록 Raycast 후 벽 반대편 안전 위치로 워프 |
| `i_swing` | 조준 후 최대 8 거리 Raycast, DistanceJoint2D 로프 부착 |
| `i_off` | 즉시 투명 상태; 알파 0.5, 별 수집 억제 |
| `i_on` | 즉시 투명 상태 해제 |
| `i_gup` | 즉시 gravityScale 1.5 감소 |
| `i_gdown` | 즉시 gravityScale 1.5 증가 |

자유대시/스윙 조준은 최대 2회전(720°)을 넘기면 플레이어를 사망시킨다. 스윙은 입력 방향으로 접선력을 주고 무입력 시 0.98 배 감쇠하며, 블록 충돌 시 해제된다.

## 8. 맵 플레이 수명주기

게임 시작 순서:

1. `PlayState.InitializePlayState`: 원본 포탈/텔레포트/스위치 복사, 레이저 초기화
2. 블록 옵션 적용
3. 각 PrefabBlock에 방향/좌표 전달
4. 각 IActivatable 활성화
5. 초기 스위치 상태 전파
6. 플레이어와 카메라 배치, 플레이 모드 활성화
7. 제한 타이머/스톱워치 시작

재시작 순서:

1. 클리어/타이머/플레이 상태 초기화
2. 이동 블록을 `origin_pos`로 복원
3. 클론 제거, 플레이어 정지/원위치
4. 비활성 아이템/블록 재활성화
5. 블록별 `DeactivateBlock`으로 코루틴/발사체/상태 정리
6. 원본 스위치 복원, 레이저 재생성
7. 다시 시작 수명주기 수행

클리어 조건은 `currentStarCount >= OriginSettings.starCount`이다. 제한 시간은 0보다 클 때만 동작하며 0 도달 시 플레이어를 죽인다. 랭킹 사용 시 별도 stopwatch를 0부터 올린다.

## 9. 카메라와 좌표계

- 블록 단위: 1 world unit
- 방향: `0=위, 1=오른쪽, 2=아래, 3=왼쪽`; 회전은 `-90° × dir`
- 8방향 시계: 0 위, 1 우상, 2 우, 3 우하, 4 하, 5 좌하, 6 좌, 7 좌상
- 카메라 Z: -10
- 기본 화면 논리 크기: 25×15
- 플레이 카메라 X는 플레이어를 즉시 추적하되 맵 범위로 clamp
- Y는 중심 ±3 dead zone을 벗어날 때만 이동
- 작은 맵은 기준 중심 `(12, 7)`에 고정
- 맵 경계 collider는 맵 외곽에서 2 unit offset/2 unit thickness를 사용

## 10. 에디터 규칙

- 단일 그리드 좌표에는 블록 하나만 허용된다.
- player `ball`은 하나만 배치 가능하다.
- `star_empty`는 별 스위치(`wb_star_sw`)와 쌍을 이루도록 배치 상태를 추적한다.
- 텔레포트 1/2 출구는 각 하나만 추적한다.
- 포탈 1/2는 A/B 두 위치를 추적한다.
- 스위치, 별 스위치, transport point 위치는 별도 인덱스로 유지한다.
- 회전 가능 여부, 목록 제외, 옵션 가능 여부는 `block_assets_config.json`이 정의한다.
- 블록 옵션 UI는 최대 3개 값을 지원하며 min/max/default를 config에서 읽는다.
- 썸네일은 맵 크기 × 블록당 20 px의 RGBA PNG로 렌더링하고 Base64로 저장한다.

에디터 테스트는 별도 데이터 복사가 아니라 같은 `CurrentBlockData`와 블록 오브젝트를 사용한다. 따라서 Bevy 포팅에서도 “편집 문서”와 “플레이 월드”를 명시적으로 복제하거나, 현행과 동일한 복원 규약을 보장해야 한다.
