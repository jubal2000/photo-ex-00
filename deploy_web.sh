#!/bin/bash
set -e

set -a && source .env && set +a

echo "▶ Building web (release)..."
dx build --platform web --release

cp web/.htaccess target/dx/photo_exchanger/release/web/public/.htaccess 2>/dev/null || true

echo "▶ Uploading to FTP: ${FTP_HOST}/${FTP_REMOTE_DIR}/"
python3 - <<PYEOF
import ftplib, os, sys

HOST   = os.environ["FTP_HOST"]
USER   = os.environ["FTP_USER"]
PASS   = os.environ["FTP_PASS"]
REMOTE = os.environ["FTP_REMOTE_DIR"]   # e.g. www/photo-ex
LOCAL  = "target/dx/photo_exchanger/release/web/public"

def cd_or_mkd(ftp, path):
    """FTP 루트부터 path까지 디렉터리를 순서대로 이동/생성."""
    ftp.cwd("/")
    for part in path.strip("/").split("/"):
        try:
            ftp.cwd(part)
        except ftplib.error_perm:
            ftp.mkd(part)
            ftp.cwd(part)

total = 0

def upload_tree(ftp, local_root, remote_root):
    global total
    for root, dirs, files in os.walk(local_root):
        rel = os.path.relpath(root, local_root)
        if rel == ".":
            remote_dir = remote_root
        else:
            remote_dir = remote_root + "/" + rel.replace(os.sep, "/")

        cd_or_mkd(ftp, remote_dir)

        for f in files:
            with open(os.path.join(root, f), "rb") as fp:
                ftp.storbinary(f"STOR {f}", fp)
            total += 1
            print(f"  ✓ {remote_dir}/{f}")

try:
    ftp = ftplib.FTP()
    ftp.connect(HOST, 21, timeout=90)
    ftp.login(USER, PASS)
    ftp.set_pasv(True)
    print(f"  접속 성공: {HOST}")

    upload_tree(ftp, LOCAL, REMOTE)
    ftp.quit()
    print(f"\n✓ 완료: {total}개 파일 업로드")
except Exception as e:
    print(f"\n✗ 업로드 실패: {e}", file=sys.stderr)
    sys.exit(1)
PYEOF
