# Nodeloom 베타 사용 안내

팀 내부 베타(`v0.1.0-beta.1`)다. 기기 한 대에 마스터를, 나머지 기기에 에이전트를 설치하고, 브라우저로 대시보드를 열어 기기를 승인하고 작업을 나눠 보낸다. 변경 내용은 [`CHANGELOG.md`](../CHANGELOG.md)에 있다.

## 1. 받기

저장소가 비공개라 저장소 접근 권한이 있는 GitHub 계정이 필요하다.

- **릴리즈 페이지에서**: 저장소의 Releases에서 `v0.1.0-beta.1`을 열고, 기기에 맞는 파일을 받는다.

| 기기 | 파일 |
|---|---|
| macOS (Apple Silicon) | `nodeloom-0.1.0-beta.1-macos-arm64.tar.gz` |
| Windows (대부분의 PC) | `nodeloom-0.1.0-beta.1-windows-x64.zip` |
| Windows on ARM (Surface Pro X·Copilot+ PC 등) | `nodeloom-0.1.0-beta.1-windows-arm64.zip` |
| Linux PC | `nodeloom-0.1.0-beta.1-linux-x64.tar.gz` |
| 라즈베리파이 4·5(64비트 OS), Jetson | `nodeloom-0.1.0-beta.1-linux-arm64.tar.gz` |

- **GitHub CLI로**: `gh auth login`을 마친 기기라면 설치 스크립트가 알맞은 파일을 받아 체크섬까지 확인한다(아래 2번의 "압축 없이").

Intel Mac과 32비트 라즈베리파이 OS용 빌드는 아직 없다.

## 2. 설치

**macOS·Linux**

```sh
tar -xzf nodeloom-0.1.0-beta.1-<기기>.tar.gz
cd nodeloom-0.1.0-beta.1-<기기>
./install.sh
```

- `~/.local/bin`에 설치한다. 그 폴더가 PATH에 없으면 스크립트가 추가 방법을 알려 준다.
- 압축 없이: `gh`로 로그인된 기기에서 저장소의 `scripts/install.sh`를 받아 `sh install.sh`를 실행한다.

**Windows** (PowerShell)

```powershell
Expand-Archive nodeloom-0.1.0-beta.1-windows-x64.zip
cd nodeloom-0.1.0-beta.1-windows-x64\nodeloom-0.1.0-beta.1-windows-x64
powershell -ExecutionPolicy Bypass -File install.ps1
```

- `%LOCALAPPDATA%\Nodeloom\bin`에 설치하고 사용자 PATH에 추가한다. 새 터미널을 열어야 PATH가 반영된다.

설치 확인: `nodeloom-agent --version` → `nodeloom-agent 0.1.0-beta.1`

## 3. 마스터 시작 (기기 한 대)

항상 켜 두기 좋은 기기(데스크톱, 라즈베리파이 등)에서 실행한다.

```sh
nodeloom-master
```

처음 실행하면 다음 두 줄이 나온다.

```
admin token: 3f0c…  (saved to …/admin-token)
open the dashboard: http://192.168.0.12:7878/#token=3f0c…
```

- **두 번째 줄의 주소를 브라우저로 열면** 로그인까지 끝난다. 같은 네트워크의 다른 기기(노트북, 휴대폰)에서도 열린다.
- 두 번째부터는 토큰을 다시 출력하지 않는다. 다른 브라우저에서 열 때는 `http://<마스터 IP>:7878`을 열고, 저장된 `admin-token` 파일의 값을 "Admin token"에 넣는다. 파일 위치는 실행할 때 로그에 나온다.
- Windows는 처음 실행할 때 방화벽 허용 창이 뜬다. "개인 네트워크"를 허용한다.
- macOS는 "네트워크의 기기를 찾도록 허용" 창이 뜰 수 있다. 허용해야 에이전트가 마스터를 자동으로 찾는다.

## 4. 기기 추가

각 기기에서 실행한다(마스터 기기에서도 실행하면 그 기기도 작업에 참여한다).

```sh
nodeloom-agent --enable-fake-runner
```

- `--enable-fake-runner`는 테스트용 작업(잠깐 쉬기만 함)을 받게 한다. 베타에는 아직 실제 작업 플러그인이 없어서, 동작을 확인하려면 이 옵션을 켠다.
- 에이전트는 같은 네트워크의 마스터를 자동으로 찾는다. 못 찾으면 `--master 192.168.0.12:7878`처럼 주소를 준다.
- 에이전트가 6자리 확인 코드를 출력한다. 대시보드의 **Waiting for approval** 카드 코드가 같은지 보고 **Approve**를 누른다. 코드는 `nodeloom-agent pairing-code`로 다시 볼 수 있다.
- 한 번 승인하면 그 기기는 다시 켤 때 자동으로 붙는다.

## 5. 작업 보내 보기

대시보드의 **Start a job**에서 Runner를 "Test (sleep only)"로 두고 **Split across machines**를 누른다. 작업이 연결된 기기 수만큼 나뉘어 처리되고, 진행률이 Jobs에 보인다. 처리 중에 에이전트 하나를 끄면 그 부분이 다른 기기로 넘어간다.

## 6. 기기 빼기

대시보드 **Paired machines**에서 **Remove**를 누르면 그 기기는 즉시 끊기고 다시 들어오지 못한다. 다시 붙이려면 그 기기에서 `nodeloom-agent reset-identity`를 실행한 뒤 새 코드로 승인한다.

## 문제 해결

| 증상 | 할 일 |
|---|---|
| 에이전트가 "looking for a master…" 뒤 실패 | 다른 서브넷·VPN·게스트 와이파이에서는 자동 탐색이 안 된다. `--master <마스터 IP>:7878`을 준다 |
| 대시보드가 "Wrong admin token" | 마스터 데이터 폴더의 `admin-token` 값을 넣는다. 잃어버렸으면 마스터를 멈추고 `nodeloom-master --rotate-admin-token`으로 새로 만든다 |
| 대시보드가 열리지 않음 | 마스터가 실행 중인지, 방화벽이 7878 포트를 막지 않는지 확인한다 |
| macOS "확인되지 않은 개발자" | 설치 스크립트로 설치했다면 나오지 않는다. 직접 복사했다면 `xattr -d com.apple.quarantine <파일>` |
| Windows SmartScreen 경고 | `install.ps1`로 설치한다. 직접 실행했다면 "추가 정보 → 실행" |
| 에이전트가 "removed from the cluster"로 거부됨 | 대시보드에서 제거된 기기다. `nodeloom-agent reset-identity` 후 다시 승인 |

## 알려진 제한

- **통신이 암호화되지 않는다.** 믿을 수 있는 네트워크(집, 연구실)에서만 쓴다. 공용 와이파이에서는 쓰지 않는다.
- 부팅할 때 자동으로 시작하지 않는다. 터미널을 닫으면 멈춘다(#9에서 해결 예정).
- 실제 작업(Whisper 전사 등)과 작업 파일 전송은 아직 없다.
- GPU 사용률은 아직 보이지 않는다.

## 피드백

GitHub Issues에 남긴다. 다음을 함께 적으면 빨리 고칠 수 있다.

- 기기와 OS 버전
- 실행한 명령
- 터미널에 나온 오류(토큰은 지운다)
- 대시보드 화면 캡처

라즈베리파이·Jetson 결과는 #24, 맥은 #1, 윈도우는 #4에 남긴다.
