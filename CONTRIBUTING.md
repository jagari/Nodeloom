# 기여 가이드

규칙의 원본은 [AGENTS.md](AGENTS.md)다. 이 문서는 처음 참여하는 팀원을 위한 순서 안내다.

## 처음 한 번

1. 도구 설치
   - Rust: https://rustup.rs (stable)
   - Node.js 22와 pnpm 10
   - 데스크톱 셸을 빌드할 사람만: Tauri 사전 요구사항 (https://tauri.app/start/prerequisites/)
2. 저장소를 받고 테스트가 통과하는지 확인한다.
   ```sh
   git clone https://github.com/jagari/Nodeloom.git
   cd Nodeloom
   cargo test --workspace
   cd apps/desktop && pnpm install && pnpm test
   ```
3. `AGENTS.md`, `docs/STATUS.md`, 맡은 영역의 `docs/ARCHITECTURE.md` 부분을 읽는다.

## 작업 하나를 끝내는 순서

1. GitHub Issues에서 이슈를 고르고 자신을 담당자로 지정한다.
2. 브랜치를 만든다: `git switch -c feat/12-render-collect main`
3. 코드와 테스트를 같이 쓴다. LLM을 쓰면 이슈 본문을 그대로 넘긴다.
4. 확인한다.
   - Rust를 고쳤다면: `cargo fmt --all`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`
   - UI를 고쳤다면: `pnpm check && pnpm test && pnpm build`
5. 푸시하고 PR을 연다. 템플릿을 채우고 `Closes #이슈번호`를 적는다.
6. CI가 초록색이고 리뷰 승인을 받으면 **Squash and merge**로 합친다.

## 리뷰 규칙

- 모든 PR은 다른 팀원 1명 이상이 리뷰한다.
- `crates/` 변경과 `nodeloom-proto` 변경은 역할 A가 리뷰한다.
- 리뷰어는 "이 코드를 작성자가 설명할 수 있는가"를 확인한다. LLM이 쓴 코드라도 설명하지 못하면 고쳐 달라고 요청한다.

## Rust가 처음이라면

- 처음에는 `good first issue` 라벨이 붙은 이슈를 고른다.
- 컴파일 오류 메시지를 그대로 LLM에 붙여 넣고, 왜 그런지 설명도 같이 요청한다.
- `cargo clippy`의 제안은 대부분 그대로 따라도 된다.
- 테스트 예시는 같은 파일 아래쪽의 `#[cfg(test)] mod tests`와 `crates/nodeloom-master/tests/e2e.rs`에 있다.

## LLM에 맥락을 주는 방법

| 도구 | 방법 |
|---|---|
| Claude Code | 저장소 폴더에서 실행하면 `CLAUDE.md` → `AGENTS.md`를 자동으로 읽는다 |
| Gemini CLI | `GEMINI.md` → `AGENTS.md`를 자동으로 읽는다 |
| Codex 등 AGENTS.md 지원 도구 | `AGENTS.md`를 바로 읽는다 |
| 채팅형 ChatGPT·Claude·Gemini | `python3 scripts/llm-context.py <영역> [파일...]` 출력을 붙여 넣는다 |

`llm-context.py`의 영역 이름: `core`, `master`, `agent`, `ui`, `platform`, `biz`. 예:

```sh
python3 scripts/llm-context.py ui apps/desktop/src/components/JobList.svelte > context.md
```

## 하지 말 것

- `main`에 직접 푸시하기
- 테스트를 끄거나 `#[ignore]`로 숨기기
- 토큰, 개인정보, 신청서 같은 문서 커밋
- 이슈 합의 없이 `nodeloom-proto` 바꾸기
