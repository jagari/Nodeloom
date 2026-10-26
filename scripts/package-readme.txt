Nodeloom 베타
==============

학생·소규모 팀의 맥·윈도우·리눅스·라즈베리파이·Jetson을 한 풀로 묶어,
파일 묶음 작업을 기기들에 나눠 처리하는 도구의 팀 내부 베타입니다.

설치
  macOS·Linux:  ./install.sh
  Windows:      powershell -ExecutionPolicy Bypass -File install.ps1
  (~/.local/bin 또는 %LOCALAPPDATA%\Nodeloom\bin 에 설치됩니다)

사용
  1. 기기 한 대에서:   nodeloom-master
     처음 실행할 때 출력되는 "open the dashboard: http://…#token=…"
     주소를 브라우저로 엽니다. 같은 네트워크의 다른 기기에서도 열 수 있습니다.
  2. 각 기기에서:      nodeloom-agent
     대시보드의 "Waiting for approval" 카드 코드가 그 기기 화면의
     코드와 같은지 확인하고 Approve를 누릅니다.
  3. "Start a job"에서 테스트 작업을 보내 봅니다
     (테스트 작업은 에이전트를 --enable-fake-runner 로 실행해야 받습니다).

주의
  - 베타입니다. 통신이 암호화되지 않으니 믿을 수 있는 네트워크에서만 쓰세요.
  - 서명하지 않은 프로그램이라 처음 실행할 때 운영체제 경고가 나올 수 있습니다.
  - 자세한 안내와 문제 해결: 저장소의 docs/BETA.md
