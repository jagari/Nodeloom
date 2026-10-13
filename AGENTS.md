# AGENTS.md: Nodeloom 작업 규칙 (사람과 LLM 공통)

이 파일은 팀과 모든 LLM 도구가 함께 쓰는 **규칙의 원본**이다. `CLAUDE.md`와 `GEMINI.md`는 이 파일을 가져오기만 한다.
규칙을 바꾸려면 이 파일을 PR로 고친다. 누구를 위해 무엇을 만드는지는 `docs/STRATEGY.md`, 배경과 이유는 `docs/DECISIONS.md`, 현재 진행도는 `docs/STATUS.md`를 본다.

## 프로젝트 한 줄 요약

학생·소규모 팀이 가진 맥·윈도우·리눅스·라즈베리파이·Jetson을 **설치만 하면 자동으로 묶어(플러그 앤 플레이)**, 한 화면에서 보고 **파일 묶음 작업을 기기들에 나눠 처리하는** 도구다. 첫 분산 작업은 Whisper 배치 전사이고, AI 엔진 감지 같은 기능은 플러그인으로 붙인다. 목표는 실사용자와 포트폴리오이며 수익화는 보류했다(`docs/DECISIONS.md` D11, D12). 기능 아이디어가 `docs/STRATEGY.md`와 맞지 않으면 만들지 않는다.

**범위 밖** (제안하거나 구현하지 않는다):
- 네트워크로 받은 명령 문자열 실행(절대 규칙 1번). 기기 주인 템플릿(로드맵 P2)도 팀 결정 전에는 만들지 않는다.
- "모든 작업이 빨라진다"는 주장. 독립 단위로 나눌 수 있는 작업만 빨라진다.
- 실시간 뷰포트·게임 fps 가속. 기기 간 연결(Thunderbolt 약 5GB/s)이 GPU 메모리 대역폭(수십~수백 GB/s)보다 훨씬 느리다.
- 추론 엔진·렌더러 자체 개발. 기존 프로그램을 감싸서 쓴다.

## 구조 지도

| 경로 | 역할 | 담당 역할 |
|---|---|---|
| `crates/nodeloom-proto` | 모든 구성 요소가 쓰는 메시지 타입(팀 공통 계약) | A |
| `crates/nodeloom-core` | 작업 분할, `Runner` 트레이트(플러그인의 바탕), 자원 수집, 토큰 비교, 작업 검증, mDNS | A |
| `crates/nodeloom-master` | 클러스터 상태, 스케줄링·재배정, HTTP/WebSocket API | A |
| `crates/nodeloom-agent` | 각 기기에서 도는 데몬: 접속, 자원 보고, 작업 실행 | B |
| `crates/nodeloom-runner-blender` | `blender -b`로 프레임 범위 렌더하는 내장 플러그인. **보류**(신규 기능 없음). 새 플러그인을 만들 때 구조 예시로 본다 | B |
| `apps/desktop/src` | 데스크톱 UI (Svelte 5 + TypeScript) | C |
| `apps/desktop/src-tauri` | Tauri 데스크톱 셸(워크스페이스와 별도 빌드) | D |
| `.github/workflows` | CI | D |
| `docs/` | 아키텍처·결정·진행도 문서 | E (모두 갱신) |

역할: A 총괄·코어 리뷰(프로토콜, 페어링, 스케줄링), B 에이전트·플러그인(GPU 지표, Whisper 등), C 데스크톱 UI, D 배포·지원 기기(설치 파일, arm64 빌드, 실기기 검증), E 문서·사용자 검증·QA. 자세한 구조는 `docs/ARCHITECTURE.md`.

## 절대 규칙

1. **에이전트는 네트워크로 받은 명령 문자열을 실행하지 않는다.** 작업은 다음 두 가지로만 표현한다.
   - ① 닫힌 enum `TaskPayload`(`nodeloom-proto`)의 **내장 플러그인**
   - ② 기기 주인이 **그 기기의 로컬 설정에만** 등록한 **템플릿의 이름과 검증된 인자**

   템플릿은 네트워크로 추가·변경할 수 없다. ②는 설계·보안 검토 후 팀이 결정하기 전까지 구현하지 않는다(`docs/STRATEGY.md` P2, D12). "명령 문자열을 받아 실행"하는 기능은 어떤 형태로도 추가하지 않는다.
2. **작업 검증은 양쪽에서 한다.** 마스터(제출 시)와 에이전트(실행 직전) 모두 `nodeloom_core::validate::validate_payload`를 호출한다. 경로는 작업 폴더 안의 상대 경로만 허용한다.
3. **외부 프로그램은 셸을 거치지 않고 인자 목록으로 실행한다.** Blender는 `--disable-autoexec`를 유지한다.
4. **인증을 약화하지 않는다.** 모든 에이전트 연결과 UI API는 인증이 필요하다. 지금은 공유 페어링 토큰이고 비교는 `nodeloom_core::auth::tokens_match`를 쓴다. 플러그 앤 플레이를 위해 "처음 한 번 화면에서 승인 → 기기별 자격 증명"으로 바꿀 예정이다(P0). 승인 전 기기에게는 작업이나 클러스터 정보를 주지 않는다. "자동으로 아무 기기나 붙는다"는 구현하지 않는다.
5. **비밀값과 개인정보를 커밋하지 않는다.** 토큰, API 키, 팀원 개인정보, D-Lab 신청서 같은 문서가 해당한다.
6. **테스트를 끄거나 건너뛰지 않는다.** 실패하면 원인을 고친다. 새 기능에는 테스트를 같이 쓴다.
7. **프로토콜을 바꾸면 양쪽을 같이 고친다.** `nodeloom-proto`를 바꿨다면 `apps/desktop/src/lib/types.ts`(손으로 맞추는 사본)도 고치고, Rust와 UI 테스트를 모두 돌린다. 프로토콜 변경은 이슈에서 먼저 합의하고 A가 승인한다.

## 명령어

```sh
# Rust (저장소 루트)
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace

# UI (apps/desktop)
pnpm install
pnpm check && pnpm test && pnpm build
pnpm dev            # http://localhost:5173

# 데스크톱 셸 (Tauri 시스템 라이브러리 필요)
cargo build --manifest-path apps/desktop/src-tauri/Cargo.toml
```

CI는 **항상 최신 stable Rust**를 쓴다. 로컬 Rust가 오래되면 로컬에서는 통과한 clippy가 CI에서 실패할 수 있으니, 작업 전에 `rustup update stable`을 실행한다.

한 기기에서 전체 흐름 실행:

```sh
cargo build
export NODELOOM_TOKEN=dev-token
./target/debug/nodeloom-master --bind 127.0.0.1:7878 --no-mdns &
./target/debug/nodeloom-agent --master 127.0.0.1:7878 --enable-fake-runner &
# UI에서 127.0.0.1:7878 / dev-token 으로 접속
```

## 작업 흐름

1. GitHub Issue에서 작업을 고르고 담당자를 지정한다. 이슈에 없는 큰 작업은 이슈부터 만든다.
2. `main`에서 `feat/<이슈번호>-<짧은-설명>` 브랜치를 만든다. 버그 수정은 `fix/...`.
3. 작게 커밋한다. 커밋 메시지는 `feat:`, `fix:`, `docs:`, `test:`, `chore:` 접두사를 쓴다.
4. 위 명령어가 모두 통과한 뒤 PR을 연다. PR 템플릿을 채운다.
5. CI 통과와 리뷰 1명 이상이 필요하다. `crates/` 변경은 A가 리뷰한다.
6. 동작이나 구조를 바꿨으면 같은 PR에서 `docs/STATUS.md`, `docs/ARCHITECTURE.md`를 갱신한다. 중요한 결정은 `docs/DECISIONS.md`에 추가한다.

## LLM 사용 규칙

- LLM이 쓴 코드도 **PR을 연 사람이 책임진다.** 이해하지 못한 코드는 올리지 않는다. 모르는 부분은 PR에 적고 리뷰어에게 묻는다.
- 커밋 전에 위 명령어를 직접 돌린다. "테스트가 통과할 것"이라는 LLM의 말을 믿지 않는다.
- LLM에게 작업을 맡길 때는 이슈 본문(목표, 관련 파일, 완료 기준)을 그대로 준다.
- 파일을 읽지 못하는 채팅형 도구(ChatGPT 등)에는 `python3 scripts/llm-context.py <영역> [파일...]` 출력을 붙여 넣는다.
- LLM이 위 절대 규칙과 충돌하는 제안을 하면 따르지 않는다.
- PR 템플릿에 LLM을 썼는지와 어떻게 검증했는지를 적는다.

## 코드 스타일

- Rust: `rustfmt` 기본값, clippy 경고 0. `unsafe` 금지(워크스페이스 설정). 오류는 `thiserror`(라이브러리)와 `anyhow`(바이너리).
- TypeScript/Svelte: Svelte 5 룬(`$state`, `$derived`, `$props`). 순수 로직은 `src/lib/*.ts`에 두고 `*.test.ts`로 테스트한다.
- UI의 색·차트 규칙: 색은 `src/app.css`의 토큰만 쓴다. 상태는 색만으로 표시하지 않고 아이콘과 글자를 함께 쓴다. 라이트·다크 모드를 모두 확인한다.
- 주석은 "왜"를 쓴다. 기존 코드의 주석 밀도와 말투를 따른다.
