# 맵·서버 계약 명세

## 1. 맵 JSON

현재 맵 저장 포맷은 Newtonsoft.Json이 public field 이름을 그대로 직렬화한 JSON이다. 별도 `schema_version`이 없으므로 필드명, 숫자 단위, 기본값 변경은 기존 서버 저장 맵과의 호환성을 깨뜨릴 수 있다.

```json
{
  "map_name": "editor_map",
  "author": "nickname",
  "map_settings": {
    "time_limit": 60.0,
    "show_time_ranking": true,
    "star_count": 3,
    "size": { "width": 25, "height": 15 },
    "tp1_exit": { "x": -1, "y": -1 },
    "tp2_exit": { "x": -1, "y": -1 },
    "portal1_positions": { "a_px": -1, "a_py": -1, "b_px": -1, "b_py": -1 },
    "portal2_positions": { "a_px": -1, "a_py": -1, "b_px": -1, "b_py": -1 },
    "sw_el": true,
    "sw_b1": true,
    "sw_b2": true
  },
  "blocks": [
    {
      "x": 2,
      "y": 3,
      "block": { "type": "block", "name": "b_normal", "dir": 0 }
    }
  ],
  "block_options": [
    {
      "x": 8,
      "y": 4,
      "name": "ob_cannon",
      "options": [
        { "value_name": "Delay", "value": 0.5 },
        { "value_name": "Interval", "value": 1.5 },
        { "value_name": "Value", "value": 8.0 }
      ]
    }
  ]
}
```

### 필드 규약

| 경로 | 형식 | 현재 의미 |
|---|---|---|
| `map_name` | string | 에디터 export 시 항상 `editor_map`; 업로드 제목은 API의 별도 `map_name` |
| `author` | string | export 시 현재 닉네임 |
| `map_settings.time_limit` | float | 초; 0은 제한 없음 |
| `show_time_ranking` | bool | stopwatch/랭킹 활성화 |
| `star_count` | int | 클리어에 필요한 별 수 |
| `size.width/height` | int | 논리 맵 크기 |
| `tp*_exit` | int 좌표 | 미배치 sentinel은 `(-1,-1)` |
| `portal*_positions` | int 좌표 2쌍 | A/B 미배치도 각 `(-1,-1)` |
| `sw_el/sw_b1/sw_b2` | bool | 스위치 군 초기 상태 |
| `blocks[].x/y` | int | 그리드 좌표 |
| `blocks[].block.type` | string | 9개 카테고리 중 하나 |
| `blocks[].block.name` | string | 리소스/프리팹 ID |
| `blocks[].block.dir` | int | 0..3, 위에서 시계 방향 |
| `block_options[].options` | ordered array | 이름/float 값; C#에서는 Queue 사용 |

로드 시 prefab registry에 이름이 없는 블록은 오류 없이 건너뛴다. `mapData.blocks == null`은 에디터 로드에서 오류지만, MapPlay 로드는 `mapData == null`만 먼저 검사한다. `block_options`는 null 허용이다.

## 2. 런타임 맵 모델

`BlockData`는 직렬화 데이터 외에 다음 런타임 필드를 가진다.

- `origin_pos`: 최초 그리드 위치
- `current_pos`: transport 등에 의해 이동한 현재 위치
- `obj`: Unity GameObject 참조

`CurrentBlockData.blockDataList`의 key도 현재 위치다. `MoveBlock(from,to)`는 key와 `current_pos`, Transform을 함께 바꾸며, `RestoreAllBlockPositions`가 원위치로 되돌린다. Bevy에서는 최소한 아래 불변식을 유지해야 한다.

1. 점유 인덱스의 좌표와 `GridPosition` component는 항상 같다.
2. 영속 원점 `OriginGridPosition`은 플레이 중 바뀌지 않는다.
3. 한 좌표에는 최대 한 block entity가 있다.
4. 이동은 점유 검사 후 원자적으로 인덱스와 component를 갱신한다.
5. disable은 entity despawn이 아니라 재시작 가능한 inactive 상태다.

## 3. 블록 에셋 설정 계약

`Resources/block_assets_config.json`은 다음 최상위 필드를 요구한다.

- `blockGroups: { category: [block_id...] }`
- `blockTypeSettings: { category: { rotatable, colorable } }`
- `blockExceptions: [block_id...]`
- `blockRotatable: [block_id...]` — 실제 코드는 이름이 이 목록에 있으면 회전 가능으로 판단
- `gearPassableBlocks: [block_id...]`
- `optionBlocks: [block_id...]`
- `blockOptions: { block_id: [{value_name,min,max,default_value}...] }`

리소스 ID는 Sprite.name과 Prefab.name을 전역 key로 사용한다. 이름 충돌 시 최초 로드된 에셋이 유지된다. 스위치 UI 가상 ID `sw_el_on/off`, `sw_b1_on/off`, `sw_b2_on/off`는 prefab lookup에서 각 `sw_el`, `sw_b1`, `sw_b2`로 매핑한다.

## 4. 로컬 영속 데이터

Unity `PlayerPrefs` key:

| Key | 형식 | 기본값 | 의미 |
|---|---|---|---|
| `SoundEnabled` | int 0/1 | 1 | 사운드 토글. 현재 프로젝트에는 오디오 에셋/재생 코드가 없음 |
| `EffectEnabled` | int 0/1 | 1 | 이펙트 토글. 현재 사망 particle 표시와 직접 연결된 코드는 없음 |
| `Token` | string | 없음 | 로그인 토큰; check_token으로 세션 복구 |

Bevy 구현은 플랫폼별 저장소를 바꾸더라도 이 세 key의 마이그레이션 또는 1회 import 정책을 정해야 한다. Token은 평문 로컬 저장이라는 현행 의미론을 그대로 문서화하되, 새 구현에서 안전 저장소 사용 여부는 별도 결정 사항이다.

## 5. HTTP 공통 규약

- base URL: `http://203.245.28.91:{port}`
- production false(현재 소스 기본값): port `2331`
- production true: port `4797`
- POST encoding: Unity `WWWForm`이 만드는 form data
- GET: query string 또는 텍스트 파일
- 응답 envelope: `{ "result": string, "message": string?, "data": T? }`
- HTTP connection/protocol error만 transport error로 처리하며 timeout, retry, cancellation, 인증 header는 구현되어 있지 않다.
- JSON은 대체로 Newtonsoft.Json을 사용하지만 좋아요 응답만 JsonUtility를 사용한다.

Rust 클라이언트는 기존 서버 호환을 위해 JSON body로 바꾸지 말고 서버 확인 전까지 form field를 유지해야 한다.

## 6. 엔드포인트 목록

### 부트/인증/프로필

| Method | Path | Request | 주요 응답/효과 |
|---|---|---|---|
| GET | `/note/version.txt` | 없음 | raw string; `v1.0.3`과 완전 일치 비교 |
| GET | `/note/update-note.txt` | 없음 | raw text |
| GET | `/note/credits.txt` | 없음 | raw text |
| POST | `/api/check_token` | `token` | `valid` + `TokenResponse(email,nickname,user_seq)` 또는 `invalid` |
| POST | `/api/login` | `email,nickname,password` | `success` + `LoginResponse(token,user_seq)`; `error_no_result`, `error_pw_wrong` |
| POST | `/api/register` | `email,nickname,password` | `success` + `RegisterResponse(user_seq)`; `error_name_already_exists` |
| GET | `/api/get_profile?user_seq=...` | query `user_seq` | `GetProfileResponse(nickname,play_count,clear_count,map_Count,last_activity,created_at)` |

`map_Count`는 대문자 C를 포함한 현행 wire field이므로 그대로 호환해야 한다.

### 에디터 저장 슬롯

| Method | Path | Form fields | 주요 응답 |
|---|---|---|---|
| POST | `/api/save_map` | `user_seq,slot_index,json_data,thumbnail` | `success` |
| POST | `/api/load_map` | `user_seq,slot_index` | `success` + `LoadMapResponse(json_data)`; `error_no_result` |
| POST | `/api/get_thumbnail_saved` | 코드 사용처 기준 `user_seq,slot_index` | `LoadMapThumbnailResponse(thumbnail)` |

`thumbnail`은 PNG byte의 Base64 문자열이다.

### 맵 허브와 플레이

| Method | Path | Form fields | 주요 응답 |
|---|---|---|---|
| POST | `/api/get_map_list` | `sort,author,map_name,offset,limit` | `MapListWrapper.items[]` |
| POST | `/api/get_map_info` | `map_name,author` | `UserMapInfoResponse(map_seq,like_count,limit_seconds,show_ranking,thumbnail)` |
| POST | `/api/get_map_json` | `map_seq` | `GetMapJsonResponse(json_data)` |
| POST | `/api/like_map` | `user_seq,map_seq,nickname` | `success` 또는 `error_already_liked` 의도 |
| POST | `/api/get_map_reviews` | `mapSeq` | `ReviewListWrapper.reviews[]`; `no_reviews` |
| POST | `/api/get_timer_records` | `map_seq` | `TimerRecordListWrapper.records[]`; `no_records` |
| POST | `/api/get_top_timer_record` | `map_seq,user_seq` | `TopTimerRecordResponse(world_best,my_best)` |
| POST | `/api/check_rating` | `user_seq,map_seq` | `rated` 또는 `not_rated` |
| POST | `/api/submit_rating` | `user_seq,map_seq,nickname,rating,comment` | `success`, `error_insert`, `error_update` |
| POST | `/api/submit_timer_record` | `user_seq,map_seq,nickname,clear_time_ms` | `success`, `error_insert` |
| POST | `/api/upload_map` | `user_seq,map_name,nickname,limit_seconds,show_ranking,json_data,thumbnail` | `success`, `error_insert` |

주의: `get_map_reviews`만 map id 필드가 camelCase `mapSeq`이고 나머지는 `map_seq`다. 서버 호환 테스트에서 반드시 그대로 보존한다.

## 7. 네트워크 DTO

```text
MapListData       { map_name:string, author:string, avg_rating:float,
                    rating_count:int, play_count:int }
UserMapInfo       { map_seq:string, like_count:int, limit_seconds:float,
                    show_ranking:int, thumbnail:string }
ReviewData        { nickname:string, rating:float, comment:string, created_at:string }
TimerRecordData   { nickname:string, clear_time_ms:float, cleared_at:string }
TimerRecordTop    { nickname:string, clear_time_ms:float }
TimeRecordUser    { clear_time_ms:float }
```

`clear_time_ms`라는 이름과 달리 클라이언트는 `Time.deltaTime`으로 누적한 초 값을 그대로 전송하고 `FormatTime`에도 초로 전달한다. Rust 모델은 서버 실측 전까지 wire name과 논리 단위를 분리하여, 예를 들어 `clear_time_seconds` 내부 필드에 `#[serde(rename = "clear_time_ms")]`를 적용하는 편이 안전하다.

## 8. 호환성 테스트 벡터

다음 fixture를 Rust 포팅 전에 확보해야 한다.

- 서버 슬롯에서 실제 맵 JSON 최소 3개: 최소 맵, 모든 블록 맵, 레거시 맵
- 실제 `get_map_list`, `get_map_info`, `get_top_timer_record` 성공/빈 결과 응답
- 로그인/토큰 만료/잘못된 비밀번호 응답
- 썸네일 Base64 샘플과 decode 후 width/height
- `block_options` 누락/null/빈 배열의 각 사례
- 미등록 block ID를 포함한 맵의 현행 Unity 로드 결과

이 fixture들은 Rust의 serde round-trip 및 HTTP contract test의 골든 데이터로 사용한다.
