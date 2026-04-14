# ymux

[English](./README.md) | **한국어** | [日本語](./README.ja.md)

Windows용 경량 tmux 스타일 터미널 멀티플렉서.

Tauri 2 (Rust) + WebView2 + xterm.js 로 만들어졌습니다. Windows에서 가볍고 빠르게
네이티브로 동작하면서, 레이아웃 저장, pane별 작업 디렉터리와 실행 명령,
여러 셸 선택 (cmd / PowerShell / pwsh / Git Bash / WSL), 그리고 각자 자신만의
레이아웃을 기억하는 번호 매겨진 워크스페이스를 제공합니다.

## 기능

- **저장되는 레이아웃**: 재귀적 가로 / 세로 분할. 각 pane은 자신의 셸, `cwd`,
  선택적 시작 명령을 기억합니다.
- **셸 자동 감지**: `cmd.exe`, Windows PowerShell, PowerShell 7 (`pwsh`),
  Git Bash, WSL 배포판을 시스템에서 찾아내서 선택 가능한 프로필로 노출합니다.
- **번호 매겨진 워크스페이스**: `Ctrl+1` .. `Ctrl+9` 로 워크스페이스 사이를
  전환합니다. 모든 워크스페이스는 자신만의 레이아웃을 저장합니다. Pane은
  전환 사이에도 살아있기 때문에 (tmux 스타일) REPL 과 tail 이 죽지 않습니다.
- **가벼움**: Tauri 바이너리 + WebView2. 인스톨러 목표 < 10 MB.

## 개발

요구사항: Rust (stable), Node 20+, pnpm (또는 npm).

```sh
pnpm install
pnpm tauri dev          # 개발 모드로 실행
pnpm tauri build        # Windows 인스톨러 빌드 (Windows에서 실행)
```

Windows가 아닌 호스트에서도 Rust 크레이트는 `cargo check` 가 깨끗하게 통과해서
Linux/macOS 에서 플랫폼 독립적인 로직을 작업할 수 있습니다. 하지만 전체
`tauri build` 와 엔드투엔드 PTY 검증은 반드시 Windows 에서 수행해야 합니다.

## 설정

`%APPDATA%\ymux\config.toml` 에 워크스페이스, 레이아웃, 캐시된 셸 프로필이
저장됩니다. 구조 변경이 있을 때마다 (디바운싱 적용) 그리고 앱 종료 시 다시
쓰여집니다.

## 키보드

| 단축키              | 동작                      |
|---------------------|---------------------------|
| `Ctrl+Shift+D`      | 가로 분할                 |
| `Ctrl+Shift+-`      | 세로 분할                 |
| `Ctrl+Shift+W`      | 포커스된 pane 닫기        |
| `Ctrl+Tab`          | pane 포커스 순환          |
| `Ctrl+1` .. `Ctrl+9`| 워크스페이스 전환         |
| `Ctrl+Shift+N`      | 새 워크스페이스           |

## 상태

초기 MVP. 로드맵은 `docs/` (예정) 를 참고하세요.

## 후원

ymux가 도움이 되셨다면 커피 한 잔 사주시면 감사하겠습니다. 프로젝트 유지에
큰 힘이 됩니다.

[![ko-fi](https://img.shields.io/badge/Ko--fi-후원하기-FF5E5B?logo=kofi&logoColor=white)](https://ko-fi.com/youngminkim)

<https://ko-fi.com/youngminkim>
