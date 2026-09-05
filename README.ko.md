# AI Switcher (한국어)

<p align="center">
  <strong>OpenAI Codex, Anthropic Claude, Google Antigravity를 위한 Windows 멀티 계정 & 작업 영역 관리자</strong>
</p>

<p align="center">
  <a href="README.md"><strong>English</strong></a> |
  <a href="README.ko.md"><strong>한국어</strong></a> |
  <a href="README.ja.md"><strong>日本語</strong></a> |
  <a href="README.zh.md"><strong>简体中文</strong></a>
</p>

<p align="center">
  <img src="https://img.shields.io/badge/Release-v0.2.0-blue.svg" alt="Release v0.2.0" />
  <img src="https://img.shields.io/badge/Platform-Windows%2010%2F11-0078D6.svg" alt="Windows 10/11" />
  <img src="https://img.shields.io/badge/Framework-Tauri%202-FFC131.svg" alt="Tauri 2" />
  <img src="https://img.shields.io/badge/Rust-Backend-dea584.svg" alt="Rust" />
  <img src="https://img.shields.io/badge/React-Frontend-61DAFB.svg" alt="React" />
</p>

---

## 📖 개요

**AI Switcher**는 여러 조직, 회사, 개인 계정으로 AI 코딩 환경을 넘나들며 작업하는 개발자를 위해 제작된 일상용 Windows 유틸리티입니다. **OpenAI Codex**, **Anthropic Claude**, **Google Antigravity**를 완벽하게 지원합니다.

**데스크톱 우선(Desktop-First)** 방식으로 설계되어, 계정 전환 시 발생하는 세션 충돌이나 인증 정보 덮어쓰기, 복잡한 디렉터리 수동 변경 문제를 완벽히 해결하며 시스템 트레이 또는 단축키를 통해 1클릭으로 격리된 작업 환경을 즉시 실행합니다.

![AI Switcher 메인 대시보드](docs/images/dashboard.png)

---

## ✨ 핵심 기능

### 🖥️ 데스크톱 우선 독립 격리 실행
- **Google Antigravity Desktop:** 독자적인 Chromium 및 `.gemini` 사용자 디렉터리를 구성하여 `--user-data-dir`, `USERPROFILE`, `APPDATA`를 완벽히 격리합니다. 프로토콜 브로커를 통해 브라우저 OAuth 인증 콜백을 격리 인스턴스로 안전하게 중계합니다.
- **Anthropic Claude Desktop:** 분리된 `--user-data-dir` 프로필을 통해 네이티브 Claude Desktop 앱을 **Claude Code**(`claude://code/new?folder=...`) 세션으로 직접 실행하며, 세션 쿠키(`sessionKey`) 기반으로 인증 상태를 정밀 검증합니다.
- **OpenAI Codex CLI & Desktop:** 독립된 `CODEX_HOME` 프로필 디렉터리를 기반으로 CLI 환경(`[Codex CLI 열기]`)에서 완벽한 다중 계정 격리를 제공합니다. Codex 데스크톱은 Windows 공유 세션을 사용하므로 `Codex 데스크톱 열기 (공유 세션)`으로 명확히 안내됩니다.

### 📁 작업 영역 (Workspace) 프리셋
- 자주 작업하는 프로젝트 폴더와 선호하는 플랫폼별 계정을 프리셋으로 저장하여 1클릭(`Codex로 열기`, `Claude로 열기`, `Antigravity로 열기`)으로 즉시 실행할 수 있습니다.
- 폴더가 삭제되었거나 경로가 변경된 경우에도 사용자 프로젝트 파일을 보호하는 디렉터리 탐색 및 안전 생성 대화상자가 제공됩니다.

### 🌐 완벽한 다국어 지원
- **한국어**, **English**, **日本語**, **简体中文** 4개 언어 지원.
- 설정에서 클릭 한 번으로 앱 재시작 없이 즉시 전체 인터페이스 언어가 변경되며 SQLite에 영구 저장됩니다.

---

## 🖼️ 스크린샷 및 화면 구성

### 1. 메인 대시보드
플랫폼별로 등록된 계정 프로필 목록, 실행 상태 뱃지, 즐겨찾기 고정, 1클릭 실행 버튼을 한눈에 관리할 수 있습니다.

![메인 대시보드](docs/images/dashboard.png)

---

### 2. 일반 설정 & 시스템 트레이 통합
Windows 부팅 시 자동 실행, 트레이로 최소화된 상태로 시작, 닫기(`X`) 버튼 클릭 시 트레이로 최소화 등 편리한 윈도우 유틸리티 옵션을 제공합니다.

![일반 설정](docs/images/settings-general.png)

---

### 3. 테마 및 화면 스타일
주간 고대비 라이트 모드, 눈의 피로를 줄여주는 다크 모드, Windows 설정에 자동 동기화되는 시스템 기본값을 지원합니다. 모든 알림 및 경고 박스는 WCAG AAA 고대비 기준(7:1 이상)을 충족합니다.

![테마 및 화면 설정](docs/images/settings-appearance.png)

---

### 4. 언어 설정
**한국어**, **English**, **日本語**, **简体中文** 중 원하는 언어를 선택할 수 있습니다.

![언어 설정](docs/images/settings-language.png)

---

### 5. 전역 단축키 (Global Hotkey)
Windows 작업 중 언제 어디서나 단축키(기본값: `CommandOrControl+Alt+S`)를 눌러 AI Switcher 창을 즉시 불러오거나 트레이로 숨길 수 있습니다.

![단축키 설정](docs/images/settings-shortcuts.png)

---

## 🛡️ 안정성 및 보안 보증

- **100% 로컬 독립 보관:** 모든 계정 프로필, 세션 토큰, 설정값은 로컬 PC의 독립 디렉터리(`%APPDATA%\AI-Switcher`)에만 저장됩니다.
- **인증 토큰 외부 전송 절대 없음:** AI Switcher는 사용자의 로그인 비밀번호, API 키, OAuth 토큰을 외부 서버로 전송하거나 가로채지 않습니다.
- **윈도우 정션/심볼릭 링크 보호:** 계정 삭제 시 Win32 Reparse Point 메타데이터를 확인하여 외부 문서나 프로젝트 원본 파일이 함께 삭제되지 않도록 디렉터리 연결 링크만 안전하게 해제합니다.
- **자동 SQLite 데이터베이스 백업:** `VACUUM INTO` 스냅샷 백업 기능 및 5개 버전 자동 유지 관리, 무결성 검증(`PRAGMA integrity_check`)을 내장했습니다.
- **익명화 지원 번들:** 문제 발생 시 비밀번호나 토큰 정보가 일절 포함되지 않도록 마스킹 처리된 진단 번들을 안전하게 내보낼 수 있습니다.

---

## 📦 다운로드 및 설치 방법

컴파일된 윈도우 설치 프로그램은 [Releases 페이지](https://github.com/TaeyanG4/ai-switcher/releases/tag/v0.2.0)에서 다운로드할 수 있습니다.

1. **`AI Switcher_0.2.0_x64-setup.exe`** 다운로드.
2. 설치 프로그램 실행 (관리자 권한 없이 `%LOCALAPPDATA%\AI Switcher`에 안전하게 설치됩니다).
3. 시작 메뉴 또는 바탕화면에서 **AI Switcher** 실행.

---

## 🛠️ 개발 환경 구축 및 직접 빌드

### 필수 요구 사항
- Windows 10/11 64-bit
- [Node.js](https://nodejs.org/) (v18 이상) & [pnpm](https://pnpm.io/)
- [Rust](https://www.rust-lang.org/) (MSVC 툴체인)

### 설치 및 로컬 실행
```powershell
# 저장소 클론
git clone https://github.com/TaeyanG4/ai-switcher.git
cd ai-switcher

# 프론트엔드 패키지 설치
pnpm install

# 백엔드 테스트 슈트 실행 (88개 테스트 통과 확인)
cargo test --manifest-path src-tauri/Cargo.toml

# 개발 모드로 앱 실행
pnpm tauri dev
```

### 정식 릴리스 빌드
```powershell
# 프론트엔드 컴파일 및 윈도우 인스톨러 생성
pnpm tauri build
```
빌드된 인스톨러 파일은 `src-tauri/target/release/bundle/nsis/`에 생성됩니다.

---

## 📄 라이선스

MIT License. 자세한 내용은 [LICENSE](LICENSE)를 참고하세요.
