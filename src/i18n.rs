//! UI strings in Korean, English, Japanese, Chinese (Simplified) and Spanish.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Lang {
    Ko,
    En,
    Ja,
    Zh,
    Es,
}

impl Lang {
    pub const ALL: [Lang; 5] = [Lang::Ko, Lang::En, Lang::Ja, Lang::Zh, Lang::Es];

    pub fn code(&self) -> &'static str {
        match self {
            Lang::Ko => "ko",
            Lang::En => "en",
            Lang::Ja => "ja",
            Lang::Zh => "zh",
            Lang::Es => "es",
        }
    }
    pub fn from_code(c: &str) -> Lang {
        match c {
            "en" => Lang::En,
            "ja" => Lang::Ja,
            "zh" => Lang::Zh,
            "es" => Lang::Es,
            _ => Lang::Ko,
        }
    }
    pub fn native_name(&self) -> &'static str {
        match self {
            Lang::Ko => "한국어",
            Lang::En => "English",
            Lang::Ja => "日本語",
            Lang::Zh => "中文",
            Lang::Es => "Español",
        }
    }
    fn idx(&self) -> usize {
        *self as usize
    }
}

macro_rules! strings {
    ($($key:literal => [$ko:literal, $en:literal, $ja:literal, $zh:literal, $es:literal]),* $(,)?) => {
        pub fn t(lang: Lang, key: &str) -> &'static str {
            let row: [&'static str; 5] = match key {
                $($key => [$ko, $en, $ja, $zh, $es],)*
                _ => return "???",
            };
            row[lang.idx()]
        }
        #[cfg(test)]
        pub const KEYS: &[&str] = &[$($key),*];
    };
}

strings! {
    "app_name" => ["PhotoEx", "PhotoEx", "PhotoEx", "PhotoEx", "PhotoEx"],
    "login_title" => ["포토카드를 교환해 보세요", "Trade your photocards", "フォトカードを交換しよう", "来交换小卡吧", "Intercambia tus photocards"],
    "login_google" => ["Google로 계속하기", "Continue with Google", "Googleで続ける", "使用 Google 继续", "Continuar con Google"],
    "login_apple" => ["Apple로 계속하기", "Continue with Apple", "Appleで続ける", "使用 Apple 继续", "Continuar con Apple"],
    "login_kakao" => ["카카오로 계속하기", "Continue with Kakao", "Kakaoで続ける", "使用 Kakao 继续", "Continuar con Kakao"],
    "login_guest" => ["게스트로 체험 (개발용)", "Try as guest (dev)", "ゲストで試す（開発用）", "以访客体验（开发）", "Probar como invitado (dev)"],
    "login_bio" => ["생체 인증으로 로그인", "Sign in with biometrics", "生体認証でログイン", "生物识别登录", "Iniciar con biometría"],
    "login_waiting" => ["브라우저에서 로그인을 완료해 주세요…", "Finish signing in in the browser…", "ブラウザでログインを完了してください…", "请在浏览器中完成登录…", "Completa el inicio de sesión en el navegador…"],
    "login_auto" => ["자동 로그인", "Stay signed in", "自動ログイン", "自动登录", "Mantener sesión"],
    "login_terms" => ["처음 로그인하면 자동으로 회원가입됩니다.", "Signing in for the first time creates your account.", "初回ログイン時に自動で会員登録されます。", "首次登录将自动注册。", "Al iniciar sesión por primera vez se crea tu cuenta."],
    "bio_reason" => ["포토카드 교환에 로그인", "Sign in to PhotoCard Exchange", "フォトカード交換にログイン", "登录小卡交换", "Iniciar sesión en Intercambio"],
    "bio_failed" => ["생체 인증에 실패했어요", "Biometric authentication failed", "生体認証に失敗しました", "生物识别失败", "Falló la autenticación biométrica"],
    "search" => ["검색", "Search", "検索", "搜索", "Buscar"],
    "menu_profile" => ["프로필", "Profile", "プロフィール", "个人资料", "Perfil"],
    "menu_trades" => ["교환 내역", "Trades", "交換履歴", "交换记录", "Intercambios"],
    "menu_holdings" => ["보유 카드", "My cards", "所持カード", "我的卡片", "Mis cartas"],
    "menu_create" => ["만들기", "Create", "作成", "创建", "Crear"],
    "menu_settings" => ["설정", "Settings", "設定", "设置", "Ajustes"],
    "menu_logout" => ["로그아웃", "Log out", "ログアウト", "退出登录", "Cerrar sesión"],
    "home_no_fav" => ["프로필에서 선호 아티스트를 선택해 주세요.", "Pick favourite artists in your profile.", "プロフィールで好きなアーティストを選んでください。", "请在个人资料中选择喜欢的艺人。", "Elige artistas favoritos en tu perfil."],
    "home_tradable" => ["교환 가능", "Tradable", "交換可能", "可交换", "Intercambiables"],
    "home_no_category" => ["아직 카테고리가 없어요", "No categories yet", "カテゴリーがまだありません", "暂无分类", "Aún no hay categorías"],
    "home_offers" => ["교환 가능한 유저", "Users open to trade", "交換可能なユーザー", "可交换的用户", "Usuarios para intercambiar"],
    "home_no_offers" => ["교환 가능한 카드가 없어요", "No tradable cards", "交換可能なカードはありません", "没有可交换的卡片", "No hay cartas intercambiables"],
    "request_trade" => ["교환 신청", "Request trade", "交換申請", "申请交换", "Solicitar intercambio"],
    "trade_title" => ["교환 신청", "Trade request", "交換申請", "交换申请", "Solicitud de intercambio"],
    "trade_theirs" => ["상대방 카드 (받을 카드)", "Their cards (you receive)", "相手のカード（受け取る）", "对方卡片（获得）", "Sus cartas (recibes)"],
    "trade_mine" => ["내 카드 (줄 카드)", "My cards (you give)", "自分のカード（渡す）", "我的卡片（给出）", "Mis cartas (das)"],
    "trade_need" => ["미보유", "Need", "未所持", "未拥有", "Me falta"],
    "trade_they_need" => ["상대 미보유", "They need", "相手未所持", "对方未拥有", "Le falta"],
    "trade_submit" => ["교환 신청하기", "Send request", "申請する", "发送申请", "Enviar solicitud"],
    "trade_sent" => ["교환 신청을 보냈어요", "Trade request sent", "交換申請を送りました", "已发送交换申请", "Solicitud enviada"],
    "trade_select_both" => ["양쪽에서 카드를 한 장 이상 선택해 주세요", "Select at least one card on each side", "両方から1枚以上選んでください", "请双方各至少选择一张", "Selecciona al menos una carta de cada lado"],
    "trade_summary" => ["받을 카드 {a}장 · 줄 카드 {b}장", "Receive {a} · Give {b}", "受け取る {a}枚・渡す {b}枚", "获得 {a} 张 · 给出 {b} 张", "Recibes {a} · Das {b}"],
    "trade_pair_rule" => ["교환 매칭", "Trade Matching", "交換マッチング", "交换配对", "Emparejamiento"],
    "trade_drag_hint" => ["카드를 드래그하거나 탭하여 서로 연결하세요", "Drag or tap cards to pair", "カードをドラッグまたはタップして繋いでください", "拖拽或点击卡片以配对", "Arrastra o toca cartas para emparejar"],
    "trade_preview" => ["교환 결과 미리보기", "Preview Trade", "交換結果プレビュー", "预览交换结果", "Vista previa"],
    "trade_paired" => ["연결됨", "Paired", "ペア完了", "已配对", "Emparejado"],
    "trade_unpaired" => ["연결되지 않음", "Not paired", "ペアなし", "未配对", "Sin emparejar"],
    "not_owned" => ["미보유", "Not owned", "未所持", "未拥有", "No poseído"],
    "not_tradable" => ["교환 불가", "Not tradable", "交換不可", "不可交换", "No intercambiable"],
    "trade_preview_title" => ["교환 후 보유 변화 미리보기", "Exchange Preview", "交換後の所持プレビュー", "交换后持有变化预览", "Vista previa post-intercambio"],
    "before_trade" => ["교환 전", "Before", "交換前", "交换前", "Antes"],
    "after_trade" => ["교환 후", "After", "交換後", "交换后", "Después"],
    "trade_give_card" => ["내가 건넬 카드", "You Give", "自分が渡すカード", "我方给出的卡片", "Tú das"],
    "trade_receive_card" => ["내가 받을 카드", "You Receive", "自分が受け取るカード", "我方获得的卡片", "Tú recibes"],
    "trade_no_pairs" => ["먼저 카드를 1:1로 매칭해 주세요", "Please pair at least 1 card first", "先にカードを1:1でペアリングしてください", "请先将卡片进行1:1配对", "Empareja al menos una carta primero"],
    "new_card" => ["새 카드!", "New!", "新規!", "新获得!", "¡Nueva!"],
    "trades_inbox" => ["받은 교환 신청", "Incoming trade requests", "受け取った交換申請", "收到的交换申请", "Solicitudes recibidas"],
    "trades_outbox" => ["보낸 교환 신청", "Sent trade requests", "送った交換申請", "发出的交换申请", "Solicitudes enviadas"],
    "trades_none" => ["교환 신청이 없어요", "No trade requests", "交換申請はありません", "暂无交换申请", "Sin solicitudes"],
    "accept" => ["수락", "Accept", "承認", "接受", "Aceptar"],
    "reject" => ["거절", "Reject", "拒否", "拒绝", "Rechazar"],
    "cancel" => ["취소", "Cancel", "キャンセル", "取消", "Cancelar"],
    "status_pending" => ["대기 중", "Pending", "保留中", "待处理", "Pendiente"],
    "status_accepted" => ["수락됨", "Accepted", "承認済み", "已接受", "Aceptada"],
    "status_rejected" => ["거절됨", "Rejected", "拒否", "已拒绝", "Rechazada"],
    "status_cancelled" => ["취소됨", "Cancelled", "キャンセル済み", "已取消", "Cancelada"],
    "status_completed" => ["교환 완료", "Completed", "交換完了", "已完成", "Completado"],
    "search_placeholder" => ["카드, 시리즈, 카테고리 검색", "Search cards, series, categories", "カード・シリーズ・カテゴリーを検索", "搜索卡片、系列、分类", "Buscar cartas, series, categorías"],
    "scope_all" => ["전체", "All", "すべて", "全部", "Todo"],
    "scope_owned" => ["보유", "Owned", "所持", "已拥有", "Mías"],
    "all_artists" => ["모든 아티스트", "All artists", "すべてのアーティスト", "全部艺人", "Todos los artistas"],
    "all_categories" => ["모든 카테고리", "All categories", "すべてのカテゴリー", "全部分类", "Todas las categorías"],
    "all" => ["전체", "All", "すべて", "全部", "Todos"],
    "no_cards" => ["등록된 카드가 없어요", "No cards registered", "登録されたカードがありません", "暂无卡片", "No hay cartas registradas"],
    "no_results" => ["검색 결과가 없어요", "No results", "結果がありません", "无结果", "Sin resultados"],
    "add_to_holdings" => ["보유 추가", "Add to my cards", "所持に追加", "加入我的卡片", "Añadir a mis cartas"],
    "added" => ["추가했어요", "Added", "追加しました", "已添加", "Añadido"],
    "profile_photo" => ["사진 변경", "Change photo", "写真を変更", "更换照片", "Cambiar foto"],
    "nickname" => ["닉네임", "Nickname", "ニックネーム", "昵称", "Apodo"],
    "greeting" => ["인사말", "Greeting", "ひとこと", "签名", "Saludo"],
    "fav_artists" => ["선호 아티스트", "Favourite artists", "好きなアーティスト", "喜欢的艺人", "Artistas favoritos"],
    "save" => ["저장", "Save", "保存", "保存", "Guardar"],
    "saved" => ["저장했어요", "Saved", "保存しました", "已保存", "Guardado"],
    "holdings_empty" => ["보유 카드가 없어요. 검색에서 카드를 추가해 보세요.", "No cards yet. Add some from search.", "所持カードがありません。検索から追加してください。", "还没有卡片，去搜索添加吧。", "Aún no tienes cartas. Añádelas desde la búsqueda."],
    "holdings_only" => ["보유 카드만", "Owned only", "所持のみ", "仅已拥有", "Solo mías"],
    "show_all_cards" => ["모두 보기 (미보유 포함)", "Show all (incl. unowned)", "すべて表示 (未所持含む)", "查看全部 (含未拥有)", "Ver todas (incl. no poseídas)"],
    "unowned" => ["미보유", "Unowned", "未所持", "未拥有", "No poseída"],
    "crop_image" => ["이미지 자르기", "Crop image", "画像を切り抜く", "裁剪图片", "Recortar imagen"],
    "crop_zoom" => ["확대 / 축소", "Zoom", "拡大 / 縮小", "缩放", "Zoom"],
    "crop_guide" => ["드래그로 이동 · 핀치/슬라이드로 확대 축소", "Drag to move · Pinch/Slide to zoom", "ドラッグで移動・ピンチ/スライドで拡大縮小", "拖动移动 · 双指/滑动缩放", "Arrastra para mover · Pellizca/Desliza para zoom"],
    "crop_apply" => ["자르기 및 적용", "Crop & Apply", "切り抜いて適用", "裁剪并应用", "Recortar y aplicar"],
    "apply_original" => ["그대로 적용", "Apply original", "そのまま適用", "直接应用", "Aplicar original"],
    "uploading_image" => ["이미지 업로드 중...", "Uploading image...", "画像アップロード中...", "正在上传图片...", "Subiendo imagen..."],
    "warn_all_tradable_title" => ["모든 카드 교환 가능 설정", "Make all cards tradable", "全カードを交換可能に設定", "全部设置为可交换", "Hacer todas intercambiables"],
    "warn_all_tradable_desc" => ["보유한 {count}장 모두를 교환 가능으로 설정하시겠습니까? 다른 사용자와 교환이 성사되면 보유 카드가 모두 소진될 수 있습니다.", "Are you sure you want to make all {count} card(s) tradable? If traded, you will have no remaining cards.", "所持している{count}枚すべてを交換可能にしますか？ 交換が成立すると所持カードがすべて無くなります。", "确定要将拥有的全部 {count} 张设置为可交换吗？如果交易成功，您将不再拥有该卡片。", "¿Estás seguro de hacer las {count} cartas intercambiables? Si se intercambian, te quedarás sin cartas."],
    "auto_match_btn" => ["자동 1:1 매칭", "Auto 1:1 Match", "自動1:1マッチング", "自动 1:1 匹配", "Emparejamiento 1:1"],
    "auto_match_success" => ["{count}개의 1:1 교환 매칭을 찾았어요!", "Found {count} 1:1 trade match(es)!", "{count}組の1:1マッチングが見つかりました！", "找到了 {count} 组 1:1 匹配！", "¡Se encontraron {count} emparejamientos 1:1!"],
    "auto_match_none" => ["비슷한 레어도의 매칭 가능한 교환 카드가 없어요.", "No matching cards found with similar rarity.", "似たレアリティのマッチング可能なカードがありません。", "未找到类似稀有度的可匹配卡片。", "No se encontraron cartas de rareza similar para intercambiar."],
    "protect_single_card" => ["1장 카드 보호", "Protect single cards", "1枚所持カード保護", "保护单张卡片", "Proteger cartas únicas"],
    "protect_single_card_desc" => ["1장만 보유 중인 카드는 교환 가능으로 설정되지 않도록 보호합니다.", "Cards with only 1 owned copy cannot be set as tradable.", "1枚のみ所持しているカードは交換可能に設定できなくなります。", "只有1张的卡片无法设置为可交换。", "Las cartas de las que solo tengas 1 copia no se podrán marcar para intercambio."],
    "single_card_protected" => ["1장 카드 보호 옵션이 켜져 있어 교환할 수 없어요.", "Single card protection is on. Cannot trade this card.", "1枚所持カード保護が有効なため交換できません。", "已开启单张卡片保护，无法交换此卡。", "La protección de carta única está activada. No se puede intercambiar."],
    "preview_owned_only" => ["보유만 보기", "Owned only", "所持のみ", "仅已拥有", "Solo poseídas"],
    "preview_all" => ["모두 보기", "Show all", "すべて表示", "查看全部", "Ver todas"],
    "preview_after_holdings" => ["교환 완료 후 보유 목록", "Holdings after trade", "交換完了後の所持リスト", "交换完成后的拥有列表", "Lista de cartas tras el intercambio"],
    "acquired_new" => ["신규 획득", "Newly acquired", "新規獲得", "新获得", "Recién adquirida"],
    "count_increased" => ["▲ 증가", "▲ Increased", "▲ 増加", "▲ 增加", "▲ Aumentado"],
    "count_decreased" => ["▼ 감소", "▼ Decreased", "▼ 減少", "▼ 减少", "▼ Disminuido"],
    "public" => ["공개", "Public", "公開", "公开", "Público"],
    "count" => ["보유", "Owned", "所持", "持有", "Tengo"],
    "tradable" => ["교환 가능", "Tradable", "交換可", "可交换", "Intercambio"],
    "remove" => ["삭제", "Remove", "削除", "删除", "Quitar"],
    "delete_card_confirm" => ["이 카드를 삭제하시겠습니까?", "Are you sure you want to delete this card?", "このカードを削除しますか？", "确定要删除此卡片吗？", "¿Seguro que quieres eliminar esta carta?"],
    "deleted" => ["카드를 삭제했어요", "Card deleted", "カードを削除しました", "已删除卡片", "Carta eliminada"],
    "create_artist" => ["새 아티스트", "New artist", "新しいアーティスト", "新艺人", "Nuevo artista"],
    "create_category" => ["새 카테고리", "New category", "新しいカテゴリー", "新分类", "Nueva categoría"],
    "create_series" => ["새 시리즈", "New series", "新しいシリーズ", "新系列", "Nueva serie"],
    "create_card" => ["새 카드", "New card", "新しいカード", "新卡片", "Nueva carta"],
    "name" => ["이름", "Name", "名前", "名称", "Nombre"],
    "artist" => ["아티스트", "Artist", "アーティスト", "艺人", "Artista"],
    "category" => ["카테고리", "Category", "カテゴリー", "分类", "Categoría"],
    "series" => ["시리즈", "Series", "シリーズ", "系列", "Serie"],
    "all_artists_common" => ["모든 아티스트 공통", "All artists (common)", "全アーティスト共通", "所有艺人通用", "Común a todos los artistas"],
    "common" => ["공통", "Common", "共通", "通用", "Común"],
    "edit_card" => ["카드 편집", "Edit card", "カード編集", "编辑卡片", "Editar carta"],
    "edit_artist" => ["아티스트 편집", "Edit artist", "アーティスト編集", "编辑艺人", "Editar artista"],
    "edit_category" => ["카테고리 편집", "Edit category", "カテゴリー編集", "编辑分类", "Editar categoría"],
    "edit_series" => ["시리즈 편집", "Edit series", "シリーズ編集", "编辑系列", "Editar serie"],
    "primary_color" => ["대표 색상", "Primary color", "代表カラー", "代表颜色", "Color principal"],
    "representative_image" => ["대표 이미지", "Main image", "代表画像", "代表图片", "Imagen principal"],
    "edit" => ["편집", "Edit", "編集", "编辑", "Editar"],
    "title_font" => ["타이틀 폰트", "Title font", "タイトルフォント", "标题字体", "Fuente del título"],
    "font_color" => ["폰트 색상", "Font color", "フォントカラー", "字体颜色", "Color de fuente"],
    "description" => ["설명", "Description", "説明", "描述", "Descripción"],
    "description_placeholder" => ["카드에 대한 설명을 적어주세요 (예: 앨범 버전, 한정판, 특징 등)", "Add a description (e.g. album version, special edition, details)", "カードの説明を入力してください（アルバムバージョン、限定版など）", "输入卡片描述（如专辑版本、限定版等）", "Añade una descripción (ej. versión de álbum, edición especial)"],
    "preview" => ["미리보기", "Preview", "プレビュー", "预览", "Vista previa"],
    "save_changes" => ["저장하기", "Save changes", "保存する", "保存修改", "Guardar cambios"],
    "edit_mode" => ["편집 모드", "Edit mode", "編集モード", "编辑模式", "Modo de edición"],
    "edit_mode_desc" => ["카드 추가 및 편집 버튼을 표시합니다", "Show card add and edit buttons", "カード追加・編集ボタンを表示します", "显示卡片添加与编辑按钮", "Muestra botones de agregar y editar cartas"],
    "rarity" => ["레어도", "Rarity", "レア度", "稀有度", "Rareza"],
    "font_default" => ["기본 (고딕)", "Default (Gothic)", "デフォルト（ゴシック）", "默认（无衬线）", "Por defecto"],
    "font_serif" => ["명조 (Serif)", "Serif", "明朝（明朝体）", "衬线体 (Serif)", "Serif"],
    "font_cursive" => ["손글씨 (Cursive)", "Cursive", "手書き風", "手写体 (Cursive)", "Cursiva"],
    "font_monospace" => ["고정폭 (Monospace)", "Monospace", "等幅（モノスペース）", "等宽字体", "Monoespaciado"],
    "font_fantasy" => ["임팩트 (Impact)", "Impact", "インパクト", "醒目黑体", "Impacto"],
    "select" => ["선택하세요", "Select…", "選択してください", "请选择", "Selecciona…"],
    "image" => ["이미지", "Image", "画像", "图片", "Imagen"],
    "image_url" => ["또는 이미지 URL", "or image URL", "または画像URL", "或图片链接", "o URL de imagen"],
    "create_btn" => ["만들기", "Create", "作成", "创建", "Crear"],
    "created" => ["공용 DB에 추가했어요", "Added to the shared catalogue", "共有DBに追加しました", "已添加到公共库", "Añadido al catálogo compartido"],
    "duplicate" => ["이미 같은 이름이 있어요", "That name already exists", "同じ名前が既にあります", "名称已存在", "Ese nombre ya existe"],
    "required" => ["필수 항목을 입력해 주세요", "Please fill the required fields", "必須項目を入力してください", "请填写必填项", "Completa los campos obligatorios"],
    "language" => ["언어", "Language", "言語", "语言", "Idioma"],
    "biometric_login" => ["생체 인증 로그인", "Biometric sign-in", "生体認証ログイン", "生物识别登录", "Inicio biométrico"],
    "biometric_unavailable" => ["이 기기에서는 사용할 수 없어요", "Not available on this device", "この端末では利用できません", "此设备不可用", "No disponible en este dispositivo"],
    "auto_login" => ["자동 로그인", "Auto sign-in", "自動ログイン", "自动登录", "Inicio automático"],
    "version" => ["버전", "Version", "バージョン", "版本", "Versión"],
    "back" => ["뒤로", "Back", "戻る", "返回", "Atrás"],
    "loading" => ["불러오는 중…", "Loading…", "読み込み中…", "加载中…", "Cargando…"],
    "error" => ["오류가 발생했어요", "Something went wrong", "エラーが発生しました", "发生错误", "Algo salió mal"],
    "ad" => ["광고", "Ad", "広告", "广告", "Anuncio"],
    "unknown_user" => ["알 수 없는 유저", "Unknown user", "不明なユーザー", "未知用户", "Usuario desconocido"],
    "pieces" => ["장", "pcs", "枚", "张", "uds"],
    "already_owned" => ["이미 보유", "Already owned", "所持中", "已拥有", "Ya posee"],
    "partner_already_owns" => ["상대방이 이미 보유 중인 카드입니다", "Partner already owns this card", "相手が既に所持しているカードです", "对方已拥有该卡片", "La otra persona ya posee esta carta"],
    "select_image" => ["사진 선택", "Select image", "画像を選択", "选择图片", "Seleccionar foto"],
    "choose_file" => ["파일 선택", "Choose File", "ファイルを選択", "选择文件", "Seleccionar archivo"],
    "tab_trade_requests" => ["교환 요청", "Trade Requests", "交換申請", "交换请求", "Solicitudes"],
    "tab_trade_history" => ["수락 내역", "Accepted History", "成立履歴", "已完成", "Historial de Aceptados"],
    "trades_filter_all" => ["전체", "All", "すべて", "全部", "Todos"],
    "trades_filter_inbox" => ["받은 요청", "Received", "受信", "收到请求", "Recibidos"],
    "trades_filter_outbox" => ["보낸 요청", "Sent", "送信", "发出请求", "Enviados"],
    "trades_empty_requests" => ["진행 중인 교환 요청이 없어요", "No active trade requests", "進行中の交換申請はありません", "暂无进行中的交换请求", "No hay solicitudes activas"],
    "trades_empty_accepted" => ["수락된 교환 내역이 없어요", "No completed trades yet", "成立した交換履歴はありません", "暂无已完成的交换记录", "No hay intercambios completados"],
    "trade_you_get" => ["받을 카드", "You receive", "受け取るカード", "获得卡片", "Recibes"],
    "trade_you_give" => ["줄 카드", "You give", "渡すカード", "给出卡片", "Entregas"],
    "trade_accepted_title" => ["교환이 완료되었습니다!", "Trade Completed!", "交換が成立しました！", "交换已成功！", "¡Intercambio completado!"],
    "trade_accepted_desc" => ["{user}님과의 교환이 성공적으로 성사되었습니다.", "The trade with {user} has been successfully completed.", "{user}様との交換が成立しました。", "与 {user} 的交换已顺利完成。", "El intercambio con {user} se ha realizado con éxito."],
    "trade_processing" => ["교환 처리 중...", "Processing trade...", "交換処理中...", "正在处理交换...", "Procesando intercambio..."],
    "confirm" => ["확인", "OK", "確認", "确认", "Confirmar"],
    "series_card_order" => ["카드 순서 변경", "Card Order", "カードの順番", "卡片顺序", "Orden de cartas"],
    "series_card_order_hint" => ["드래그하거나 화살표를 눌러 순서를 변경하세요 (왼쪽이 1순위)", "Drag or click arrows to reorder (left is 1st)", "ドラッグまたは矢印をクリックして順番を変更してください（左が1位）", "拖拽或点击箭头调整顺序（左侧优先）", "Arrastra o pulsa las flechas para reordenar (izquierda es prioridad)"],
    "no_cards_in_series" => ["등록된 카드가 없습니다", "No cards registered", "登録されたカードがありません", "暂无卡片", "No hay cartas"],
    "card_default_prefix" => ["카드", "Card", "カード", "卡片", "Carta"],
    "duplicate_name_warning" => ["이미 존재하는 이름입니다", "This name already exists", "既に存在する名前です", "该名称已存在", "Este nombre ya existe"],
    "trade_locations" => ["교환 가능 위치", "Trade Locations", "交換可能場所", "可交换地点", "Lugares de intercambio"],
    "trade_locations_hint" => ["최대 5개까지 등록할 수 있어요 (미등록 시 모든 장소 가능)", "Up to 5 locations (all available if empty)", "最大5か所まで登録できます（未登録時はどこでも可能）", "最多添加5个地点（未填写则任何地点均可）", "Hasta 5 ubicaciones (todas válidas si está vacío)"],
    "trade_locations_all" => ["모두 가능 (장소 무관)", "Anywhere (all locations)", "どこでも可能（場所不問）", "任何地点均可", "Cualquier lugar"],
    "add_location" => ["추가", "Add", "追加", "添加", "Añadir"],
    "location_placeholder" => ["예: 강남역, 홍대입구역", "e.g. Times Square, Central Park", "例: 渋谷駅、新宿駅", "例：人民广场、江南站", "ej. Estación Sol, Centro"],
    "view_trade_locations" => ["교환 장소 보기", "View Locations", "交換場所を見る", "查看交换地点", "Ver ubicaciones"],
    "partner_trade_locations_title" => ["상대의 교환 가능 장소", "Partner's Trade Locations", "お相手の交換可能場所", "对方的可交换地点", "Ubicaciones de intercambio"],
    "chat_with_user" => ["대화하기", "Chat", "チャット", "聊天", "Chatear"],
    "trade_complete_btn" => ["교환 완료", "Complete Trade", "交換完了", "完成交换", "Completar"],
    "trade_complete_confirm_title" => ["교환을 완료하시겠습니까?", "Mark trade as completed?", "交換を完了にしますか？", "确认已完成交换吗？", "¿Marcar como completado?"],
    "trade_complete_confirm_desc" => ["실제 카드를 교환하셨다면 완료로 변경하세요.", "Mark as completed after physical exchange.", "実際にカードを交換した場合は完了にしてください。", "如果已完成实体卡片交换，请变更为已完成。", "Márcalo como completado si ya has intercambiado."],
    "chat_placeholder" => ["메시지를 입력하세요", "Type a message...", "メッセージを入力...", "输入消息...", "Escribe un mensaje..."],
    "chat_send" => ["전송", "Send", "送信", "发送", "Enviar"],
    "chat_title" => ["교환 대화", "Trade Chat", "交換チャット", "交换聊天", "Chat de intercambio"],
    "max_locations_reached" => ["최대 5개까지 추가할 수 있습니다", "Maximum 5 locations reached", "最大5か所まで追加できます", "最多添加5个地点", "Máximo 5 ubicaciones permitidas"],
}

/// Simple `{name}` placeholder substitution.
#[allow(dead_code)]
pub fn tf(lang: Lang, key: &str, args: &[(&str, String)]) -> String {
    let mut s = t(lang, key).to_string();
    for (k, v) in args {
        s = s.replace(&format!("{{{k}}}"), v);
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn every_key_translated() {
        for k in KEYS {
            for l in Lang::ALL {
                assert!(!t(l, k).is_empty(), "{k} {l:?}");
            }
        }
        assert_eq!(t(Lang::En, "search"), "Search");
        assert_eq!(t(Lang::Ko, "nope"), "???");
    }
}
