# 역할 C: 대시보드 UI (이슈 #37)

- **브랜치**: `feat/37-demo-dashboard`
- **고칠 수 있는 파일**: `apps/desktop/src/**`, `apps/desktop/index.html`, `apps/desktop/public/**`
  - **제외**: `src/components/AppPage.svelte`, `src/lib/app.ts`, `src/lib/app.test.ts`(D 소유), `src/lib/types.ts`(프로토콜 사본, 고정)
- **리뷰어**: D
- 규칙과 계약은 `docs/handoff/README.md`. API에서 받는 데이터는 지금의 `ClusterSnapshot`과 `MasterInfo`뿐이다. 필요한 정보는 이 둘에서 계산한다.

## 왜 중요한가

시연을 보는 사람은 "작업이 여러 기기로 나뉘어 동시에 처리된다"와 "한 대가 빠지면 넘어간다"를 **화면으로** 이해해야 한다. 지금 Jobs 목록은 이 이야기를 한눈에 보여 주지 못한다.

## 쓸 수 있는 데이터

`src/lib/types.ts` 참고.

- `JobView.tasks[]`: `{ task_id, node, frame_start, frame_end, frames_done, state, error }`
  - `node`는 그 조각을 맡은 기기 id이고, 재배정되면 바뀐다.
  - A가 P1을 하면 한 기기가 조각을 여러 개 맡을 수 있다. 그래도 모양은 같다.
- `NodeView`: `{ info, metrics, busy_tasks }`
- 기기 이름은 `src/lib/names.ts`의 `displayNames`를 쓴다.

## P0-1. 작업 카드: 기기별 구간 막대

- 작업 하나를 프레임 축 위의 가로 막대로 그린다. 조각마다 구간이 있고, 기기별로 구분한다. 진행된 부분은 채워서 보인다.
- 구간 아래에 기기 이름, 프레임 범위, 진행률, 상태(아이콘+글자)를 보인다.
- 색은 `src/app.css` 토큰만 쓴다. 기기 구분은 색만으로 하지 않고 이름 라벨을 같이 쓴다. 라이트와 다크를 모두 확인한다.
- 계산 로직(구간 비율, 진행률, 기기별 묶기)은 `src/lib/jobs.ts`에 두고 `jobs.test.ts`로 테스트한다.

## P0-2. 재배정이 눈에 보이게

- 스냅샷 사이에 조각의 `node`가 바뀌면 "→ 다른 기기로 넘어감" 표시를 10초 동안 보인다(예: "moved from pi-4 to macbook").
- 순수 함수로 이전·다음 스냅샷을 비교한다(`src/lib/jobs.ts` 또는 새 `src/lib/moves.ts`). 테스트를 쓴다.

## P0-3. 시연용 작업 버튼

- `SubmitForm.svelte`에 "Demo job" 버튼을 둔다. 테스트 러너, 프레임 1~600, 프레임당 50ms다. 1대면 30초, 3대면 약 10초 걸린다.
- 테스트 러너가 있는 기기가 없으면 끈다. 이유를 글로 보인다.
- 지금은 기기가 없으면 Runner 선택이 비어 보인다. 그때 안내 문구("Add a machine first")를 보인다.

## P0-4. 기기 카드: 지금 하는 일

- `NodeCard.svelte`에 그 기기가 지금 맡은 조각(프레임 범위, 진행률)을 보인다. 없으면 "Idle"이다.

## P1 (시간이 남으면)

- KPI에 "frames per second(전체)"와 진행 중인 작업의 예상 남은 시간을 넣는다. 스냅샷 기록(`src/lib/history.ts`)으로 계산한다.
- 시연 화면은 프로젝터 해상도(1280×720)에서 스크롤 없이 핵심이 보이게 한다.

## 확인

- `pnpm check && pnpm test && pnpm build`
- 진짜 마스터로 화면을 확인한다. `AGENTS.md`의 "한 기기에서 전체 흐름 실행"을 따르거나, 대시보드를 담은 마스터를 쓴다.
  - 에이전트 3개를 `--enable-fake-runner`로 띄운다.
  - Demo job을 실행하고, 도중에 에이전트 하나를 Ctrl+C로 끈다.
  - 라이트·다크 화면을 캡처해 PR에 붙인다.

## 하지 말 것

- `types.ts`, `AppPage.svelte`, `lib/app.ts` 수정
- API 호출 추가. `client.ts`의 기존 메서드만 쓴다.
- 새 npm 의존성(차트 라이브러리 등) 추가. SVG나 CSS로 그린다.
- 문서 직접 수정. PR 본문의 "문서 반영 요청"에 적는다.

## 진행 메모

- 2026-11-02 P0-1~4 구현(브랜치 `feat/37-demo-dashboard`).
  - 구간 막대·기기별 묶기: `src/lib/jobs.ts`의 `jobSegments`, `workByNode`, `nodeSlots`(색은 짝지은 기기 목록 순서라 한 대가 빠져도 남은 기기 색이 안 바뀐다).
  - 재배정 감지: `src/lib/moves.ts`의 `trackMoves`. 마스터는 끊긴 기기의 조각을 `node: null`로 되돌렸다가 같은 갱신 안에서 다시 나눠 주므로, 조각마다 마지막 기기를 기억해 비교한다. 기다리는 동안은 "left: … wait for another machine…", 다른 기기가 받으면 "Moved to another machine: … A → B"를 10초 보인다.
  - 기기 색은 `app.css`의 `--series-1`~`--series-6`(dataviz 기본 팔레트, 라이트·다크 검증 통과). 7대째부터는 회색+이름.
  - Demo job은 Start a job 카드의 큰 버튼. 기존 입력 폼은 "Custom job" 접기 안으로 옮겼다(대본의 대안 경로 그대로 사용 가능).
  - 대시보드 순서를 Machines → Start a job → Jobs → Add a machine으로 바꿔 1280×720에서 작업 카드가 덜 밀리게 했다.
- 확인 필요: 작업 환경에서 npm 레지스트리가 막혀 `pnpm check/test/build`와 실제 마스터 확인은 로컬에서 해야 한다. 순수 로직 테스트(jobs·moves 19개)는 TypeScript로 따로 돌려 통과했다.
- 2026-11-03 한 화면 배치(브랜치 `feat/37-dashboard-layout`, Refs #37).
  - 맨 위 한 줄: 제목 · KPI(Machines online, Pieces running, Average CPU) · 연결 상태. 그 아래 왼쪽 연결도(`ClusterMap.svelte`), 오른쪽 기기 목록(`MachineList.svelte` + 한 줄 행 `NodeRow.svelte`, 옛 `NodeCard`).
  - 연결도 배치는 `src/lib/topology.ts`의 `mapLayout`(위 줄은 왼→오, 아래 줄은 오→왼으로 시계 방향, 라벨은 바깥쪽). 순서는 `jobs.ts`의 `machineOrder`(= `nodeSlots`와 같은 짝지은 목록 순서)라 기기가 빠져도 자리와 색이 그대로다. 1~8대 겹침 없음을 테스트로 확인.
  - 이동 배너가 떠 있는 동안 받은 기기의 선을 굵게+펄스(`receivingMachines`), reduced-motion이면 굵게만.
  - 작업은 최신 1개만 크게(`JobCard.svelte`), 나머지는 "Earlier jobs (N)"로 접음. 이동 배너는 구간 막대 아래로 옮겨 배너가 여러 개여도 막대가 화면 밖으로 밀리지 않게 했다.
  - 기기가 1대 이하이거나 승인 대기가 있으면 승인 카드와 Add a machine(펼침)이 맨 위, 아니면 접어서 맨 아래. Paired machines도 접음.
  - 행 안의 CPU 스파크라인은 뺐다(44px로는 안 보이고, 화면 낭독용 표가 표 셀 안에서 가로 스크롤을 만들었다). `Sparkline.svelte` 삭제, `history.ts`는 P1 KPI용으로 남김.
  - 세로로 쌓는 기준은 1100px 미만(창 1280이면 스크롤바·창 테두리로 뷰포트가 1280보다 조금 좁아질 수 있어서).
- 2026-11-03 한국어/영어 전환(브랜치 `feat/37-language`, Refs #37).
  - 사전과 번역 함수는 `src/lib/i18n.ts`(순수 TS라 D의 AppPage도 가져다 쓸 수 있다), 현재 언어 상태는 `src/lib/locale.svelte.ts`의 `$state`. 컴포넌트는 `t(key, params)`만 부른다.
  - 한국어 사전은 `satisfies Record<MessageKey, Message>`라 키가 빠지거나 남으면 타입 오류. 복수형은 `{ one, other }`를 `Intl.PluralRules`로 고르고, 한국어는 "기기 3대"처럼 한 형태.
  - 문장 안의 코드·굵은 글씨·링크는 `Rich.svelte`(자리 표시를 같은 이름의 snippet으로 그림)로 넣어 언어마다 어순을 따른다.
  - 화면 문장을 만들던 lib 함수(`lookalikes`, `promptProblem`, `summaryProgress`, `jobRole`, `documentProblem`, `workLabel`)는 `t`를 인자로 받는다. client.ts·pdf.ts가 직접 쓰는 오류는 `MessageError`(키+값, message는 영어), 화면은 `errorText(e, t)`로 보인다. 마스터가 보낸 오류 원문은 그대로.
  - 잡은 오류는 문자열이 아니라 오류 그대로 상태에 두어, 언어를 바꾸면 이미 떠 있는 오류 문구도 바뀐다.
  - "Join a cluster", "Enter the address"는 AppPage(D)의 버튼 이름이라 번역하지 않았다. AppPage가 번역되면 같이 바꾼다.
