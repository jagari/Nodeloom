# Nodeloom 베타 사용 안내

팀 내부 베타(`v0.1.0-beta.2`)다. 기기 한 대가 클러스터를 운영하고(마스터), 나머지 기기가 거기에 참여한다(에이전트). 변경 내용은 [`CHANGELOG.md`](../CHANGELOG.md)에 있다.

- **PC(맥·윈도우·리눅스)**: 데스크톱 앱을 설치한다. 터미널을 쓰지 않는다.
- **화면 없는 기기(라즈베리파이·Jetson·서버)**: 명령줄 버전을 설치한다(아래 "명령줄 버전").

두 방식은 같은 클러스터에 섞어 쓸 수 있다.

## 1. 받기

저장소가 비공개라 저장소 접근 권한이 있는 GitHub 계정이 필요하다. 저장소의 Releases에서 `v0.1.0-beta.2`를 열고, 기기에 맞는 파일을 받는다.

| 기기 | 데스크톱 앱 | 명령줄 버전 |
|---|---|---|
| macOS (Apple Silicon) | `nodeloom-desktop-0.1.0-beta.2-macos-arm64.dmg` | `nodeloom-0.1.0-beta.2-macos-arm64.tar.gz` |
| Windows (대부분의 PC) | `nodeloom-desktop-0.1.0-beta.2-windows-x64.exe` | `nodeloom-0.1.0-beta.2-windows-x64.zip` |
| Windows on ARM (Surface Pro X·Copilot+ PC 등) | `nodeloom-desktop-0.1.0-beta.2-windows-arm64.exe` | `nodeloom-0.1.0-beta.2-windows-arm64.zip` |
| Linux PC (Ubuntu 22.04 이상, Debian 12 이상) | `nodeloom-desktop-0.1.0-beta.2-linux-x64.deb` | `nodeloom-0.1.0-beta.2-linux-x64.tar.gz` |
| 라즈베리파이 4·5(64비트 OS), Jetson | (`linux-arm64.deb`, 화면이 있을 때) | `nodeloom-0.1.0-beta.2-linux-arm64.tar.gz` |

Intel Mac과 32비트 라즈베리파이 OS용 빌드는 아직 없다.

## 2. 데스크톱 앱 설치

베타는 코드 서명을 하지 않아서 처음 열 때 경고가 나온다. 아래 방법으로 연다.

**macOS**

1. `.dmg`를 열고 Nodeloom을 응용 프로그램 폴더로 끈다.
2. Nodeloom을 연다. "Apple은 … 확인할 수 없습니다" 창이 뜨면 **완료**를 누른다.
3. 시스템 설정 → 개인정보 보호 및 보안 → 아래쪽의 **그래도 열기**를 누른다.
   - 터미널을 쓸 수 있다면 대신 `xattr -dr com.apple.quarantine /Applications/Nodeloom.app`을 실행한다.
4. "로컬 네트워크의 기기를 찾도록 허용" 창이 뜨면 **허용**한다. 허용해야 기기들이 서로 찾는다.

**Windows**

1. `.exe`를 실행한다. "Windows의 PC 보호" 창이 뜨면 **추가 정보 → 실행**을 누른다.
2. 관리자 권한 없이 사용자 계정에 설치된다. 시작 메뉴에 Nodeloom이 생긴다.
3. 처음 실행할 때 방화벽 창이 뜨면 **개인 네트워크**를 허용한다.

**Linux**

```sh
sudo apt install ./nodeloom-desktop-0.1.0-beta.2-linux-x64.deb
```

앱 목록에서 Nodeloom을 연다. 트레이 아이콘은 Ubuntu 기본 데스크톱에서 보인다. 다른 GNOME 환경은 AppIndicator 확장이 필요할 수 있다.

## 3. 처음 실행

처음 열면 이 기기의 역할을 고른다.

- **Start a cluster here**: 이 기기가 클러스터를 운영한다. 항상 켜 두기 좋은 기기 **한 대**에서 고른다.
  - 앱 창이 바로 대시보드가 된다. 이 기기도 작업에 참여하며, 승인 없이 자동으로 붙는다.
  - 다른 기기(노트북, 휴대폰)의 브라우저에서 볼 주소는 트레이 메뉴 → **Settings**에 나온다.
- **Join a cluster**: 나머지 기기에서 고른다.
  - 같은 네트워크의 클러스터를 스스로 찾는다. 찾으면 화면에 6자리 확인 코드가 크게 나온다.
  - 클러스터를 운영하는 기기의 대시보드에서 **Waiting for approval** 카드를 본다. 코드가 같으면 **Approve**를 누른다. 한 번 승인하면 다시 켜도 자동으로 붙는다.

"Start Nodeloom when I log in"을 켜 두면 로그인할 때 트레이에서 조용히 시작한다.

창을 닫아도 트레이에서 계속 돈다. 완전히 끄려면 트레이 메뉴 → **Quit Nodeloom**을 누른다. 역할을 바꾸려면 Settings → **Change role…**을 누른다. 승인된 기기 목록과 이 기기의 신원은 그대로 남는다.

## 4. 작업 보내 보기

대시보드의 **Start a job**에서 Runner를 "Test (sleep only)"로 두고 **Split across machines**를 누른다.

- 작업이 연결된 기기 수만큼 나뉘어 처리되고, 진행률이 Jobs에 보인다.
- 처리 중에 기기 하나를 끄면 그 부분이 다른 기기로 넘어간다.

베타에는 아직 실제 작업 플러그인이 없다. 데스크톱 앱은 테스트 작업만 받는다.

## 5. 기기 빼기

대시보드 **Paired machines**에서 **Remove**를 누르면 그 기기는 즉시 끊기고 다시 들어오지 못한다. 다시 붙이려면 다음과 같이 한다.

- 데스크톱 앱: 그 기기의 앱 화면에서 **Pair as a new machine**을 누른다.
- 명령줄 버전: `nodeloom-agent reset-identity`를 실행한다.

그러면 새 코드가 나오고, 대시보드에서 다시 승인하면 된다.

## 명령줄 버전

화면 없는 기기나 서버용이다. 데스크톱 앱과 같은 데이터 폴더를 쓴다. 한 기기에서 둘을 동시에 실행하지는 않는다.

**설치**

macOS·Linux:

```sh
tar -xzf nodeloom-0.1.0-beta.2-<기기>.tar.gz
cd nodeloom-0.1.0-beta.2-<기기>
./install.sh
```

- `~/.local/bin`에 설치한다. 그 폴더가 PATH에 없으면 스크립트가 추가 방법을 알려 준다.
- `gh auth login`을 마친 기기라면 저장소의 `scripts/install.sh`만 받아 `sh install.sh`로 실행해도 된다. 알맞은 파일을 받아 체크섬까지 확인한다.

Windows (PowerShell):

```powershell
Expand-Archive nodeloom-0.1.0-beta.2-windows-x64.zip
cd nodeloom-0.1.0-beta.2-windows-x64\nodeloom-0.1.0-beta.2-windows-x64
powershell -ExecutionPolicy Bypass -File install.ps1
```

- `%LOCALAPPDATA%\Nodeloom\bin`에 설치하고 사용자 PATH에 추가한다.

**실행**

- 클러스터를 운영할 기기: `nodeloom-master`를 실행한다.
  - 처음 실행하면 `open the dashboard: http://192.168.0.12:7878/#token=…` 줄이 나온다. 이 주소를 브라우저로 열면 로그인까지 끝난다.
  - 두 번째부터는 토큰을 다시 출력하지 않는다. 다른 브라우저에서는 `http://<마스터 IP>:7878`을 열고, 데이터 폴더의 `admin-token` 파일 값을 넣는다.
- 참여할 기기: `nodeloom-agent --enable-fake-runner`를 실행한다.
  - 마스터를 자동으로 찾는다. 못 찾으면 `--master 192.168.0.12:7878`로 주소를 준다.
  - 출력된 6자리 코드를 대시보드와 대조해 승인한다. 코드는 `nodeloom-agent pairing-code`로 다시 볼 수 있다.

명령줄 버전은 부팅할 때 자동으로 시작하지 않는다(#9).

## 문제 해결

| 증상 | 할 일 |
|---|---|
| 앱이 "Looking for a master…"에서 멈춤 | 클러스터를 운영하는 기기에서 Nodeloom이 켜져 있는지 본다. 다른 서브넷·VPN·게스트 와이파이에서는 자동 탐색이 안 된다. 그런 경우에는 명령줄 에이전트의 `--master <IP>:7878`을 쓴다 |
| 앱이 "port 7878 is already in use"로 시작하지 못함 | 같은 기기에서 명령줄 `nodeloom-master`가 돌고 있다. 그것을 끄거나, 앱의 역할을 "Join a cluster"로 바꾼다 |
| 대시보드가 "Wrong admin token" | 마스터 데이터 폴더의 `admin-token` 값을 넣는다. 잃어버렸으면 마스터를 멈추고 `nodeloom-master --rotate-admin-token`으로 새로 만든다 |
| 다른 기기에서 대시보드가 열리지 않음 | 마스터 기기의 방화벽이 7878 포트를 막지 않는지 확인한다 |
| macOS "손상되었기 때문에 열 수 없습니다" | `xattr -dr com.apple.quarantine /Applications/Nodeloom.app` |
| 기기가 "Removed from …"로 거부됨 | 대시보드에서 제거된 기기다. **Pair as a new machine**(앱) 또는 `nodeloom-agent reset-identity`(명령줄) 후 다시 승인한다 |

## 알려진 제한

- **통신이 암호화되지 않는다.** 믿을 수 있는 네트워크(집, 연구실)에서만 쓴다. 공용 와이파이에서는 쓰지 않는다.
- 실제 작업(Whisper 전사 등)과 작업 파일 전송은 아직 없다.
- GPU 사용률은 아직 보이지 않는다.
- 데스크톱 앱은 서명하지 않았고, 자동 업데이트가 없다. 새 버전은 다시 받아 설치한다.

## 피드백

GitHub Issues에 남긴다. 다음을 함께 적으면 빨리 고칠 수 있다.

- 기기와 OS 버전
- 실행한 명령(앱이라면 누른 버튼)
- 오류 메시지(토큰은 지운다)
- 화면 캡처

라즈베리파이·Jetson 결과는 #24, 맥은 #1, 윈도우는 #4, 데스크톱 앱 창은 #3에 남긴다.
