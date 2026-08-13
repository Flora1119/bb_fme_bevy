# 발견 사항과 미확정 항목

## 분류

- **확정**: 소스/에셋에서 직접 확인됨
- **결함 후보**: 코드상 의도와 결과가 어긋나 보이나 런타임 재현 전
- **결정 필요**: Rust 포팅 시 제품 결정이 필요한 항목
- **실행 확인**: 정적 분석으로 확정할 수 없음

## 우선순위 높은 결함 후보

### F-01 `ob_gear` 옵션 이름 불일치

- 확정: config는 `Speed`, `CounterClockWise`를 저장한다.
- 확정: `ApplyGearMoveOptions`는 `MoveSpeed`, `CounterClockWise`를 match한다.
- 결과 후보: Speed 값은 적용되지 않고 prefab 기본 `moveSpeed=3`이 유지된다.
- 이전 결정: Unity 버그 호환 또는 의도대로 수정 중 하나를 fixture로 명시한다.

### F-02 좋아요 성공 분기 역전

`SubmitLike`는 `result != "success"`일 때 “좋아요 성공”을 표시하고, 뒤의 `else if result == "error_already_liked"`는 도달할 수 없다. 현행 UI 오류로 보인다.

### F-03 입력 release 이벤트 구독 해제 실패 가능성

PlayerController는 `OnRelease += () => ...`와 `OnRelease -= () => ...`에 서로 다른 lambda instance를 사용한다. 비활성/활성 반복 시 handler가 누적될 수 있다.

### F-04 시계 8방향 계산

`fb_clock_d8`의 `arrowDir`은 0..7인데 LaunchPlayer는 먼저 `DirToVector2(arrowDir)`(0..3만 지원)을 호출한 뒤 대각 변환한다. 4..7은 zero가 되어 절반 방향이 동작하지 않을 가능성이 크다. `DirToAllDirectionVector2`는 별도로 존재하지만 여기서 사용되지 않는다.

### F-05 스위치 전역 상태 이중화

`PlayState.current_sw_el/b1/b2`는 초기화되지만 SwitchBlock을 밟을 때 직접 변경되지 않는다. 실제 표시 상태는 각 component `isEnable`로 전파된다. 다른 시스템이 `current_sw_*`를 읽으면 오래된 값을 볼 수 있다.

### F-06 레이저 근방 갱신의 전체 삭제

`RefreshLasersNear`는 모든 beam을 삭제한 후 반경 안 source만 재생성한다. 반경 밖 source의 beam은 사라질 수 있다. 이름상 최적화 의도와 결과가 다르다.

### F-07 코루틴 중지 방식

`BlockMoveAnimator.StartMove`는 이미 이동 중일 때 새로 만든 `MoveCoroutine(...)` enumerator를 `StopCoroutine`에 넘긴다. 실행 중 instance와 다르므로 이전 코루틴이 중단되지 않을 가능성이 있다.

### F-08 클리어 중복 판정

클론의 별 수집은 count를 올리지만 PlayerController와 달리 즉시 클리어 조건을 검사하지 않는다. 클론이 마지막 별을 먹은 경우 다음 플레이어 item trigger 전까지 클리어되지 않을 수 있다.

### F-09 맵 로드 실패 후 stale buffer 사용

`LoadMapCoroutine`은 서버 로드 coroutine 뒤에 성공 여부와 관계없이 `SaveNLoadMapBuffer.jsonData`를 `JsonToMapData`에 전달한다. 이전 JSON이 남아 있으면 실패 후 과거 맵이 로드될 수 있다.

### F-10 `ResourceCache.optionBlockNames` 재로드 초기화 누락

`LoadAllResources`는 다수 list를 Clear하지만 `optionBlockNames`는 Clear하지 않는다. 재호출 시 중복이 누적된다. 현재 일반 부트에서는 한 번만 호출된다.

## 계약/단위 모호성

### F-11 `clear_time_ms`는 실제로 초

클라이언트 timer는 초를 누적하고, 같은 float를 `clear_time_ms`라는 form/DTO field로 전송·표시한다. 서버가 실제로 초를 저장하는지 확인이 필요하다.

### F-12 맵 크기와 좌표 상한

transport의 in-map 검사는 `x <= width`, `y <= height`다. 일반적으로 width 25라면 valid x가 0..24인지 0..25인지 불명확하다. border/camera/thumbnail 계산과 실제 에디터 배치 범위를 골든 맵으로 확정해야 한다.

### F-13 `blockRotatable` 명칭/주석 불일치

필드 이름과 `IsRotatableBlock`은 allow-list로 사용하지만 일부 주석은 “회전 불가능한 이름”이라고 서술한다. UI 실동작이 최종 기준이다.

### F-14 map format version 없음

기존 저장 데이터 migration 지점을 식별할 수 없다. Bevy가 새 필드를 추가할 때 외부 envelope 또는 optional `schema_version` 도입 정책이 필요하다.

### F-15 unknown block의 조용한 누락

로드 시 registry에 없는 prefab ID는 skip된다. 저장 후 다시 내보내면 해당 블록이 유실된다. 호환을 그대로 유지할지 unknown placeholder로 보존할지 결정해야 한다.

### F-16 HTTP/보안

- base URL은 HTTP 고정 IP다.
- 토큰은 PlayerPrefs 평문 저장이다.
- timeout/retry/cancellation이 없다.
- production은 소스 bool 수동 변경이다.

서버를 그대로 두는 포팅과 보안/배포 구조 개선을 같은 작업으로 묶을지 분리할지 결정해야 한다.

## 미완성/범위 불명확 기능

- 공식 맵 플레이는 Main에서 준비 중 처리된다.
- 포탈 코드 주석은 transport, laser, 고정 중력 오브젝트 상호작용이 미완성임을 명시한다.
- `SoundToggle`, `EffectToggle` UI/저장은 있으나 오디오 에셋/재생 코드가 없고 effect toggle도 death effect와 연결되지 않는다.
- `GameBalance.ItemPriority`는 정의되어 있으나 능력 선택은 FIFO다.
- `isProduction=false`가 소스 기본값이며 실제 배포 빌드 과정은 저장소에서 확인되지 않았다.
- 자동화 테스트/테스트 assembly는 Assets에서 발견되지 않았다.

## 실기기/Unity 실행으로 캡처할 항목

원본 수정 없이 Unity Editor 또는 빌드에서 다음을 영상·로그·스크린샷으로 확보해야 한다.

1. 5개 씬의 기준 해상도 전체 화면과 dialog 상태
2. player Rigidbody mass/drag/material 및 각 prefab collider 형상
3. 바닥/벽 bounce의 10초 position/velocity trace
4. 터치 single/double과 mouse MultiTap의 실제 event 순서
5. `fb_clock_d8` 8방향 발사 결과
6. gear Speed 옵션 변경 전후 결과
7. 스위치/문/별 스위치 재시작 결과
8. 먼 두 laser source 중 하나 주변 블록 이동 결과
9. 클론이 마지막 별을 먹는 결과
10. 맵 width/height의 가장자리 배치 가능 좌표
11. 모든 HTTP 성공/오류 raw response fixture
12. 실제 서버 save slot의 JSON과 thumbnail

## 포팅 전 제품 결정 체크리스트

- [ ] 현행 결함을 그대로 보존하는 compatibility mode가 필요한가?
- [ ] Rust 클라이언트도 기존 서버를 그대로 사용하는가?
- [ ] HTTP→HTTPS와 token 저장 개선은 이전 범위에 포함되는가?
- [ ] desktop/mobile/web 중 1차 목표 플랫폼은 무엇인가?
- [ ] physics backend 선택에서 완전한 Unity 감각과 구현 속도 중 우선순위는 무엇인가?
- [ ] 에디터 테스트는 snapshot 복제인가, 현행처럼 같은 entity 복원인가?
- [ ] unknown/legacy block을 보존할 것인가?
- [ ] map schema version을 언제 도입할 것인가?
- [ ] 공식 맵 플레이는 포팅 범위인가?
- [ ] 사운드/이펙트 토글의 미구현 기능도 새로 구현할 것인가?
