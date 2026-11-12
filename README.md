# 포토카드 교환 (PhotoCard Exchange)

Rust + [Dioxus 0.7](https://dioxuslabs.com) 단일 코드베이스로 **Web / iOS / Android**에서 동작하는 포토카드 교환 앱입니다.
DB는 Firestore, 인증은 Firebase Auth(REST)를 사용합니다.

## 구조

```
src/
  main.rs, app.rs        앱 셸 (헤더 · 화면 스택 · 하단 광고 · 햄버거 메뉴)
  state.rs               전역 상태, 화면 스택(GNB 없음), 자동 로그인 토큰 관리
  api.rs                 앱 데이터 로직 (카탈로그 / 보유카드 / 교환)
  models.rs              Firestore 데이터 모델 (주석에 컬렉션 구조)
  i18n.rs                한국어 · 영어 · 일본어 · 중국어 · 스페인어
  firebase/              Firestore · Auth · Storage · Functions REST 클라이언트
  platform/              저장소(Keychain 등) · 생체인증 · 브라우저 열기
  screens/               login, home(+카테고리 오퍼), trade, search, profile, holdings, create, settings
firebase/
  firestore.rules        보안 규칙 (공용 카탈로그, 본인 보유카드, 교환 상태 전이)
  firestore.indexes.json 컬렉션 그룹 인덱스
  functions/index.js     authSession · kakaoToken · acceptTrade
  hosting/auth.html      Google / Apple / Kakao 로그인 페이지 (시스템 브라우저에서 열림)
  scripts/seed.js        에뮬레이터용 데모 데이터
```

### 로그인 흐름
앱 → 시스템 브라우저로 `auth.html?provider=..&session=<랜덤>` 열기 → 페이지에서 OAuth 로그인
→ `authSession` 함수에 refresh token 저장(1회용, 5분 TTL) → 앱이 폴링해서 수령 → Firebase 세션.
Google은 WebView 내 OAuth를 막기 때문에 이 방식을 쓰며, 세 플랫폼이 동일하게 동작합니다.
Kakao는 Firebase 기본 제공자가 아니므로 `kakaoToken` 함수가 Firebase 커스텀 토큰을 발급합니다.
처음 로그인하면 `users/{uid}` 프로필이 생성됩니다(회원가입).

### 생체 인증
- **iOS**: `LAContext`(Face ID / Touch ID)를 사용합니다.
- **Android**: 프레임워크 `BiometricPrompt`(API 28+, BIOMETRIC_STRONG)를 사용합니다. 콜백이 Java 추상 클래스라서 순수 JNI로는 구현할 수 없습니다.
  그래서 [`android/bio/BioAuth.java`](android/bio/src/com/photoexchanger/bio/BioAuth.java)를 미리 `classes.dex`로 컴파일해서 Rust 바이너리에 `include_bytes!`로 넣고, 실행 중에 `InMemoryDexClassLoader`로 불러옵니다.
  dx가 매번 다시 만드는 Gradle 프로젝트는 수정할 필요가 없습니다. Java 코드를 고쳤다면 `android/bio/build_dex.sh`를 다시 실행하세요.
- **권한**: `Dioxus.toml`의 `[permissions] biometrics`가 Android `USE_BIOMETRIC`과 iOS `NSFaceIDUsageDescription`을 넣어 줍니다.
- **웹**: 지원하지 않아서 설정에 "이 기기에서는 사용할 수 없어요"로 표시됩니다.
- **흐름**: 설정에서 켤 때 한 번 인증합니다. 그다음 앱을 실행하면 자동 로그인 대신 생체 인증 창이 먼저 뜨고, 성공하면 저장된 토큰으로 로그인합니다.

## 로컬 실행 (Firebase 에뮬레이터)

```bash
cd firebase/functions && npm install && cd ../..
cd firebase && firebase emulators:start --project demo-photo-exchanger
```

```bash
node firebase/scripts/seed.js
```

```bash
dx serve --platform web
```

```bash
dx serve --platform ios
```

```bash
ANDROID_HOME=~/Library/Android/sdk ANDROID_NDK_HOME=~/Library/Android/sdk/ndk/28.2.13676358 dx serve --platform android
```

에뮬레이터 모드(`USE_EMULATOR` 기본값 1)에서는 로그인 화면에 **게스트로 체험(개발용)** 버튼이 나옵니다.
Android 에뮬레이터는 `10.0.2.2`로 호스트에 접속합니다.

## 운영 설정

1. Firebase 프로젝트 생성 → Authentication에서 **Google, Apple** 제공자 활성화, Firestore · Storage 생성.
2. `firebase/.firebaserc`의 프로젝트 ID, `firebase/hosting/config.js`의 웹 설정과 `useEmulator: false`, `kakaoRestApiKey` 입력.
3. Kakao Developers: 앱 생성 → 카카오 로그인 활성화 → Redirect URI에 `https://<project>.firebaseapp.com/auth.html` 등록.
   `firebase/functions/.env`에 `KAKAO_REST_API_KEY` (선택: `KAKAO_CLIENT_SECRET`).
4. Apple: Apple Developer에서 Services ID / Key를 만들어 Firebase Apple 제공자에 등록.
5. 배포:
   ```bash
   cd firebase && firebase deploy --project <project-id>
   ```
6. 앱 빌드 시 환경변수 지정 (`.env.example` 참고):
   ```bash
   USE_EMULATOR=0 FIREBASE_API_KEY=... FIREBASE_PROJECT_ID=... FIREBASE_STORAGE_BUCKET=... AUTH_PAGE_URL=https://<project>.firebaseapp.com/auth.html FUNCTIONS_BASE_URL=https://asia-northeast3-<project>.cloudfunctions.net dx bundle --platform web --release
   ```
7. 웹 광고: `ADSENSE_CLIENT`, `ADSENSE_SLOT` 지정 시 하단 영역에 AdSense가 표시됩니다.

## 현재 한계 / 후속 작업

- **모바일 광고(AdMob)**: 네이티브 SDK 연동이 필요해서 지금은 320×50 자리만 확보해 두었습니다. 웹은 AdSense가 동작합니다.
- **토큰 저장**: iOS는 Keychain, Android는 앱 전용 내부 저장소, 웹은 localStorage에 저장합니다. (Android Keystore 암호화는 후속 작업)
- **검색**: 공용 카탈로그 전체를 받아 클라이언트에서 필터링합니다. 카드가 수만 장 규모로 커지면 Algolia나 Typesense 같은 검색 서비스 연동이 필요합니다.
