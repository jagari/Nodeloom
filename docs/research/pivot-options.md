# 렌더팜을 멈추고 연구실 GPU부터 검증하라

현재 방향 (a), 곧 사용자의 맥·윈도우 기기를 묶는 Blender 분산 렌더는 **그대로 밀고 갈 근거가 약하다.** 무료 Flamenco는 Blender Studio가 계속 유지하고, 유료 CrowdRender는 Nodeloom에 없는 한 프레임 타일 분할과 Win/Mac/Linux 혼합을 월 5~30달러에 이미 판다. "기기 2대 이상으로 애니메이션을 렌더하는 개인"이 얼마나 되는지 보여 주는 공개 데이터도 없다. 세 후보 모두 학생 팀이 만든 도구에 누군가 돈을 냈다는 증거는 없다. 그래서 지금 고를 일은 "어느 쪽으로 갈지"가 아니라 "2주 안에 어느 가설을 가장 싸게 죽이거나 살릴 수 있는지"다. 이 기준에서는 (b) 대학 연구실·동아리 GPU 공유가 1순위다. 국내 연구실이 GPU 점유 갈등을 사람이 지키는 규칙과 예약 캘린더로 막고 있다는 1차 흔적이 여럿 있고, 팀이 캠퍼스 안에서 직접 사용자를 만나 "서버에 설치해도 된다"는 가장 강한 신호를 받아 낼 수 있다. 다만 (b)는 AGENTS.md 절대 규칙 1번(닫힌 `TaskPayload`, 임의 명령 금지)과 정면으로 부딪힌다. 규칙을 지키려면 **작업을 실행하지 않는 관측·예약 전용 제품**으로 시작해야 하고, 그러면 Nodeloom의 "작업 분배"라는 정체성은 당분간 사라진다. (c) 로컬 AI 추론 클러스터는 관심 지표가 가장 크지만(exo 별 47.8k), 큰 엔진들이 2026년에 대시보드를 하나씩 내장하고 있어 남는 틈이 빠르게 줄고 있다. 권고는 (a)의 신규 기능 개발을 멈추고, 2주 동안 (b)를 관측 전용으로 검증하면서 같은 GPU 지표 작업을 재사용해 (c)를 가볍게 확인한 뒤, 미리 정한 기준으로 하나를 고르는 것이다.

> **근거 표기 방식.** 모든 출처의 확인 날짜는 **2026-10-12**다. 각 사실 옆에 세 가지 표시를 붙였다. **원문 확인**은 조사 노트가 페이지를 직접 열어 읽었다고 밝힌 경우(주로 github.com 저장소 페이지)와 이 저장소의 로컬 코드다. **검색 요약 기준**은 원문을 열지 못하고 검색 엔진이 뽑은 문장에만 근거한 경우다. 노트가 열람 여부를 밝히지 않은 출처도 보수적으로 여기에 넣었다. **확인 못함**은 조사에서 답을 찾지 못한 경우다. 후보 (a)의 노트는 WebFetch가 모든 외부 도메인에서 DNS 오류로 실패해 **전부 검색 요약 기준**이다. 노트와 이 보고서에 나오는 판정 기준 숫자(예: "인터뷰 대상의 절반 이상")는 **업계 기준이 아니라 팀이 정할 제안값**이다.

## 여섯 기준 중 (a)가 앞서는 곳은 코드 재사용 하나뿐이다

아래 표는 세 노트를 같은 기준으로 다시 정리한 것이다. "판단" 열의 높음·중간·낮음은 이 보고서의 평가이며 출처가 있는 수치가 아니다.

| 기준 | (a) Blender 분산 렌더 (현재) | (b) 연구실·동아리 GPU 공유 | (c) 로컬 AI 추론 클러스터 관리 |
|---|---|---|---|
| 수요 신호 | **낮음.** 도구는 많지만 "기기 2대 이상 + 정기 렌더" 사용자 규모는 확인 못함. Reddit 글도 확인 못함 | **중간.** 국내 연구실의 점유 갈등 규칙·예약 제도·자작 도구가 보임(대부분 검색 요약 기준). 정량 설문은 국내에 없음 | **전 세계 관심은 높음**(exo 47.8k★, 원문 확인). 다만 확인된 불만은 엔진 안정성 쪽이고 "모니터링이 없어 힘들다"는 불만은 확인 못함 |
| 경쟁자와 가격 | 무료 Flamenco·Pidgeon·BlendFarm·SheepIt, 유료 CrowdRender 월 5~250달러·RenderWorkshop 49.99달러·RenderX 10달러부터(모두 검색 요약 기준) | 무료 Slurm·KAI·SkyPilot·dstack·nvitop, 상용 Run:ai GPU당 연 4,500달러(2차 출처)·Backend.AI(가격 비공개). 소규모 연구실 지불 사례는 못 찾음 | 무료 exo·LocalAI·GPUStack·llama.cpp RPC·distributed-llama 등. 공개 가격표는 없음. GPUStack Enterprise는 문의형 |
| 기존 코드 재사용 | **가장 높음**(현재 제품). 단 에셋 동기화·결과 수집·서명 설치 파일이 비어 있음 | **중간.** 연결·인증·mDNS·재배정·대시보드는 재사용, 프레임 분할은 거의 못 씀. 노트는 절반 이상을 신규로 추정 | **중간~높음.** 에이전트·mDNS·토큰·대시보드 재사용, 맥·윈도우 대상도 같음. 엔진 어댑터·GPU/Metal 지표는 신규 |
| 팀 역량 적합도(Rust 경험 적은 5명) | **낮음.** 남은 핵심(에셋 동기화, 타일 분할)은 CrowdRender도 수년째 버그가 이어지는 영역 | **관측 전용이면 중간.** 실행·격리·다중 사용자까지 가면 낮음 | **중간.** HTTP 폴링 어댑터는 쉬운 편. Apple GPU 지표의 권한 문제는 확인 못함 |
| 학교 안 사용자 접근성 | 중간. 영상·애니메이션·게임 전공 학생이 대상이나 동아대 내 규모는 확인 못함 | **가장 높음.** 캠퍼스 연구실·AI 동아리를 직접 찾아갈 수 있음. 단 동아대 GPU 현황은 확인 못함 | 낮음. 국내 대학 연구실 사례를 거의 찾지 못함 |
| 절대 규칙 1번과의 관계 | 맞음(`Blender` 변형) | **충돌.** ML 작업은 본질적으로 임의 코드 | 맞출 수 있음(엔진 실행을 새 `TaskPayload` 변형으로, 노트의 추론) |
| 2주 검증 핵심 | 경쟁 도구와 나란히 설치·렌더해 앞서는 단계가 있는지 | 연구실 8곳 이상 인터뷰 + 관측 전용 에이전트 설치 허락 | 팀 기기로 엔진 세팅 고통 기록 + 인터뷰 + 대기자 수 |

표가 말하는 것은 단순하다. (a)의 유일한 우위는 이미 쓴 코드인데, 그 코드가 경쟁 도구보다 앞서는 부분(프레임 범위 균등 분할 + 대시보드)은 이 시장에서 가장 단순한 기능이다. 이미 쓴 코드는 매몰 비용이다. 오히려 Nodeloom에서 재사용 가치가 큰 자산은 프레임 분할이 아니라 **에이전트-마스터 연결, mDNS 발견, 페어링 토큰, WebSocket 대시보드**다. 이 자산은 세 후보 모두에 옮겨 갈 수 있다.

## (a) Blender 분산 렌더는 무료 Flamenco와 유료 CrowdRender 사이에 끼어 있다

**경쟁 구도부터 불리하다.** CrowdRender는 Free, Basic(연간 결제 시 월 5달러, 월 결제 시 8달러), Pro(월 12달러), Studio(월 30달러), Intergalactic(월 250달러) 다섯 등급을 둔다. 검색 도구는 이 페이지의 요약문이 일부 깨져 있었다고 경고했다([CrowdRender Download](https://discovery.crowd-render.com/download), 검색 요약 기준). 예전 문서는 한 프레임을 타일로 나눠 노드에 배정하고 Linux·macOS·Windows 혼합이 가능하다고 설명한다([CrowdRender Documentation V03X-V04X](https://www.crowd-render.com/documentation-v030), 검색 요약 기준, 오래된 문서). 무료 쪽에서는 Flamenco 3.8이 2025-12-05에 Blender 5.0 호환으로 나왔다. 날짜는 Mastodon 게시물 ID로 계산했다([Blender Mastodon](https://mastodon.social/@Blender/115667701402891929), 검색 요약 기준). Blender 5.0 자체는 2025-11-18에 나왔다([Blender 5.0 Release Notes](https://developer.blender.org/docs/release_notes/5.0/), 검색 요약 기준). Pidgeon Render Farm은 무료 P2P 방식이지만 **macOS 바이너리가 없다**([GitHub PidgeonTools/PidgeonRenderFarm](https://github.com/PidgeonTools/PidgeonRenderFarm), 검색 요약 기준). 소규모 유료 도구로는 RenderWorkshop 49.99달러([Gumroad](https://spellbinder6.gumroad.com/l/renderworkshop), 검색 요약 기준)와 RenderX 10달러부터([Gumroad](https://blacktom.gumroad.com/l/renderx), 검색 요약 기준)가 있다. Flamenco 개발자는 SIGGRAPH 2025에서 "Please Don't Write a New Render Farm"이라는 제목으로 발표했다([talks.stuvel.eu](https://talks.stuvel.eu/2025-08-siggraph/index-siggraph-2025-flamenco.html), 제목만 검색 요약 기준, 본문 확인 못함). 이 제목은 새 렌더팜을 만드는 일을 업계가 어떻게 보는지 보여 준다.

**같은 사용자를 두고 클라우드 팜도 경쟁한다.** SheepIt은 무료지만 공식 FAQ 기준 동시 렌더 3개(붐비면 1개)이고, EU 미러 FAQ는 2개라고 적어 출처끼리 다르다([SheepIt FAQ](https://www.sheepit-renderfarm.com/faq), [SheepIt EU FAQ](https://www.europe.sheepit-renderfarm.com/faq), 검색 요약 기준). 프로젝트 크기 한도는 2024년에 2GB로 늘었다([SheepIt News](https://www.sheepit-renderfarm.com/news/1711465057), 검색 요약 기준). RebusFarm 자체 가격 페이지는 CPU 1.20센트/GHzh, GPU 0.45센트/OBh와 **학생 할인 50%**를 적는다([RebusFarm Prices](https://rebusfarm.net/buy/products), 검색 요약 기준). 다만 Blendergrid는 같은 회사를 1.41센트/0.53센트로 적어 숫자가 다르고, 이유는 확인 못함이다([Blendergrid 비교 2026](https://blendergrid.com/articles/best-blender-render-farms-compared), 검색 요약 기준). 가끔 마감 때만 렌더하는 학생에게는 할인된 클라우드 팜이 월 구독보다 싸다. 그래서 Nodeloom Pro가 이기려면 "자주, 반복해서" 렌더하는 사용자를 찾아야 한다. 그런 사용자의 규모는 아무도 측정하지 않았다.

**수요의 크기는 확인할 수 없다.** blender.org 다운로드가 2020년에 약 1,400만 회였다는 제3자 인용이 있다([Polyfable](https://polyfable.com/tutorials/how-many-blender-users-are-there/), 검색 요약 기준, 2020년 수치). Blender Feedback Survey 2025는 5,102명이 참여했고 재단 스스로 적극 사용자 쪽으로 치우쳤다고 밝혔다([Blender Feedback Survey 2025](https://survey.blender.org/feedback/2025/), 검색 요약 기준). 하지만 렌더팜 사용률이나 보유 기기 수 문항은 확인 못함이고, r/blender의 "여러 PC로 렌더" 스레드도 검색에 나오지 않았다(확인 못함). 노트의 해석대로 무료 도구 4개 이상과 유료 도구 3개 이상이 작게 나눠 가진 모습은 "수요는 있지만 돈을 크게 쓰는 시장은 아니다"로 읽는 것이 맞다.

**남은 일이 가장 어렵다.** 경쟁 도구가 실제로 막히는 곳은 분배 알고리즘이 아니라 파일·에셋 동기화와 OS별 경로다. CrowdRender는 동기화 실패와 업로드 중 멈춤을 변경 기록에 적고 있다([CrowdRender Download 변경 기록](https://discovery.crowd-render.com/download), 검색 요약 기준). Flamenco는 공유 저장소 마운트가 필요하고, Windows에서는 UNC 경로가 안 된다는 이슈가 있다([Flamenco Manager Configuration](https://flamenco.blender.org/usage/manager-configuration/), [Flamenco issue #100540](https://projects.blender.org/studio/flamenco/issues/100540), 검색 요약 기준). 타일 분할은 CUDA GPU+CPU와 적응형 샘플링을 함께 쓸 때 타일 사이에 시각 차이가 생기는 버그(T82717)와 바로 부딪힌다([developer.blender.org T82717](https://developer.blender.org/T82717), 검색 요약 기준, 현재 상태 확인 못함). Mac(Metal)과 Windows(CUDA/OptiX)가 같은 장면에서 같은 결과를 낸다는 공식 보증도, 다르다는 공식 벤치마크도 확인 못함이다. 설치 파일 서명 비용 역시 확인 못함이다. Nodeloom 편에서 가장 강하게 반론하면 이렇다. Pidgeon에는 macOS가 없고 Flamenco는 공유 저장소 설정이 번거로우므로 "맥과 윈도우를 설치 몇 번으로 묶는 경험"이라는 틈이 있다. 또 Metal과 CUDA 결과를 비교하는 "일관성 검사"는 경쟁 도구에서 눈에 띄지 않는다. 하지만 CrowdRender가 이미 같은 틈을 노리고 있고, 이 틈을 메우려면 결국 가장 어려운 에셋 동기화를 먼저 해야 한다. (a)를 살릴 조건은 인터뷰에서 "설치가 어렵다"나 "맥과 윈도우를 섞으면 결과가 다르다" 같은 **구체적 불만이 반복될 때**로 좁혀야 한다.

## (b) 연구실 GPU 공유는 학교 안 고통이 가장 잘 보이지만 규칙 1번과 지불 의사에 걸린다

**국내 연구실은 스케줄러보다 가시성과 합의가 부족하다.** 한 GitHub 저장소의 "DSW A6000×4 서버 사용 규칙"은 "시작 전에 nvidia-smi로 메모리와 실행 중 프로세스를 확인", "전체 GPU 점유는 예상 시작·종료 시각을 사전 공지", "타인의 프로세스를 임의로 종료하지 않습니다"라고 적는다([jeongiryang/wave-barrier-free-gyeongnam #475](https://github.com/jeongiryang/wave-barrier-free-gyeongnam/issues/475), 검색 요약 기준). **GPU 4개짜리 서버 한 대에서도 점유 갈등을 규칙으로 막고 있다는 흔적이다.** 충북대 소프트웨어학부는 2025-06-10부터 서버 1대를 최대 4명이 쓰고 1인당 GPU 2개를 배정하며, 대여신청서와 시설 예약 시스템과 사무실 승인을 거친다([충북대 소프트웨어학부 공지](https://software.cbnu.ac.kr/sub0401/196608), 검색 요약 기준). 고려대 공동기기원은 GPU 클라우드를 웹 예약으로 운영한다([고려대 공동기기원](https://crf.korea.ac.kr/front/org/machine/723?search_org_seq=19), 검색 요약 기준). 경희대 과학지능융합전공은 GPU 노드 4대에 Slurm을 강제하고 "자원 독과점으로 판단되는 작업은 관리자가 임의로 종료"할 수 있게 했다([경희대 과학지능융합전공 HPC](https://sichpc.khu.ac.kr/), 검색 요약 기준). 연구실 서버 31대를 관리한 경험담은 서버를 개인에게 나눠 주면 유휴 자원이 낭비되고, 동적 배정은 유지보수 부담이 크다고 비교한다([velog @bismute](https://velog.io/@bismute/20%EC%96%B5%EC%9B%90-%EC%97%B0%EA%B5%AC%EC%8B%A4-%EC%84%9C%EB%B2%84-31%EB%8C%80-%EA%B4%80%EB%A6%AC-%EB%B0%8F-%EC%9C%A0%EC%A7%80%EB%B3%B4%EC%88%98-%ED%95%98%EA%B8%B0), 검색 요약 기준). 직접 도구를 만든 연구실도 있다. `gpu-lab`은 한국어 연구실 GPU 예약·모니터링 포털이다. 설계 원칙은 "Soft Reservation — no enforcement"로, GPU 권한과 프로세스를 건드리지 않고 예약과 실제 사용을 비교한다. 별은 0개, 커밋은 22개다([GitHub mansrl0929/gpu-lab](https://github.com/mansrl0929/gpu-lab), 원문 확인). 해외 근거로는 NLP 연구자 설문(2023)에서 **57.4%가 연산 자원 부족으로 실험을 못 했고, 62%가 GPU 8개 미만에 접근**한다는 결과가 있다([arXiv 2306.16900](https://arxiv.org/pdf/2306.16900), 검색 요약 기준). 이 수치는 소규모 GPU 연구실이 다수라는 규모 근거가 된다. 그러나 **국내 대학원생의 GPU 공유 불편을 정량으로 잰 설문은 확인 못함**이다. 노트의 해석처럼 비어 있는 자리는 "Slurm은 과하고 nvidia-smi와 단톡방·캘린더로는 부족한" 서버 1~5대 연구실이다.

**스케줄러 엔진으로는 경쟁할 수 없다.** Slurm은 무료지만 컨트롤러, 노드 공통 `slurm.conf`, Munge, `gres.conf` 설정이 필요하다([SLURM in the Wild](https://mtreviso.github.io/blog/slurm.html), 검색 요약 기준). NVIDIA는 Run:ai의 스케줄링 엔진을 KAI Scheduler라는 이름으로 2025년 4월 Apache-2.0으로 공개했다. 별은 1.6k이고 Kubernetes, Helm, GPU Operator가 필수다([GitHub NVIDIA/KAI-Scheduler](https://github.com/NVIDIA/KAI-Scheduler), 원문 확인). **핵심 스케줄러의 가격이 0이 된 셈이다.** 상용 Run:ai는 2차 출처 기준 **GPU당 연 4,500달러**이고 NVIDIA 공식 가격표는 확인 못함이다([ZenML 비교](https://www.zenml.io/blog/kai-scheduler-vs-runai), 검색 요약 기준). 이 가격이면 GPU 8개 연구실이 연 36,000달러를 내야 하므로 소규모 연구실에는 현실성이 없다. dstack은 온프레미스 서버를 SSH fleet으로 묶고 2026년 8월에 0.21.0을 냈다(별 2.3k, [GitHub dstackai/dstack](https://github.com/dstackai/dstack), 원문 확인). SkyPilot은 별 10.7k, nvitop은 별 7.2k로 활발하다([GitHub skypilot-org/skypilot](https://github.com/skypilot-org/skypilot), [GitHub XuehaiPan/nvitop](https://github.com/XuehaiPan/nvitop), 원문 확인). 국내에서는 Lablup의 Backend.AI가 2019년 국민대 도입으로 엔터프라이즈 시장에 들어갔다([Backend.AI 제품](https://www.backend.ai/ko/product/Backend.AI), 검색 요약 기준, 가격 비공개). 국내 대학이 기관 단위로 GPU 관리 소프트웨어를 산 선례는 있다는 뜻이다. 그러나 **2~20 GPU 연구실이 스케줄러 소프트웨어에 돈을 낸 사례는 국내외 모두 찾지 못했다.** 남은 차별점은 노트의 정리대로 30분 안에 끝나는 설치, 한국어 UI, 예약과 실사용 대조, 연구실 규칙 자동화, 교수·조교용 사용 보고서 같은 "운영 UX"다. 수익은 연구실 구독보다 학과·단과대 단위 기능(여러 연구실 통합 보고서, SSO, 사용량 정산, 감사 로그)에서 찾아야 한다는 것이 노트의 추정이다.

**정부 GPU가 수요를 키울 수도, 줄일 수도 있다.** 「첨단 GPU 활용 지원 사업」(2026)은 연구실(교수)당 1개 프로젝트로 H200이나 B200을 배정한다. 수시 모집(2026.04.01~07.09)은 자원이 모두 배정되어 접수가 끝났다. 주관기관 표기는 NIPA와 KAIT로 출처마다 달라 확인 못함이다([서울대 CSE 공지](https://cse.snu.ac.kr/en/community/notice/25036), [aiinfrahub 공고 05](https://aiinfrahub.kr/project/05), 검색 요약 기준). 과기정통부는 약 2천 장을 산·학·연에 추가 지원한다고 발표했다([ZDNet Korea 2026-03-16](https://zdnet.co.kr/view/?no=20260316162951), 검색 요약 기준). 국가슈퍼컴퓨터 6호기 '한강'은 사용자 접수가 2026-09-28~10-16이다. 새로 생긴 '탐색연구'는 최소 검토로 1노드(GH200 4개) 상당을 준다([전자신문 2026-09-09](https://www.etnews.com/20260909000035), [아시아경제 2026-09-08](https://view.asiae.co.kr/article/2026090820573771804), 검색 요약 기준). 이 자원은 경쟁·소진형이라 일상 실험은 여전히 연구실 자체 서버에 의존할 가능성이 크다. 반대로 정부 자원이 커질수록 자체 서버 관리 도구의 중요도가 떨어질 수도 있다. 노트는 두 해석 모두 추정이라고 밝힌다. **동아대 내부 GPU 현황은 확인 못함**이다.

### 규칙 1번이 (b)에 주는 결과: 실행을 포기하거나 규칙을 바꾸거나

이 부분이 (b) 결정의 핵심이다. ML 학습 작업은 본질적으로 "사용자가 쓴 임의 Python 코드를 GPU에서 실행"하는 일이다. 기존 도구는 임의 코드를 막지 않는다. 대신 위험을 두 가지로 관리한다. 하나는 누가 실행했는지(유닉스 계정, Munge 인증)이고, 다른 하나는 어디서 실행되는지(컨테이너, 권한)다. Slurm은 작업을 제출한 유닉스 사용자 권한으로 실행한다([greennode Slurm 가이드](https://greennode.ai/blog/set-up-slurm-cluster), 검색 요약 기준). Backend.AI는 GPU를 컨테이너에 장치 단위로 할당한다([Lablup GitHub](https://github.com/lablup), 검색 요약 기준). 반면 Nodeloom의 `TaskPayload`는 `Fake { frame_start, frame_end, millis_per_frame }`와 `Blender { blend_file, frame_start, frame_end }` 두 변형뿐인 닫힌 enum이다(`crates/nodeloom-proto/src/lib.rs`, 원문 확인). AGENTS.md 절대 규칙 1번은 "명령 문자열을 받아 실행"하는 기능 추가를 금지한다. 그래서 (b)를 고르면 아래 셋 중 하나를 골라야 한다.

| 선택지 | 규칙 1번 | 얻는 것 | 잃는 것·필요한 것 |
|---|---|---|---|
| ① 관측·예약 전용(실행 안 함) | **지킨다** | gpu-lab과 같은 방향. Rust 에이전트의 가벼움과 데스크톱 대시보드가 차별점 | **작업 큐와 재배정이라는 Nodeloom의 핵심 강점을 버린다.** 제품이 "GPU 사용 현황판 + 예약"이 된다 |
| ② `TaskPayload::Container { image, args, workdir }` 같은 변형 추가 | enum 모양은 유지하지만 **컨테이너 안에서는 사실상 임의 코드가 돌아 규칙의 취지와 충돌** | 진짜 작업 큐 | 이슈 합의, `docs/DECISIONS.md` 기록, A 승인이 필요하다. 페어링 토큰 하나로는 부족해 **사용자별 인증**(규칙 4번 재설계), 감사 로그, rootless 컨테이너가 필요하다. "임의 명령을 실행하지 않아서 안전하다"는 사업 메시지를 더는 쓸 수 없다 |
| ③ Slurm·dstack 같은 기존 실행기 위의 UI·예약 계층 | 실행 책임을 넘기므로 지킨다 | 실행 보안을 기존 도구에 맡김 | Nodeloom 에이전트의 역할이 줄고, 대상 연구실이 이미 그 실행기를 써야 한다 |

결론은 분명하다. **2주 검증 안에서는 ①만 가능하다.** ②는 보안 모델 전체를 다시 설계하는 일이고, Rust 경험이 적은 팀이 2주 안에 해서는 안 된다. 재배정의 가치도 Blender보다 작다. ML 학습은 체크포인트 규약 없이 재시작하면 처음부터 다시 돌기 때문이다. 또 연구실 GPU 서버는 Linux가 주류일 것으로 추정되어(GPUStack 워커도 Linux만 지원, [GitHub gpustack/gpustack](https://github.com/gpustack/gpustack), 원문 확인), 지금 Nodeloom이 겨냥하는 맥·윈도우 데스크톱과 대상 플랫폼이 다르다.

코드 측면에서 재사용할 수 있는 것은 노트의 로컬 확인대로 Rust 약 2,222줄과 UI(TS·Svelte) 약 1,400줄 중 연결 골격, `tokens_match`, mDNS, `requeue`·`register_node` 같은 마스터 상태 관리와 `NodeCard`·`Meter`·`Sparkline`·`StatusBadge` 같은 UI 컴포넌트다(원문 확인). 새로 만들어야 하는 것은 NVML 기반 GPU별 사용률·VRAM·온도·전력·**GPU를 쓰는 프로세스와 그 사용자** 수집, 예약 캘린더, 사용자별 보고서다. Rust NVML 바인딩(nvml-wrapper)의 상태는 확인 못함이다. Docker 환경에서는 nvidia-smi가 프로세스를 컨테이너 런타임으로 보여 실제 사용자를 알기 어렵다는 점도 설계에 반영해야 한다([DEV: My GPU Was Busy](https://dev.to/sikamikanikobg/my-gpu-was-busy-i-finally-found-out-who-was-using-it-1pkg), 검색 요약 기준).

## (c) 로컬 AI 추론 클러스터는 관심은 가장 크지만 엔진들이 대시보드를 흡수하고 있다

**관심 지표는 셋 중 가장 강하다.** exo는 별 47.8k, Apache-2.0이고, 기기끼리 자동으로 서로를 찾으며 localhost:52415에 클러스터 관리 대시보드를 내장했다([GitHub exo](https://github.com/exo-explore/exo), 원문 확인). LocalAI는 별 49.5k이고 **2026년 9월 4.10.0에서 "fleet operations dashboard"를 넣었다**([GitHub LocalAI](https://github.com/mudler/LocalAI), 원문 확인). AMD는 2026년 2월 Framework Desktop 4대를 llama.cpp RPC로 묶어 1조 파라미터 모델을 돌리는 가이드를 냈다([AMD 기술 문서](https://www.amd.com/en/developer/resources/technical-articles/2026/how-to-run-a-one-trillion-parameter-llm-locally-an-amd.html), 검색 요약 기준). Apple은 2026년 8월 Mac Studio를 발표하며 Thunderbolt 5 RDMA 클러스터링을 마케팅했고 "4대 묶으면 3배 빠른 추론"이라고 주장했다. 이 수치는 Apple 자체 주장이다([MacRumors 2026-08-26](https://macrumors.com/2026/08/26/new-mac-studio-can-be-clustered-together/), 검색 요약 기준).

**하지만 빈자리가 빠르게 줄고 있다.** 자동 발견, 배치, 내장 UI를 모두 갖춘 도구가 이미 exo(Mac), LocalAI(다중 OS), GPUStack(Linux GPU, Grafana 통합)이다. GPUStack은 실시간 "GPU Cluster Topology" 뷰를 Enterprise 전용으로 분리했다([GitHub gpustack/gpustack](https://github.com/gpustack/gpustack), 원문 확인). 엔진 공통 관측 층에도 yokai, LlamaMan, LLMStats 같은 작은 프로젝트가 들어와 있다([pkg.go.dev yokai](https://pkg.go.dev/github.com/spencerbull/yokai@v0.22.5), [GitHub llamaMan](https://github.com/nullata/llamaMan), [Unraid LLMStats](https://ca.unraid.net/apps/llmstats-0vmakla05h6j3u), 검색 요약 기준). 엔진 공통 층을 노리면 경쟁 엔진이 대시보드를 내장할 때마다 가치가 줄어든다. 2026년에 이미 세 엔진이 그렇게 했다. 공개 가격표가 붙은 유료 제품도 확인 못함이다. GPUStack Enterprise는 문의형이고([aicoolies](https://aicoolies.com/pricing/gpustack), 검색 요약 기준, 애그리게이터), LM Link는 프리뷰 기간 무료이며 정식 출시 때 유료 등급을 둘 계획이라고만 알려졌다([letsdatascience](https://letsdatascience.com/news/lm-studio-adds-iphone-to-mac-link-for-local-llms-97175e03), 검색 요약 기준, 2차 출처).

**확인된 고통은 관측 도구가 고칠 수 없는 쪽이다.** 검색 요약에 따르면 한 리뷰는 exo가 1.0 이후에도 신뢰성 문제와 크래시와 설정 마찰을 겪는다는 HN 반응을 모았고, "2대를 넘기면 버그가 많다"는 평도 있다([LobsterPack](https://www.lobsterpack.com/blog/exo-distributed-ai-cluster-macs/), 검색 요약 기준). 이것은 엔진 내부의 안정성 문제다. Nodeloom 저장소가 전제하는 물리 한계도 같은 방향을 가리킨다. Thunderbolt 약 5GB/s는 GPU 메모리 대역폭(수십~수백 GB/s)보다 훨씬 느리다(AGENTS.md, 원문 확인). 그러니 (c)에서 Nodeloom이 주장할 수 있는 가치는 처리량이 아니라 **"세팅 시간 단축과 고장 진단"**으로 한정된다. "모니터링이 없어서 힘들다"는 불만은 직접 확인 못함이고, r/LocalLLaMA 스레드도 검색에 나오지 않았다.

**기술 적합도는 셋 중 가장 깔끔하다. 이것이 (c)를 가장 강하게 옹호하는 근거다.** llama.cpp RPC는 발견 기능 없이 `--rpc 192.168.88.10:50052`처럼 host:port를 손으로 적고, 모니터링 없이 디버그 로그만 있다. README는 "fragile and insecure"이며 "Never run the RPC server on an open network or in a sensitive environment!"라고 경고한다([llama.cpp RPC README](https://github.com/ggml-org/llama.cpp/blob/master/tools/rpc/README.md), 원문 확인). Nodeloom의 mDNS 발견과 페어링 토큰은 이 두 빈칸에 그대로 들어맞는다. RDMA 협상이 실패하면 조용히 TCP로 내려간다는 점은 "어느 링크가 느려졌나"를 보여 주는 진단 기능의 실제 신호가 된다(같은 출처). 엔진 실행(`llama-server`, `rpc-server`, `exo`)을 새 `TaskPayload` 변형으로 넣으면 규칙 1번과도 맞출 수 있다(노트의 추론). 대상 플랫폼도 맥과 윈도우로 지금 Nodeloom과 같다. 그러나 이 적합도는 "만들 수 있다"는 뜻일 뿐 "필요하다"는 뜻은 아니다. 한국 맥락에서는 클리앙 뉴스 게시판 글 하나가 검색에 걸렸을 뿐 본문은 확인 못함이고([clien.net](https://www.clien.net/service/board/news/19155566)), 국내 대학 연구실의 로컬 LLM 클러스터 운영 실태도 확인 못함이다. 2주 안에 캠퍼스에서 실제 사용자를 만날 가능성은 (b)보다 낮다.

## 결론과 권고

이번 비교에서 새로 드러난 점이 둘 있다. 첫째, Nodeloom이 지켜야 할 자산은 "Blender 프레임 분할"이 아니라 **"가벼운 에이전트 + 자동 발견 + 토큰 인증 + 한눈에 보는 대시보드"**다. 그리고 (b)와 (c)는 같은 첫 작업, 곧 에이전트에 GPU 지표를 붙이는 일을 공유한다. 방향을 바꿔도 버리는 코드는 생각보다 적고, 두 후보를 동시에 싸게 시험할 수 있다. 둘째, 학생 팀에게 결정적인 변수는 시장 크기가 아니라 **2주 안에 진짜 사용자를 만나 설치 허락을 받을 수 있는가**다. 이 점에서 (b)가 앞선다. 반대로 (b)의 지불 의사 증거가 가장 약하다는 점은 숨기지 말아야 한다. (b)가 성공해도 처음 1년은 "교내 실사용 사례가 있는 오픈소스"에 가깝고, 오픈코어 수익은 학과·단과대 단위 기능에서 나와야 한다.

권고는 다음과 같다. **(a)의 신규 기능 개발(에셋 동기화, 타일 분할, 서명 설치 파일)을 지금 멈춘다.** 기존 코드는 그대로 두되, 1~3일짜리 경쟁 도구 비교로 (a)를 공식적으로 닫거나 좁힐지 판정한다. **2주 동안 (b)를 규칙 1번을 바꾸지 않는 "관측·예약 전용"으로 1순위 검증한다.** 같은 GPU 지표 코드에 llama-server `/metrics`와 Ollama `/api/ps` 폴링만 얹어 (c)를 가볍게 확인한다. 2주 뒤 아래 판정 기준으로 하나를 고른다. 컨테이너 실행(선택지 ②)은 (b)가 판정을 통과하고, 이슈 합의와 A 승인과 `docs/DECISIONS.md` 기록을 거친 뒤에만 논의한다.

## 2주 실행 계획

역할은 AGENTS.md를 따른다(A 총괄·코어 리뷰, B 에이전트·러너, C 데스크톱 UI, D 배포·실기기 검증, E 문서·사업화·QA). 2026-10-13(화)에 시작하면 2026-10-26(월)이 판정일이다.

| 기간 | 담당 | 할 일 | 산출물 |
|---|---|---|---|
| 1일차 | 전원(E 주도) | 아래 판정 기준을 숫자까지 미리 합의하고 이슈와 `docs/DECISIONS.md` 초안에 기록한다. 결과를 보고 기준을 바꾸지 않는다 | 판정 기준 문서 |
| 1~3일차 | D | (a) 마감 비교. 팀의 맥 1대와 윈도우 1대로 같은 Blender 데모 애니메이션을 CrowdRender Free, Flamenco, SheepIt, Nodeloom으로 렌더한다. 설치부터 첫 결과까지 시간, 막힌 지점, 결과 수집 방법을 기록한다. 여유가 있으면 같은 프레임을 Metal과 CUDA/OptiX로 렌더해 픽셀 차이를 잰다 | 비교표. Nodeloom이 앞서는 단계가 있는지 |
| 1~3일차 | E | 동아대 AI·컴퓨터공학·전자 계열 연구실과 AI 동아리 중 GPU 서버를 2대 이상 공유하는 곳 8~10곳을 찾는다. 학과 사무실에 공용 GPU 서버 운영 규정이 있는지 묻는다 | 대상 목록, 연락처 |
| 1~2일차 | B (A 리뷰) | Rust NVML 바인딩의 상태를 확인하는 짧은 스파이크. 막히면 `nvidia-smi` 출력 파싱으로 대체할지 A가 결정한다(외부 프로그램은 인자 목록으로 실행, 규칙 3번) | 기술 판단 메모 |
| 3~7일차 | B, C (A 리뷰) | **실행 기능 없이** 에이전트에 GPU별 사용률·VRAM·온도·전력·GPU 프로세스와 사용자 수집을 더하고, 대시보드에 GPU 카드를 띄운다. `TaskPayload`는 건드리지 않는다. 프로토콜을 바꾸면 `types.ts`도 함께 고치고 Rust·UI 테스트를 모두 돌린다(규칙 7번) | 관측 전용 빌드(Linux) |
| 3~8일차 | E, C | 연구실 인터뷰 8곳 이상(서버 관리자와 일반 사용자 각각). 질문: 지난 2주 GPU 대기 시간, 누가 쓰는지 단톡방에 물은 횟수, 관리에 쓰는 주당 시간, 지금 도구, 정부 GPU나 KISTI 6호기를 신청했는지와 그래도 자체 서버가 필요한지, **우리 에이전트를 서버에 깔게 해 줄 수 있는지**, 돈을 낸다면 누구 예산에서 얼마인지 | 인터뷰 기록, 설치 허락 수 |
| 4~7일차 | D | (c) 가벼운 확인. 팀 기기에 llama.cpp RPC, exo, LocalAI를 세팅하며 단계 수, 시간, "상태를 몰라 답답했던 순간"을 기록한다. 관측 빌드에 llama-server `/metrics`와 Ollama `/api/ps` 폴링을 얹은 데모 영상을 만든다 | 세팅 고통 기록, 데모 영상 |
| 7~12일차 | E | (c) 데모와 짧은 설명을 국내외 커뮤니티에 올리고 대기자 등록과 인터뷰 응답을 받는다(목표 5명 이상 대화) | 대기자 수, 인터뷰 기록 |
| 8~14일차 | D | 설치를 허락한 연구실 1~2곳 서버에 관측 빌드를 깔고 약 1주 동안 GPU별 유휴 시간 비율을 잰다 | 실측 유휴율 |
| 14일차 | 전원 | 1일차에 합의한 기준으로 판정하고 결과를 `docs/DECISIONS.md`와 `docs/STATUS.md`에 남긴다 | 방향 결정 |

판정 기준은 노트가 제안한 값을 옮긴 것이다. **근거 있는 업계 기준이 아니라 팀이 1일차에 확정할 출발점이다.** (b)는 다음 네 가지를 본다. 인터뷰 대상의 절반 이상이 최근 2주 안에 대기나 갈등을 겪었는가. 설치를 허락한 연구실이 2곳 이상인가. 실측 유휴율이 의미 있게 높은가. 관리자가 "이 화면이면 단톡방 질문이 줄겠다"고 말하는가. 신호가 약하면 Slurm·dstack 위의 한국어 대시보드·예약 계층(선택지 ③)으로 좁히거나 (b)를 접는다. (c)는 노트의 예시로 인터뷰 10명 중 5명 이상이 관측·진단을 상위 3개 고통으로 꼽고 대기자 30명 이상이면 진행한다. 2주 일정상 (c)의 인터뷰 수가 모자라면 "보류"로 처리하고 무리하게 판정하지 않는다. (a)는 "기기 2대 이상 + 정기 애니메이션 렌더 + 기존 도구 불만"인 사람이 거의 없거나, 1~3일차 비교에서 CrowdRender·Flamenco보다 명확히 나은 단계가 하나도 없으면 접는다. 반대로 같은 구체적 불만이 반복되면 그 하나만 푸는 좁은 제품으로 줄인다.

## 열린 질문

1. 동아대 안에 GPU 서버를 공유하는 연구실·동아리가 몇 곳이고, 학과 공용 서버나 운영 규정이 있는가? 세 노트 모두 확인 못함이다.
2. 정부 GPU(첨단 GPU 활용 지원, 과기정통부 추가 2천 장, KISTI 6호기 탐색연구)가 연구실 자체 서버 관리 수요를 늘리는가, 줄이는가?
3. 국내 대학 연구비로 소규모 소프트웨어를 살 수 있는가? 산다면 연구실, 학과, 대학 중 누구 예산이며 절차는 무엇인가? 노트는 확인 못함으로 남겼다.
4. Rust NVML 바인딩(nvml-wrapper)이 지금 쓸 만한 상태인가? Apple Silicon GPU 지표를 루트 권한 없이 읽을 수 있는가?
5. 팀과 A는 (b)가 판정을 통과할 경우 규칙 1번을 바꿔 컨테이너 실행을 받아들일 의사가 있는가? 그렇다면 사용자별 인증과 감사 로그를 누가 설계하는가?
6. Metal과 CUDA/OptiX 출력의 실제 픽셀 차이는 얼마이며, 프레임 단위 분배에서 깜빡임으로 보이는가? 공개 자료가 없다.
7. CrowdRender의 등급별 노드 제한, 현재 라이선스, Blender 4.4 이후 지원과 2025~2026 릴리스 기록은 어떤가?
8. Blender 사용자 중 성능 좋은 기기를 2대 이상 가진 비율, r/blender와 r/LocalLLaMA의 최근 불만 유형과 빈도는 어떤가?
9. Run:ai 교육 할인, ClearML과 Backend.AI 학술 라이선스의 공식 가격은 얼마인가? 이번 비교의 상용 가격은 모두 2차 출처다.
10. macOS·Windows 설치 파일 서명 비용과 공증 절차는 얼마나 걸리는가? (a)와 (c)처럼 데스크톱에 배포하는 후보라면 피할 수 없는 비용이다.
