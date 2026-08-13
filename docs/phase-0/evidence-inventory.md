# 분석 근거와 인벤토리

## 분석 시점 상태

분석 대상 Git worktree에는 분석 시작 전부터 다음 수정이 존재했다. 본 작업은 이를 수정하거나 되돌리지 않았다.

- `BB_FME.slnx`
- `Packages/manifest.json`
- `Packages/packages-lock.json`
- `ProjectSettings/ProjectVersion.txt`

분석 문서는 Unity 원본 저장소 밖에서 작성했으며, 현재 Bevy 프로젝트의 `docs/phase-0`에 보관한다.

## 주요 근거

### 프로젝트/빌드

- `ProjectSettings/ProjectVersion.txt`
- `ProjectSettings/EditorBuildSettings.asset`
- `ProjectSettings/Physics2DSettings.asset`
- `ProjectSettings/TimeManager.asset`
- `ProjectSettings/TagManager.asset`
- `Packages/manifest.json`

### 씬

- `Assets/Scenes/Loading.unity`
- `Assets/Scenes/Main.unity`
- `Assets/Scenes/UserMapHub.unity`
- `Assets/Scenes/MapEditor.unity`
- `Assets/Scenes/MapPlay.unity`

### 코어/상태/계약

- `Assets/Scripts/Core/Loading.cs`
- `Assets/Scripts/Core/Main.cs`
- `Assets/Scripts/Core/PlayingManager.cs`
- `Assets/Scripts/Core/Settings.cs`
- `Assets/Scripts/Core/PlayerInputHandler.cs`
- `Assets/Scripts/Core/PlayerControls.cs`
- `Assets/Scripts/Utils/MapData.cs`
- `Assets/Scripts/Utils/MapEdit.cs`
- `Assets/Scripts/Utils/PlayState.cs`
- `Assets/Scripts/Utils/Transfer.cs`
- `Assets/Scripts/Utils/OptionApplier.cs`
- `Assets/Scripts/Utils/Network.cs`
- `Assets/Resources/block_assets_config.json`

### 에디터/플레이/허브

- `Assets/Scripts/MapEditor/*.cs`
- `Assets/Scripts/MapPlay/*.cs`
- `Assets/Scripts/UserMapHub/*.cs`

### 게임플레이

- `Assets/Scripts/Blocks/Core/*.cs`
- `Assets/Scripts/Blocks/Abilities/*.cs`
- `Assets/Scripts/Blocks/Interactions/*.cs`
- `Assets/Scripts/Blocks/Prefabs/*.cs`
- `Assets/Resources/mapdata/**/*.prefab`

## 정적 인벤토리

| 종류 | 수량 |
|---|---:|
| C# files | 56 |
| C# lines | 약 10,456 |
| Build scenes | 5 |
| All prefabs under Assets | 89 |
| Prefabs under Resources/mapdata | 82 |
| Configured block IDs | 82 |
| Images under Resources/sprites | 91 |
| Images under Assets total | 120 |
| Materials | 1 |
| PhysicsMaterial2D | 1 |
| Audio files | 0 |
| `.anim`/Animator Controller files | 0 |

## 분석 제한

- Unity Editor/빌드를 실행하지 않았다.
- 서버에 요청하지 않았다.
- Library/PackageCache 구현은 package version 확인 외에는 제품 코드 분석 범위에서 제외했다.
- prefab의 모든 collider 좌표/물리 property를 표 형태로 추출하지 않았다. 이는 1단계 fixture/asset conversion 전에 별도 기계 추출 대상으로 남긴다.
- UI 텍스트/anchor/색상의 픽셀 명세는 씬 YAML 전체 렌더 검증 전이므로 이번 문서에서는 사용자 흐름 중심으로 기록했다.

## 신뢰도 표기

- 코드와 config의 직접 값: 높음
- Unity event/collision callback 순서: 중간 이하, 실행 trace 필요
- 서버 field 단위/오류 결과: 중간, raw fixture 필요
- prefab collider가 결정하는 체감 물리: 중간 이하, 렌더/물리 캡처 필요
