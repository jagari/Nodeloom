# 역할 B: 에이전트 (이슈 #36)

- **브랜치**: `feat/36-agent-reconnect`
- **고칠 수 있는 파일**: `crates/nodeloom-agent/**`. 새 테스트 파일 `crates/nodeloom-agent/tests/*.rs`도 된다.
- **리뷰어**: A
- 규칙과 계약은 `docs/handoff/README.md`. **`AgentConfig`에 필드를 추가하지 않는다.** 마스터 e2e 테스트와 데스크톱 앱이 구조체 리터럴로 만들기 때문이다. `AgentStatus`의 변형과 JSON 모양도 그대로 둔다. 앱 화면(D)이 그대로 읽는다.

## 왜 중요한가

- 마스터 노트북이 잠들거나 와이파이가 끊기면, 에이전트는 응답 없는 소켓을 끝없이 기다린다.
  - `crates/nodeloom-agent/src/lib.rs`의 `session` 안 승인 후 `loop { tokio::select! { … next_master_message(&mut stream) … } }`
  - 마스터가 돌아와도 다시 붙지 않는다.
- 명령줄 에이전트는 30초 안에 마스터를 못 찾으면 종료한다(`main.rs`의 `discovery::find_master(Duration::from_secs(30))`). 라즈베리파이를 먼저 켜 두면 시연 때 꺼져 있다.

## P0-1. 응답 없는 마스터를 감지하고 다시 연결

1. 승인 후 루프에서 다음 두 가지를 한다. 간격은 `config.metrics_interval`에서 계산한다. 테스트는 `metrics_interval`을 50ms로 줘서 빠르게 돈다. 필드를 추가하지 않기 위해서다.
   - `metrics_interval × 5`마다 `Message::Ping`을 보낸다(최소 1초, 최대 5초).
   - **아무 프레임이나** 받을 때마다 마지막 수신 시각을 갱신한다. `next_master_message`는 Ping과 Pong을 건너뛰므로, 그 전에 시각을 기록하도록 고친다.
2. `metrics_interval × 15`(최소 3초, 최대 15초) 동안 아무것도 못 받으면 `Err("the master stopped answering")`로 세션을 끝낸다.
   - `run_forever`가 상태를 `Connecting { last_error: … }`로 바꾸고 다시 연결한다.
3. **테스트**(`crates/nodeloom-agent/tests/`): `tokio-tungstenite`로 가짜 마스터를 만든다.
   1. Challenge에 이어 Welcome을 보낸다.
   2. 그 뒤로는 소켓을 읽지 않는다(Pong도 가지 않는다).
   3. 에이전트 상태 채널(`AgentConfig.status`)이 `Connecting { last_error: Some(…) }`로 바뀌는지 확인한다.
   4. 다시 접속을 시도하는지(가짜 마스터에 두 번째 연결이 오는지) 확인한다.
4. 마스터의 e2e 테스트(`cargo test --workspace`)가 모두 통과해야 한다. 정상 마스터에서 끊기면 안 된다. 마스터는 Ping에 자동으로 Pong한다.

## P0-2. 명령줄 에이전트는 마스터를 찾을 때까지 계속 찾는다

- `main.rs`에서 `find_master`가 실패하면 종료하지 말고, 로그를 남긴 뒤 다시 찾는다. 앱의 `find_master_forever`와 같은 동작이다.
- `--metrics-interval`은 1~5초로 제한한다. A의 10초 무응답 기준보다 짧아야 한다.
- 테스트: 반복 로직을 작은 함수로 빼서 단위 테스트한다. mDNS는 테스트하지 않는다.

## P0-3. 라즈베리파이 실기기 확인 (#24)

1. 릴리즈의 `linux-arm64` 명령줄 패키지를 설치한다.
2. 마스터(노트북 앱)에 자동 탐색으로, 그리고 `--master <주소>`로 각각 붙는다. 테스트 작업을 받는지 본다.
3. 결과(OS 버전, 걸린 시간, 문제)를 #24에 적고, 시연 대본용으로 E에게 알린다.

## P1. 마스터 주소가 바뀌면 다시 찾기 (명령줄)

- 탐색으로 찾은 마스터에 연속 N번(예: 5번) 접속에 실패하면, 탐색부터 다시 한다. 공유기가 마스터에 새 IP를 준 경우다.
- `run_forever`는 바꾸지 말고, 새 `pub fn`을 **추가**해 `main.rs`에서만 쓴다. 앱(D)은 시연 후에 같은 함수를 쓸 수 있다.

## 하지 말 것

- `AgentConfig` 필드나 `AgentStatus` 변형 변경, `run_forever` 시그니처 변경
- 마스터 쪽 파일 수정. 마스터가 Ping을 보내는 것은 A의 일이다. 이 작업은 A의 변경 없이도 동작해야 한다.
- 문서 직접 수정. PR 본문의 "문서 반영 요청"에 적는다.

## 진행 메모

- **P0-1 완료 (`crates/nodeloom-agent/src/lib.rs`, `crates/nodeloom-agent/tests/reconnect.rs`)**:
  - `keepalive(metrics_interval)`로 Ping 주기(`metrics_interval × 5`, 1~5초)와 무응답 한계(`metrics_interval × 15`, 3~15초)를 계산한다(`AgentConfig` 필드 추가 없음).
  - 승인(`Welcome`) 이후 세션 루프에서 주기적으로 `Message::Ping`을 전송하고, `next_master_message`는 Ping/Pong을 건너뛰기 전에 모든 수신 프레임마다 `last_heard`를 갱신한다.
  - 무응답 한계 동안 아무 프레임도 받지 못하면 `Err("the master stopped answering")`로 세션을 종료하며, `run_forever`가 `AgentStatus::Connecting { last_error: Some(...) }`를 보고한 뒤 재연결한다.
  - `crates/nodeloom-agent/tests/reconnect.rs`에 `tokio-tungstenite` 가짜 마스터 테스트(`reconnects_when_the_master_stops_answering`)를 추가했다.
- **P0-2 완료 (`crates/nodeloom-agent/src/lib.rs`, `crates/nodeloom-agent/src/main.rs`)**:
  - `find_master_forever`와 내부 반복 헬퍼 `retry_until_ok`를 추가해, 마스터를 찾을 때까지 2초 간격으로 재시도한다(`retry_until_ok_keeps_trying_until_it_succeeds` 단위 테스트 포함).
  - `--metrics-interval` 인자를 `clap::value_parser!(u64).range(1..=5)`로 1~5초로 제한하고 단위 테스트(`metrics_interval_is_one_to_five_seconds`)를 추가했다.
- **P1 완료 (`crates/nodeloom-agent/src/lib.rs`, `crates/nodeloom-agent/src/main.rs`)**:
  - `run_forever` 시그니처는 그대로 유지하고, 새 공개 함수 `run_forever_discovering` 및 상수 `REDISCOVER_AFTER_FAILURES (= 5)`를 추가했다.
  - 탐색으로 찾은 마스터에 연속 5회 접속(`Welcome` 이전)이 실패하면 `find` 클로저를 다시 호출해 새 주소를 탐색한다(`looks_for_the_master_again_after_repeated_failures` 단위 테스트 포함).
- **문서 반영 요청 (E에게)**:
  - 에이전트가 승인 후에도 `metrics_interval × 5`(1~5초)마다 WebSocket Ping을 보내고 `metrics_interval × 15`(3~15초) 동안 프레임이 없으면 재연결함.
  - 명령줄 에이전트(`nodeloom-agent`)는 `--master` 생략 시 마스터를 찾을 때까지 계속 탐색하며, 연속 5회 접속 실패 시 mDNS 탐색을 다시 수행함. `--metrics-interval`은 1~5초로 제한됨.
