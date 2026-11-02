# 시연 스프린트 핸드오프 (2026-11-02 → 시연 11-03)

5명이 각자 Claude와 함께 **동시에** 작업하고, 모든 PR을 어떤 순서로 병합해도 충돌하지 않게 나눈 계획이다. 각자 자기 역할 문서만 보고 시작해도 된다.

## 출발점

- 시작 기준은 [jagari/Nodeloom#40](https://github.com/jagari/Nodeloom/pull/40)(beta.3: 윈도우 수정, 기기 추가 안내, 주소 직접 입력)이 병합된 `main`이다. 그 전에 시작하지 않는다.
- 현재 동작은 다음 문서에서 확인한다.
  - 사용법: `docs/BETA.md`
  - 구조: `docs/ARCHITECTURE.md`
  - 진행도: `docs/STATUS.md`

## 시연에서 보여줄 것

1. 노트북 A에서 앱을 켜고 **Start a cluster here**를 고른다. 창이 대시보드가 된다.
2. 노트북 B(윈도우)와 맥에서 **Join a cluster**를 고른다. 코드를 대조해 승인한다.
3. 라즈베리파이를 명령줄로 붙인다.
4. 테스트 작업을 보낸다. 기기별로 나뉘어 처리되는 모습을 보여 준다.
5. **기기 하나의 와이파이를 끈다. 그 몫이 10초 안에 다른 기기로 넘어가 작업이 끝난다.** 지금은 이 단계가 안 된다(역할 A, B).

## 역할과 소유 파일

**자기 역할의 파일만 고친다.** 표에 없는 파일은 아무도 고치지 않는다. 다른 영역에 고칠 것이 있으면 그 역할의 이슈에 댓글로 요청한다.

| 역할 | 이슈 | 브랜치 | 고칠 수 있는 파일 |
|---|---|---|---|
| A 마스터·코어 | #35 | `feat/35-dead-agent` | `crates/nodeloom-master/**`, `crates/nodeloom-core/**` |
| B 에이전트 | #36 | `feat/36-agent-reconnect` | `crates/nodeloom-agent/**` |
| C 대시보드 | #37 | `feat/37-demo-dashboard` | `apps/desktop/src/**`, `apps/desktop/index.html`, `apps/desktop/public/**`. **단, 아래 D 소유 파일과 `src/lib/types.ts`는 제외** |
| D 데스크톱 앱·배포 | #38 | `feat/38-app-release` | `apps/desktop/src-tauri/**`, `apps/desktop/src/components/AppPage.svelte`, `apps/desktop/src/lib/app.ts`, `apps/desktop/src/lib/app.test.ts`, `.github/workflows/**`, `scripts/install.*`, `scripts/package-readme.txt`. 릴리즈할 때만 루트 `Cargo.toml`의 `version` 한 줄 |
| E 문서·QA·시연 | #39 | `docs/39-demo` | `docs/**`(다른 역할의 `docs/handoff/*.md`는 제외), `README.md`, `CHANGELOG.md`, `AGENTS.md` |

각 역할은 `docs/handoff/` 안의 **자기 파일**(`A-master.md` 등)도 고칠 수 있다. 진행 메모나 다음 사람에게 남길 말을 적는다.

## 아무도 바꾸지 않는 것 (계약)

이것을 바꾸면 다른 사람의 PR이 컴파일되지 않거나 동작이 어긋난다. 시연 전에는 고정한다.

1. **프로토콜**: `crates/nodeloom-proto/**` 전체와 `apps/desktop/src/lib/types.ts`. 메시지, `PROTOCOL_VERSION`, `TaskPayload`를 바꾸지 않는다.
2. **HTTP·WebSocket API**: 경로와 JSON 모양을 바꾸지 않는다. 대상은 `/api/state`, `/api/info`, `/api/jobs`, `/api/pending/*`, `/api/devices/*`, `/ws/ui`, `/ws/agent`다. 새 경로 추가는 A만 할 수 있다.
3. **공개 Rust API**: 기존 `pub` 항목의 이름, 인자, 구조체 필드를 바꾸거나 지우지 않는다. 새 `pub` 항목을 **추가**하는 것은 된다.
   - `nodeloom_agent`: `AgentConfig`(필드 추가도 금지. 다른 사람 코드가 구조체 리터럴로 만든다), `AgentStatus`(변형과 JSON 모양), `run_forever`, `run_once`, `identity::*`
   - `nodeloom_master`: `Cluster::new`, `Cluster::with_store`, `Cluster::with_settings`, `Settings`(기존 필드), `DeviceStore::open`, `load_admin_token`, `serve`, `dashboard_url`, `lan_ip`, `server::MasterInfo`
   - `nodeloom_core`: `discovery::{advertise, find_master}`, `metrics::*`, `FakeRunner`, `RunnerSet`, `validate::*`
4. **의존성**: 새 의존성을 추가하지 않는다. `Cargo.lock`, `apps/desktop/src-tauri/Cargo.lock`, `apps/desktop/pnpm-lock.yaml`은 그대로 둔다.
5. **버전**: 시연 릴리즈 때 D만 올린다(아래 "릴리즈 순서").
6. **앱 설정 파일** `settings.json`의 키(`role`, `master`)

## 절대 규칙은 그대로

`AGENTS.md`의 절대 규칙 1~7은 시연 스프린트에도 그대로 적용한다. 특히 다음 세 가지다.

- 인증을 약화하지 않는다(4번).
- 테스트를 끄지 않는다(6번).
- 프로토콜을 바꾸지 않는다(7번, 위 계약 1).

**스프린트 동안만 바꾸는 규칙**이 하나 있다. `docs/STATUS.md`, `docs/ARCHITECTURE.md`, `CHANGELOG.md`는 **E만** 고친다. 다른 역할은 PR 본문에 "문서 반영 요청" 절을 쓰고, E가 모아서 반영한다. 문서 충돌을 막기 위해서다.

## PR 절차

1. 최신 `main`에서 위 표의 브랜치를 만든다.
2. 작게 커밋한다. 접두사는 `feat:`, `fix:`, `test:`, `docs:`, `chore:`를 쓴다.
3. PR을 올리기 전에 아래 명령을 **모두** 통과시킨다. 자기 영역만 고쳤어도 전부 돌린다. 예를 들어 에이전트를 고치면 마스터의 e2e 테스트가 깨질 수 있다.

   ```sh
   cargo fmt --all --check
   cargo clippy --workspace --all-targets -- -D warnings
   cargo test --workspace
   cd apps/desktop && pnpm install && pnpm check && pnpm test && pnpm build && cd -
   # src-tauri를 고쳤거나 D라면 추가로:
   cargo clippy --manifest-path apps/desktop/src-tauri/Cargo.toml --all-targets -- -D warnings
   cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml
   ```

4. PR 제목은 `[시연 X] …`로 쓰고, 본문에 `Closes #N`을 넣는다. PR 템플릿을 채운다(LLM 사용, 확인 방법).
5. 병합 직전에 `git merge origin/main`(rebase하지 않는다)을 하고, CI가 초록인지 확인한 뒤 병합한다.
6. 리뷰는 한 명 이상 받는다. 짝은 A↔B, C↔D이고, E는 아무 PR이나 본다. `crates/` 변경은 A가 본다(A의 PR은 B가 본다).
7. 다른 사람 테스트가 깨지면, 그 테스트를 끄지 않고 자기 코드를 고친다. 테스트 자체를 바꿔야 하면 그 파일의 주인에게 이슈 댓글로 요청한다.

## 릴리즈 순서 (시연 빌드 `v0.1.0-beta.4`)

1. A, B, C, D의 P0 PR을 병합한다.
2. E가 `CHANGELOG.md`에 `## [0.1.0-beta.4]` 절을 쓰고 병합한다. 릴리즈 워크플로는 이 절이 없으면 실패한다.
3. D가 버전 세 곳을 `0.1.0-beta.4`로 올리는 PR을 병합한다.
   - 루트 `Cargo.toml`
   - `apps/desktop/src-tauri/Cargo.toml`
   - `apps/desktop/src-tauri/tauri.conf.json`

   이어서 `cargo update -w`와 src-tauri의 `cargo update -p nodeloom-desktop`으로 버전만 반영된 lock 파일을 함께 올린다.
4. D가 Actions → **binaries** → **Run workflow**(`main`, **release** 체크)를 실행한다. `release`와 `verify-release`가 초록인지 확인한다.
5. E가 그 빌드로 리허설한다. 이후에는 **시연을 막는 버그만** 고친다. 소유자가 고치고, 위 순서로 `beta.5`를 낸다.

## 각자의 Claude에게 처음 줄 말

```text
너는 Nodeloom 시연 스프린트의 역할 X를 맡았다.
먼저 docs/handoff/README.md와 docs/handoff/<내 역할 파일>.md를 끝까지 읽어라.
- 내 역할의 "고칠 수 있는 파일" 밖은 절대 수정하지 마라. 계약(공개 API, 프로토콜, 의존성)도 바꾸지 마라.
- 최신 main에서 <브랜치>를 만들고, P0부터 순서대로 하라. 작업마다 테스트를 같이 써라.
- README의 PR 절차 명령을 모두 통과시킨 뒤 "[시연 X]" 제목으로 PR을 올려라(Closes #<이슈>).
- 문서(STATUS, ARCHITECTURE, CHANGELOG)는 고치지 말고 PR 본문의 "문서 반영 요청"에 적어라.
- 막히면 추측해서 범위를 넓히지 말고 나에게 물어라.
```

## 지금 알고 있는 위험

| 위험 | 대응 |
|---|---|
| 갑자기 꺼진 기기의 작업이 수 분 동안 넘어가지 않는다 | A P0. 시연 5단계의 핵심이다 |
| 마스터가 꺼지면 에이전트가 영원히 기다린다 | B P0 |
| 윈도우: 공용 네트워크에서는 자동 탐색과 접속이 막힌다 | 시연 네트워크를 "개인"으로 설정한다(E 대본). 주소 직접 입력은 이미 있다 |
| 윈도우 앱 실기기 확인이 부족하다 | D P0. beta.3에서 고친 두 문제를 직접 확인한다 |
| 학교·행사장 와이파이는 기기끼리 통신을 막는 경우가 많다 | 휴대폰 핫스팟이나 가져간 공유기를 쓴다(E 대본) |
