# Unity JSON 골든 fixture

## 목적

`assets/maps/unity_phase4_vertical_slice.json`은 BB_FME Unity 프로젝트가 사용하는
실제 C# JSON DTO와 Newtonsoft.Json 직렬화 방식을 기준으로 생성한 호환성 검증용
골든 fixture이다.

이 fixture의 목적은 Rust/Bevy 포트가 Unity 맵 데이터의 다음 정보를 손실 없이
읽을 수 있는지 검증하는 것이다.

- 맵 메타데이터와 설정
- 블록 좌표
- 블록 카테고리
- 블록 ID
- 블록 방향
- 블록 옵션
- 역직렬화 후 재직렬화 가능한 데이터 의미

## Unity 원본 기준

생성 기준 Unity 저장소:

Flora1119/BB_FME

확인한 Git 기준선:

5c154ecbd3cf3db40e13bf48440011d33cb335f1

관련 Unity 소스:

Assets/Scripts/Utils/MapData.cs
Assets/Scripts/Utils/Transfer.cs
Assets/Resources/block_assets_config.json

MapData.cs에는 Unity 맵 JSON에 사용되는 실제 DTO가 정의되어 있다.

주요 타입:

MapJsonDataInfo
Data_MapSettings
Data_Size
Data_Position
Data_Portal
Data_BlockEntry
Data_BlockData
Data_BlockOption
Data_BlockOptionInfoValue

Unity의 실제 Transfer.ExportMapToJson() 역시 이 DTO를 구성한 다음 다음 방식으로
JSON을 생성한다.

string json = JsonConvert.SerializeObject(map, Formatting.Indented);
SaveNLoadMapBuffer.jsonData = json;

따라서 fixture 생성에서도 동일한 Unity DTO와 동일한 Newtonsoft.Json serializer를
사용했다.

생성 환경에 대한 제한

2026-08-17 기준 기존 BB_FME 서버 계약이 만료되어 전체 Unity 게임의 서버 의존
실행 흐름을 사용할 수 없었다.

따라서 이번 fixture는 실제 MapEditor의 Play Mode 저장 동작이나 서버 저장 API를
통해 캡처한 파일이 아니다.

대신 Unity Editor의 Edit Mode에서 임시 Editor 스크립트를 실행하여:

실제 Unity MapJsonDataInfo DTO
-> 실제 Data\_\* DTO
-> Newtonsoft.Json
-> JsonConvert.SerializeObject(..., Formatting.Indented)
-> JSON 파일

순서로 생성했다.

즉 이 fixture가 직접 증명하는 범위는 다음과 같다.

Unity의 실제 JSON 스키마

- Unity가 사용하는 실제 C# 필드 타입
- Unity가 사용하는 실제 Newtonsoft.Json 직렬화
  ↓
  Rust MapDocument 호환성

반면 다음 경로 자체를 end-to-end로 검증하는 fixture는 아니다.

Unity MapEditor
-> CurrentBlockData
-> Transfer.ExportMapToJson()
-> 서버 저장

서버 또는 원본 런타임 실행 환경을 다시 확보할 수 있다면
향후 실제 MapEditor export 결과를 추가 fixture로 보강할 수 있다.

fixture 구성

fixture 이름: assets/maps/unity_phase4_vertical_slice.json

맵 크기: 25 x 15

필수 별 개수: 1

포함 블록:
| 위치 | 카테고리 | ID | 방향 |
| --------- | ----------- | ---------- | -: |
| `(2, 2)` | `item` | `ball` | 0 |
| `(6, 2)` | `item` | `star` | 0 |
| `(2, 0)` | `block` | `b_normal` | 0 |
| `(8, 0)` | `spike` | `s_normal` | 1 |
| `(10, 0)` | `funcblock` | `fb_jump` | 0 |

별에는 Unity의 실제 옵션 정의를 사용해 다음 값을 명시했다.

Scale = 0.7

이를 통해 단순 블록뿐 아니라 block_options의 위치, 블록 이름, 옵션 이름,
실수 값까지 Rust에서 보존되는지 확인한다.

생성 절차

Unity 프로젝트 루트에서 실제 프로젝트를 Unity Editor로 연다.

Play Mode는 사용하지 않는다.

임시 Editor 스크립트:

Assets/Editor/ExportUnityGoldenFixture.cs

를 만들고 Unity 프로젝트에 정의된 BB_FME.Json DTO를 직접 사용하여 위 fixture
데이터를 구성한다.

직렬화는 실제 게임과 동일하게 다음 코드를 사용한다.

string json =
JsonConvert.SerializeObject(
map,
Formatting.Indented
);

생성 파일:

unity_phase4_vertical_slice.json

은 내용을 손으로 수정하지 않고 그대로 Bevy 저장소로 복사한다.

cp unity_phase4_vertical_slice.json \
 /home/flora1119/rust_projects/bb_fme_bevy/assets/maps/unity_phase4_vertical_slice.json

fixture 생성 후 임시 Unity Editor 스크립트는 삭제할 수 있다.

원본 무결성

2026-08-17 생성 fixture의 SHA-256: c50b694f4f0b45eab8630967556cf06dc91024313bd8979607de5eb3826cc3f9

fixture를 의도적으로 다시 생성하거나 변경한 경우 이 값도 함께 갱신한다.

단순 formatting 목적만으로 fixture를 수정하지 않는다.

이 파일은 사람이 관리하는 예제 JSON이 아니라 Unity serializer 출력의 기준 자료로
취급한다.

Rust 검증

호환성 테스트: tests/map_document_compat.rs

테스트: unity_editor_golden_fixture_round_trips_and_validates

검증 흐름:

Unity JSON
-> serde_json
-> MapDocument
-> map validation
-> ValidatedMap
-> Rust JSON serialization
-> MapDocument

현재 테스트는 다음을 확인한다.

Unity JSON이 MapDocument로 역직렬화된다.
25 x 15 맵 크기가 유지된다.
별 개수 1이 유지된다.
ball이 (2, 2)에 유지된다.
star가 (6, 2)에 유지된다.
b_normal이 (2, 0)에 유지된다.
s_normal의 방향 1이 유지된다.
fb_jump가 (10, 0)에 유지된다.
star의 Scale = 0.7 옵션이 유지된다.
validation 후 ValidatedMap에서도 별 옵션이 유지된다.
Rust에서 다시 직렬화하고 역직렬화한 뒤 동일한 MapDocument 의미가 유지된다.
부동소수점 round-trip 정책

Unity JSON에는 별 옵션이 다음처럼 기록된다.

{
"value_name": "Scale",
"value": 0.7
}

Unity의 float와 Rust의 f32는 모두 IEEE 754 단정밀도 부동소수점이므로
Rust에서 역직렬화한 실제 값은 대략 다음과 같이 표현될 수 있다.

0.699999988079071

따라서 JSON Value 전체를 문자적 숫자 표현까지 완전히 동일하게 비교하지 않는다.

대신 옵션 값에는 허용 오차 비교를 사용하고, 전체 round-trip은:

MapDocument
-> serialize
-> deserialize
-> MapDocument

후의 의미적 동등성을 검사한다.

이는 0.7과 그 f32 표현 차이를 데이터 손실로 잘못 판단하는 것을 방지한다.

검증 명령

개별 호환성 테스트:

cargo test --test map_document_compat

전체 정적 및 회귀 검증:

cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test --quiet

이 세 검증과 Unity fixture 테스트가 모두 통과하면
Phase 4의 Unity JSON 골든 fixture 체크포인트를 완료한 것으로 본다.
