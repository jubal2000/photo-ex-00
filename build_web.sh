#!/bin/bash
set -e

set -a && source .env && set +a

echo "▶ Building web (release)..."
dx build --platform web --release

cp web/.htaccess target/dx/photo_exchanger/release/web/public/.htaccess 2>/dev/null || true

echo ""
echo "✓ 빌드 완료"
echo "  업로드 폴더: target/dx/photo_exchanger/release/web/public/"
echo ""
echo "  이 폴더의 내용물 전체를 웹 서버 루트에 업로드하세요."
echo "  (index.html 이 루트에 위치해야 합니다)"
