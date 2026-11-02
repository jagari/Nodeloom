# 역할 D: 데스크톱 앱·배포 (이슈 #38)

- **브랜치**: `feat/38-app-release`. 버전을 올리는 PR은 따로 `chore/38-beta4`로 만든다.
- **고칠 수 있는 파일**
  - `apps/desktop/src-tauri/**`. 그 안의 `Cargo.lock`도 포함하지만 새 의존성은 넣지 않는다.
  - `apps/desktop/src/components/AppPage.svelte`, `apps/desktop/src/lib/app.ts`, `apps/desktop/src/lib/app.test.ts`
  - `.github/workflows/**`, `scripts/install.*`, `scripts/package-readme.txt`
  - 릴리즈할 때만 루트 `Cargo.toml`의 `version` 한 줄
- **리뷰어**: C
- 규칙과 계약은 `docs/handoff/README.md`. `settings.json` 키와 앱 명령의 이름·인자는 지금 그대로 둔다.

## 지금 상태 (beta.3)

- 맥 앱은 사용자가 확인했고 잘 동작한다.
- 윈도우 앱은 beta.2에서 두 가지가 확인됐고, beta.3에서 고쳤다. 다만 **실기기에서 고쳐졌는지는 아직 아무도 보지 않았다.**
  1. 역할을 고르면 앱이 사라졌다. 재시작할 때 중복 실행 방지 잠금이 남아 있었다. `src-tauri/src/lib.rs`의 `restart()` 주석 참고.
  2. 트레이 Settings를 누르면 앱이 멈췄다. 이벤트 처리기 안에서 창을 만들면 WebView2가 멈춘다. 지금은 창을 시작할 때 한 번만 만든다.
- 로그 파일 위치: `%LOCALAPPDATA%\nodeloom\logs\desktop.log`(윈도우), `~/Library/Application Support/nodeloom/logs/desktop.log`(맥). 앱 설정 화면에도 나온다.

## P0-1. 윈도우·맥 실기기 확인과 수정

beta.3 설치 파일로 각 OS에서 확인한다. 결과는 아래 표 형식으로 PR과 #3에 남긴다.

| 확인 | 윈도우 x64 | 맥 |
|---|---|---|
| 설치(경고 넘기기 포함) | | |
| 첫 실행 → Start a cluster here → **재시작 후 창이 다시 뜸**(사라지지 않음) | | |
| 대시보드가 Live, 이 기기 1대 | | |
| 트레이 → Settings가 열림(**멈추지 않음**) | | |
| 다른 기기가 Join a cluster로 자동 탐색되어 대기 목록에 뜸 | | |
| 탐색이 안 될 때 Enter the address로 붙음 | | |
| 창 닫기 → 트레이에 남음 → 트레이 Open으로 다시 열림 | | |
| 로그인 시 자동 실행(재부팅 1회) | | |
| 트레이 Quit 후 다시 실행 | | |

문제가 나오면 로그 파일로 원인을 찾아 고친다(소유 파일 안에서). 고친 것에는 `src-tauri` 단위 테스트를 같이 쓴다.

## P0-2. 윈도우 방화벽 안내

- 윈도우는 처음 포트를 열 때 방화벽 창을 띄운다. 네트워크가 "공용"이면 허용해도 막힌다.
- 앱 화면(`AppPage.svelte`)에서 윈도우일 때만 안내를 보인다(마스터·참여 모두).
  - 문구 예: "If other computers can't find this one: allow Nodeloom in Windows Firewall, and set this network to Private."
  - OS 판별은 `navigator.userAgent`로 한다. 판별 함수는 `src/lib/app.ts`에 두고 테스트를 쓴다.

## P0-3. 시연 릴리즈 `v0.1.0-beta.4`

`docs/handoff/README.md`의 "릴리즈 순서"를 따른다. 요약하면 다음과 같다.

1. A·B·C의 PR이 병합되고, E의 CHANGELOG `0.1.0-beta.4` 절이 병합된 뒤에 시작한다.
2. 버전 세 곳을 올리는 PR(`chore/38-beta4`)을 만든다. 루트 `Cargo.toml`, `src-tauri/Cargo.toml`, `tauri.conf.json`이다.
3. 그 PR을 병합한 뒤 Actions → binaries → Run workflow를 실행한다(main, release 체크).
4. 릴리즈 페이지에 11개 파일이 있는지 확인한다(명령줄 5, 데스크톱 5, `SHA256SUMS`). 링크를 팀에 공유한다.

## P1 (시간이 남으면)

- 트레이 툴팁에 이 기기 상태를 보인다(예: "Nodeloom — in the cluster"). 1~2초마다 `Running.agent`를 읽는다.
  - 트레이 갱신은 메인 스레드에서 한다(`AppHandle::run_on_main_thread`).
  - **창을 이벤트 처리기 안에서 새로 만들지 않는다.**
- `binaries.yml`의 리눅스 데스크톱 확인 단계: `xvfb-run`의 자식(Xvfb, 앱)이 남는다. 프로세스 그룹으로 정리한다.
- 윈도우 러너에서도 앱을 띄워 보는 확인 단계를 검토한다. 실패하면 릴리즈를 막으므로, 처음에는 `continue-on-error`로 관찰한다.

## 하지 말 것

- 동기 명령이나 이벤트 처리기 안에서 `WebviewWindowBuilder::build` 호출(윈도우에서 멈춤)
- 재시작할 때 `app.restart()` 직접 호출. `lib.rs`의 `restart()`를 쓴다.
- `crates/**`, 대시보드 컴포넌트(C) 수정
- 문서 직접 수정. PR 본문의 "문서 반영 요청"에 적는다.

## 진행 메모

(작업하면서 여기에 적는다)
