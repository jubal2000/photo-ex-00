# PhotoCard Exchanger (포토카드 교환) Project Guidelines

## 1. 개요 및 핵심 원칙
- **플랫폼**: Rust + Dioxus 0.7 기반 Web, iOS, Android 크로스 플랫폼.
- **백엔드**: Firebase Firestore, Firebase Storage, Firebase Auth, Cloud Functions.
- **UI/UX 원칙**:
  - **이모지(Emoji) 사용 엄격 금지**: 앱 내의 모든 화면, 컴포넌트, 텍스트, 버튼, 알림 메시지, 플레이스홀더에서 일체의 이모지(Unicode Emoji) 사용을 금지합니다.
  - **심플 & 미니멀 디자인**: 이모지 대신 명확한 텍스트 라벨, 간결한 타이포그래피, 세련된 CSS 뱃지/테두리/배경색, 기능적 기호(단순 닫기 ×, 텍스트 화살표 → 등)를 사용하여 전문적이고 통일된 룩앤필을 유지합니다.
  - **직관적인 피드백**: 상태 변화(성공, 로딩, 오류)는 텍스트 뱃지, 프로그레스 바, CSS 스피너를 활용합니다.

## 2. 디자인 및 컴포넌트 규칙
- **버튼**:
  - 이모지 접두사 없이 명확한 동작 동사 사용 (예: "저장", "교환 신청", "자동 1:1 매칭", "미리보기", "수정", "삭제").
- **플레이스홀더 / 빈 상태 (Empty States)**:
  - 이모지(🃏, 👤, 📁, 📬 등) 대신 세련된 회색 톤 플레이스홀더 박스, 아바타는 첫 글자 이니셜 또는 모노톤 실루엣, 텍스트 설명으로 표시.
- **배지 및 상태 표시**:
  - 색상 칩과 텍스트를 조합 (예: `pill-unowned`, `badge-inbox`, `badge-outbox`, `status-pill new`).

## 3. 다국어 (i18n)
- `src/i18n.rs`의 모든 언어(한국어, 영어, 일본어, 중국어, 스페인어) 번역 문자열에 이모지를 포함하지 않습니다.

## 4. 빌드 및 배포
- 코드 수정 후에는 항상 컴파일 오류 검사를 수행합니다:
  - `cargo check`
  - `cargo check --target aarch64-apple-ios`
- 웹 배포: `./deploy_web.sh` (Cafe24 호스팅)
- iOS 배포: `./deploy_ios.sh` (Xinno Test iPhone 실기기 연결)

