# 아키텍처

코드를 바꾸면서 이 문서와 다른 동작을 만들었다면 같은 PR에서 이 문서도 고친다.

## 구성 요소

```
 ┌──────────────┐  HTTP / WebSocket (/api, /ws/ui)  ┌──────────────────┐
 │ 데스크톱 UI    │ ────────────────────────────────▶ │ nodeloom-master  │
 │ (Tauri+Svelte)│ ◀──── 클러스터 스냅샷(JSON) ───── │ 상태·스케줄링·API │
 └──────────────┘                                   └────────▲─────────┘
                                                             │ WebSocket (/ws/agent)
                                      ┌──────────────────────┼──────────────────────┐
                                      │                      │                      │
                               ┌──────┴──────┐        ┌──────┴──────┐        ┌──────┴──────┐
                               │ agent (맥)   │        │ agent (맥)   │        │ agent (PC)  │
                               │ Runner들     │        │ Runner들     │        │ Runner들     │
                               └─────────────┘        └─────────────┘        └─────────────┘
```

- **master** (`crates/nodeloom-master`): 한 대에서 실행한다. 에이전트 연결을 받고, 작업을 나누어 보내고, UI에 상태를 제공한다. 기본 포트는 `7878`이다.
- **agent** (`crates/nodeloom-agent`): 작업할 모든 기기에서 실행한다. 자원 사용량을 보고하고 배정받은 작업을 `Runner`로 실행한다.
- **UI** (`apps/desktop`): 마스터 API의 클라이언트일 뿐이다. 같은 API를 쓰는 다른 클라이언트(웹, 모바일)를 나중에 붙일 수 있다.
- **proto** (`crates/nodeloom-proto`): 위 셋이 주고받는 모든 메시지 타입. UI 쪽 사본은 `apps/desktop/src/lib/types.ts`(손으로 맞춤).

## 연결과 탐색

- 마스터는 mDNS로 `_nodeloom._tcp.local.`을 알린다(`--no-mdns`로 끔).
- 에이전트는 `--master host:port`가 없으면 mDNS로 마스터를 찾는다(최대 30초).
- Thunderbolt Bridge도 운영체제가 IP 네트워크로 잡으므로 LAN과 같은 경로를 쓴다.
- 에이전트의 기기 ID는 사용자 설정 폴더의 `nodeloom/node-id`에 저장해 재시작해도 유지한다.
- 연결이 끊기면 에이전트는 1초부터 최대 30초까지 간격을 늘리며 다시 접속한다.

## 메시지 흐름 (WebSocket, JSON, `type` 필드로 구분)

```
agent → master   Hello { protocol, token, info }        첫 메시지, 10초 안에 보내야 함
master → agent   Welcome | Rejected { reason }          프로토콜 버전·토큰 확인 결과
agent → master   Metrics(...)                           주기적 자원 보고 (기본 1초)
master → agent   AssignTask(TaskSpec)                   작업 배정
agent → master   TaskProgress { task_id, frames_done, frames_total }
agent → master   TaskDone { task_id } | TaskFailed { task_id, error }
master → agent   CancelTask { task_id }
```

- `PROTOCOL_VERSION`이 다르면 연결을 거부한다. 호환되지 않는 메시지 변경 시 올린다.
- WebSocket 메시지 최대 1MB, HTTP 요청 본문 최대 64KB.

## 작업과 스케줄링 (`nodeloom-master/src/state.rs`)

- **작업(job)** 은 프레임 범위 하나다. 제출 시점에 그 작업을 실행할 수 있는 기기 수만큼 연속된 구간의 **태스크(task)** 로 나눈다(`nodeloom_core::split::split_frames`). 실행할 기기가 없으면 409를 돌려준다.
- 대기 중인 태스크는 실행 중 태스크가 가장 적은 기기에 배정한다.
- 기기가 끊기면 그 기기의 실행 중 태스크를 대기로 되돌리고(진행률 0) 다른 기기에 다시 배정한다. 같은 ID로 다시 접속해도 이전 세션의 태스크는 다시 배정한다.
- 에이전트는 **자기에게 배정된 태스크만** 진행·완료·실패로 바꿀 수 있다.
- 메모리에는 최근 작업 100개만 둔다. 끝난 작업부터 지운다. 마스터를 재시작하면 작업 기록은 사라진다(영속 저장 없음).

## UI용 API (`nodeloom-master/src/server.rs`)

| 경로 | 설명 |
|---|---|
| `GET /api/state` | 전체 스냅샷 `ClusterSnapshot` |
| `POST /api/jobs` | `SubmitJob { payload }` 제출. 400 잘못된 작업, 409 실행 가능한 기기 없음 |
| `GET /ws/ui` | 스냅샷 스트림. 연결 직후 한 번, 이후 변경 시마다(최대 초당 약 10회) |
| `GET /ws/agent` | 에이전트 전용 |

- 인증: `Authorization: Bearer <토큰>` 또는 `?token=<토큰>`(브라우저 WebSocket은 헤더를 못 넣기 때문).
- CORS는 모든 출처를 허용한다. 쿠키가 아니라 토큰 인증이라 CSRF 위험이 없기 때문이다.

## 러너 (`Runner` 트레이트, `nodeloom-core/src/runner.rs`)

- 작업 종류마다 러너가 하나다. `TaskPayload`의 각 변형이 정확히 하나의 러너에 대응한다.
- `FakeRunner`: 프레임마다 잠깐 쉬기만 한다. 테스트용이며 에이전트에서는 `--enable-fake-runner`로만 켠다.
- `BlenderRunner` (`nodeloom-runner-blender`): Blender를 찾아(맥 앱 경로, 윈도우 기본 설치 경로, PATH) 다음처럼 실행한다.
  `blender --background --disable-autoexec <작업폴더>/<파일> --render-output <작업폴더>/renders/frame_#### --frame-start N --frame-end M --render-anim`
  표준 출력의 `Saved:` 줄 수로 진행률을 계산한다. 결과물은 각 기기의 `renders/`에 남는다(수집 기능 미구현).
- 새 작업 종류를 추가하는 순서: `TaskPayload` 변형 추가(proto, 이슈에서 합의) → `validate_payload` 규칙 → 러너 구현과 테스트 → 에이전트에 등록 → UI 타입·폼.

## 보안 모델

- 같은 네트워크의 아무나 작업을 보내지 못하게 모든 연결에 페어링 토큰을 요구한다. 토큰은 마스터가 시작할 때 무작위로 만들거나 `NODELOOM_TOKEN`으로 지정한다.
- 작업은 닫힌 enum이라 에이전트가 모르는 종류(예: 셸 명령)는 역직렬화 단계에서 거부된다.
- 파일 경로는 상대 경로만 허용하고 `..`, 절대 경로, 드라이브 문자, 역슬래시를 거부한다. 마스터와 에이전트 양쪽에서 검사한다.
- **현재 통신은 암호화되지 않는다.** 토큰이 평문으로 오가므로 신뢰하는 네트워크에서만 쓴다. TLS는 남은 작업이다.

## 빌드 구성

- Rust 워크스페이스는 `crates/*`만 포함한다. Tauri 셸(`apps/desktop/src-tauri`)은 GUI 시스템 라이브러리가 필요해서 워크스페이스에서 빼고 따로 빌드한다.
- UI는 Vite + Svelte 5 + TypeScript 6이다. svelte-check가 TypeScript 7을 단독으로 지원하지 않아 6에 고정했다.
- 지원 플랫폼과 빌드(D14):

| 플랫폼 | 타깃 | CI 테스트 | 배포 바이너리 |
|---|---|---|---|
| 리눅스 x64 | `x86_64-unknown-linux-musl` | `ubuntu-latest` | 정적 링크 |
| 리눅스 arm64 (라즈베리파이 4·5, Jetson) | `aarch64-unknown-linux-musl` | `ubuntu-24.04-arm` | 정적 링크 |
| 윈도우 x64 | `x86_64-pc-windows-msvc` | `windows-latest` | |
| 윈도우 arm64 | `aarch64-pc-windows-msvc` | `windows-11-arm` | |
| macOS arm64 | `aarch64-apple-darwin` | `macos-latest` | |

  - 테스트는 각 아키텍처의 러너에서 그대로 돌린다(교차 컴파일 아님). CI 테스트는 glibc 빌드로, 배포 바이너리는 위 타깃으로 만든다.
  - 리눅스 배포 바이너리를 musl 정적 링크로 만드는 이유: 라즈베리파이 OS(Bookworm, glibc 2.36)와 JetPack(Ubuntu 20.04·22.04, glibc 2.31·2.35)은 빌드 러너(Ubuntu 24.04, glibc 2.39)보다 glibc가 오래돼, glibc에 링크한 바이너리는 실행되지 않는다. 의존성에 C 코드가 없어 musl용 별도 도구 없이 빌드된다.
  - 의존성에 C 코드나 OpenSSL을 넣으면 이 전제가 깨진다. 넣기 전에 musl·윈도우 ARM 빌드를 확인한다.
