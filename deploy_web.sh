#!/bin/bash
set -e

set -a && source .env && set +a

echo "▶ Cleaning old web build..."
rm -rf target/dx/photo_exchanger/release/web/public

echo "▶ Building web (release)..."
dx build --platform web --release

cp web/.htaccess target/dx/photo_exchanger/release/web/public/.htaccess 2>/dev/null || true

python3 -c '
path = "target/dx/photo_exchanger/release/web/public/index.html"
try:
    with open(path, "r", encoding="utf-8") as f:
        html = f.read()
    patch = """<script>
(function() {
  if (window.__peImeGuard) return;
  window.__peImeGuard = true;
  var isComposing = false;
  document.addEventListener("compositionstart", function(e) { isComposing = true; if (e.target) e.target.__isComposing = true; }, true);
  document.addEventListener("compositionend", function(e) { isComposing = false; if (e.target) e.target.__isComposing = false; }, true);
  function getPropDesc(obj, prop) { var cur = obj; while (cur) { var d = Object.getOwnPropertyDescriptor(cur, prop); if (d) return d; cur = Object.getPrototypeOf(cur); } return null; }
  function patchProto(proto) {
    var desc = getPropDesc(proto, "value");
    if (!desc || !desc.set || !desc.get) return;
    var origSet = desc.set, origGet = desc.get;
    Object.defineProperty(proto, "value", {
      get: function() { return origGet.call(this); },
      set: function(val) {
        if (this === document.activeElement) {
          if (val !== "" && (isComposing || this.__isComposing)) return;
          if (origGet.call(this) === val) return;
        }
        origSet.call(this, val);
      },
      configurable: true, enumerable: true
    });
  }
  patchProto(HTMLInputElement.prototype);
  patchProto(HTMLTextAreaElement.prototype);
})();
</script></head>"""
    if "</head>" in html and "__peImeGuard" not in html:
        html = html.replace("</head>", patch, 1)
        with open(path, "w", encoding="utf-8") as f:
            f.write(html)
        print("  ✓ Injected IME guard to index.html")
except Exception as e:
    print("  ! Index.html patch notice:", e)
'

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
