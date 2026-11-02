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

(작업하면서 여기에 적는다)
