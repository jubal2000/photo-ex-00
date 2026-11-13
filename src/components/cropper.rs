//! Image cropping, resizing, JPEG 80% compression, and upload progress components.

use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use base64::Engine;
use dioxus::prelude::*;

use crate::state::use_app;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CropShape {
    /// Photocard vertical rectangle (55:85 ratio)
    CardRect,
    /// Profile circle (1:1 with circle preview)
    Circle,
    /// Square (1:1)
    Square,
}

/// Reads an image from an <input type="file"> event or via JS FileReader fallback for mobile WebViews (iOS/Android).
pub async fn read_image_data_url(e: &Event<FormData>, input_id: &str) -> Option<String> {
    // 1. Try Dioxus native file reader first
    if let Some(file) = e.files().into_iter().next() {
        let ct = file.content_type().unwrap_or_else(|| "image/jpeg".into());
        if let Ok(bytes) = file.read_bytes().await {
            if !bytes.is_empty() {
                let b64 = BASE64_STANDARD.encode(&bytes);
                return Some(format!("data:{ct};base64,{b64}"));
            }
        }
    }

    // 2. Fallback for iOS / Android WebView where e.files() may be empty:
    // Read directly from DOM element using HTML5 FileReader
    let js = format!(
        r##"
        (async () => {{
            try {{
                const el = document.getElementById("{input_id}");
                if (!el || !el.files || !el.files[0]) {{
                    dioxus.send("");
                    return;
                }}
                const file = el.files[0];
                const reader = new FileReader();
                reader.onload = function(evt) {{
                    dioxus.send(evt.target.result || "");
                }};
                reader.onerror = function() {{
                    dioxus.send("");
                }};
                reader.readAsDataURL(file);
            }} catch (err) {{
                console.error("FileReader fallback error:", err);
                dioxus.send("");
            }}
        }})();
        "##
    );
    let mut eval = document::eval(&js);
    if let Ok(val) = eval.recv::<String>().await {
        if val.starts_with("data:image/") {
            return Some(val);
        }
    }
    None
}

fn clamp_pan(
    pan_x: f64,
    pan_y: f64,
    zoom: f64,
    rendered_w: f64,
    rendered_h: f64,
    view_w: f64,
    view_h: f64,
) -> (f64, f64) {
    let curr_w = rendered_w * zoom;
    let curr_h = rendered_h * zoom;
    let max_pan_x = ((curr_w - view_w) / 2.0).max(0.0);
    let max_pan_y = ((curr_h - view_h) / 2.0).max(0.0);
    let cx = pan_x.clamp(-max_pan_x, max_pan_x);
    let cy = pan_y.clamp(-max_pan_y, max_pan_y);
    (cx, cy)
}

/// Modal for interactive image cropping & zooming.
/// Produces a JPEG image compressed to 80% quality with dimensions <= 1024x2048.
#[component]
pub fn ImageCropperModal(
    image_src: String,
    shape: CropShape,
    on_cancel: EventHandler<()>,
    on_confirm: EventHandler<Vec<u8>>,
) -> Element {
    let app = use_app();
    let mut zoom = use_signal(|| 1.0f64);
    let mut pan_x = use_signal(|| 0.0f64);
    let mut pan_y = use_signal(|| 0.0f64);
    let mut nat_w = use_signal(|| 0.0f64);
    let mut nat_h = use_signal(|| 0.0f64);
    let mut is_dragging = use_signal(|| false);
    let mut last_mouse = use_signal(|| (0.0f64, 0.0f64));
    let mut pinch_start_dist = use_signal(|| None::<f64>);
    let mut pinch_start_zoom = use_signal(|| 1.0f64);
    let mut is_processing = use_signal(|| false);

    let (view_w, view_h, mask_class) = match shape {
        CropShape::CardRect => (220.0f64, 340.0f64, "mask-rect"),
        CropShape::Circle => (240.0f64, 240.0f64, "mask-circle"),
        CropShape::Square => (240.0f64, 240.0f64, "mask-square"),
    };

    // Query natural dimensions of the image on mount
    use_effect(move || {
        spawn(async move {
            let js = r#"
            (() => {
                const img = document.getElementById("crop-source-image");
                if (img && img.naturalWidth && img.naturalHeight) {
                    dioxus.send(`${img.naturalWidth},${img.naturalHeight}`);
                } else {
                    dioxus.send("0,0");
                }
            })()
            "#;
            let mut eval = document::eval(js);
            if let Ok(val) = eval.recv::<String>().await {
                let parts: Vec<&str> = val.split(',').collect();
                if parts.len() == 2 {
                    if let (Ok(w), Ok(h)) = (parts[0].parse::<f64>(), parts[1].parse::<f64>()) {
                        if w > 0.0 && h > 0.0 {
                            nat_w.set(w);
                            nat_h.set(h);
                        }
                    }
                }
            }
        });
    });

    let nw = nat_w();
    let nh = nat_h();

    let (rendered_w, rendered_h, min_limit) = if nw > 0.0 && nh > 0.0 {
        let scale_x = view_w / nw;
        let scale_y = view_h / nh;
        let cover_scale = scale_x.max(scale_y);
        (nw * cover_scale, nh * cover_scale, 1.0f64)
    } else {
        (view_w, view_h, 1.0f64)
    };

    let on_apply_original = {
        let mut app = app;
        let img_src_clone = image_src.clone();
        move |_| {
            if is_processing() {
                return;
            }
            is_processing.set(true);

            let img_src = img_src_clone.clone();
            spawn(async move {
                let js = r##"
                (async () => {
                    try {
                        let img = document.getElementById("crop-source-image");
                        if (!img || !img.naturalWidth || !img.naturalHeight) {
                            img = document.querySelector(".crop-image-elem");
                        }
                        if (!img || !img.naturalWidth || !img.naturalHeight) {
                            dioxus.send("ERROR:Image element not ready");
                            return;
                        }

                        let w = img.naturalWidth;
                        let h = img.naturalHeight;
                        const maxDim = 1600;
                        if (w > maxDim || h > maxDim) {
                            if (w > h) {
                                h = Math.round(h * (maxDim / w));
                                w = maxDim;
                            } else {
                                w = Math.round(w * (maxDim / h));
                                h = maxDim;
                            }
                        }

                        const canvas = document.createElement("canvas");
                        canvas.width = Math.round(w);
                        canvas.height = Math.round(h);
                        const ctx = canvas.getContext("2d");

                        ctx.fillStyle = "#ffffff";
                        ctx.fillRect(0, 0, canvas.width, canvas.height);
                        ctx.drawImage(img, 0, 0, canvas.width, canvas.height);

                        const dataUrl = canvas.toDataURL("image/jpeg", 0.85);
                        dioxus.send(dataUrl);
                    } catch (err) {
                        console.error("Apply original error:", err);
                        dioxus.send("ERROR:" + (err && err.message ? err.message : String(err)));
                    }
                })();
                "##;

                let mut eval = document::eval(js);
                match eval.recv::<String>().await {
                    Ok(val) => {
                        if let Some(b64) = val.strip_prefix("data:image/jpeg;base64,") {
                            if let Ok(bytes) = BASE64_STANDARD.decode(b64) {
                                on_confirm.call(bytes);
                                return;
                            } else {
                                app.show_toast("이미지 디코딩에 실패했습니다.");
                            }
                        } else if let Some(err_msg) = val.strip_prefix("ERROR:") {
                            if let Some(pos) = img_src.find(";base64,") {
                                let b64 = &img_src[pos + 8..];
                                if let Ok(bytes) = BASE64_STANDARD.decode(b64) {
                                    on_confirm.call(bytes);
                                    return;
                                }
                            }
                            app.show_toast(format!("처리 오류: {err_msg}"));
                        } else {
                            app.show_toast("잘못된 이미지 형식입니다.");
                        }
                    }
                    Err(e) => {
                        if let Some(pos) = img_src.find(";base64,") {
                            let b64 = &img_src[pos + 8..];
                            if let Ok(bytes) = BASE64_STANDARD.decode(b64) {
                                on_confirm.call(bytes);
                                return;
                            }
                        }
                        app.show_toast(format!("처리 실패: {e}"));
                    }
                }
                is_processing.set(false);
            });
        }
    };

    let on_apply = {
        let mut app = app;
        move |_| {
            if is_processing() {
                return;
            }
            is_processing.set(true);

            let z = zoom();
            let px = pan_x();
            let py = pan_y();
            let vw = view_w;
            let vh = view_h;
            let rw = rendered_w;
            let rh = rendered_h;
            let shape_str = match shape {
                CropShape::CardRect => "rect",
                CropShape::Circle => "circle",
                CropShape::Square => "square",
            };

            spawn(async move {
                let js = format!(
                    r##"
                    (async () => {{
                        try {{
                            let img = document.getElementById("crop-source-image");
                            if (!img || !img.naturalWidth || !img.naturalHeight) {{
                                img = document.querySelector(".crop-image-elem");
                            }}
                            if (!img || !img.naturalWidth || !img.naturalHeight) {{
                                dioxus.send("ERROR:Image element not ready");
                                return;
                            }}

                            const vw = {vw};
                            const vh = {vh};
                            const zoom = {z};
                            const panX = {px};
                            const panY = {py};
                            const shape = "{shape_str}";
                            let renderedW = {rw};
                            let renderedH = {rh};

                            if (!renderedW || !renderedH || renderedW <= 0 || renderedH <= 0) {{
                                const scaleX = vw / img.naturalWidth;
                                const scaleY = vh / img.naturalHeight;
                                const coverScale = Math.max(scaleX, scaleY);
                                renderedW = img.naturalWidth * coverScale;
                                renderedH = img.naturalHeight * coverScale;
                            }}

                            // Target dimensions <= 1024 x 2048
                            let outW = 1024;
                            let outH = 1024;
                            if (shape === "rect") {{
                                outW = 1024;
                                outH = Math.round(1024 * (vh / vw));
                            }}

                            if (outW > 1024) {{
                                outH = Math.round(outH * (1024 / outW));
                                outW = 1024;
                            }}
                            if (outH > 2048) {{
                                outW = Math.round(outW * (2048 / outH));
                                outH = 2048;
                            }}

                            const canvas = document.createElement("canvas");
                            canvas.width = Math.round(outW);
                            canvas.height = Math.round(outH);
                            const ctx = canvas.getContext("2d");

                            // Fill white background for clean jpegs
                            ctx.fillStyle = "#ffffff";
                            ctx.fillRect(0, 0, canvas.width, canvas.height);

                            // Current dimensions of image on screen under zoom
                            const currW = renderedW * zoom;
                            const currH = renderedH * zoom;

                            // Viewfinder top-left is (0, 0), center is (vw/2, vh/2).
                            // Image top-left in viewfinder coordinates:
                            const imgLeft = (vw - currW) / 2.0 + panX;
                            const imgTop = (vh - currH) / 2.0 + panY;

                            // Scale from screen viewfinder pixels to canvas pixels
                            const scaleToCanvas = outW / vw;

                            const destX = imgLeft * scaleToCanvas;
                            const destY = imgTop * scaleToCanvas;
                            const destW = currW * scaleToCanvas;
                            const destH = currH * scaleToCanvas;

                            // Draw image onto canvas: exact 1:1 match with viewfinder display
                            ctx.drawImage(img, 0, 0, img.naturalWidth, img.naturalHeight, destX, destY, destW, destH);

                            // 80% JPEG compression
                            const dataUrl = canvas.toDataURL("image/jpeg", 0.80);
                            dioxus.send(dataUrl);
                        }} catch (err) {{
                            console.error("Crop error:", err);
                            dioxus.send("ERROR:" + (err && err.message ? err.message : String(err)));
                        }}
                    }})();
                    "##
                );

                let mut eval = document::eval(&js);
                match eval.recv::<String>().await {
                    Ok(val) => {
                        if let Some(b64) = val.strip_prefix("data:image/jpeg;base64,") {
                            if let Ok(bytes) = BASE64_STANDARD.decode(b64) {
                                on_confirm.call(bytes);
                                return;
                            } else {
                                app.show_toast("이미지 디코딩에 실패했습니다.");
                            }
                        } else if let Some(err_msg) = val.strip_prefix("ERROR:") {
                            app.show_toast(format!("자르기 오류: {err_msg}"));
                        } else {
                            app.show_toast("잘못된 이미지 형식입니다.");
                        }
                    }
                    Err(e) => {
                        app.show_toast(format!("자르기 처리 실패: {e}"));
                    }
                }
                is_processing.set(false);
            });
        }
    };

    let img_transform = if nw > 0.0 && nh > 0.0 {
        format!(
            "width: {rendered_w:.1}px; height: {rendered_h:.1}px; max-width: none; max-height: none; transform: translate({px}px, {py}px) scale({z});",
            px = pan_x(),
            py = pan_y(),
            z = zoom()
        )
    } else {
        format!(
            "transform: translate({px}px, {py}px) scale({z});",
            px = pan_x(),
            py = pan_y(),
            z = zoom()
        )
    };

    rsx! {
        div { class: "crop-modal-overlay",
            div { class: "crop-modal-container",
                div { class: "crop-modal-header",
                    h3 { {app.tr("crop_image")} }
                    button {
                        class: "crop-close-btn",
                        title: app.tr("cancel"),
                        aria_label: app.tr("cancel"),
                        onclick: move |_| on_cancel.call(()),
                        "✕"
                    }
                }

                div { class: "crop-viewport-wrapper",
                    // Interactive touch/mouse pan area
                    div {
                        class: "crop-viewport",
                        onmousedown: move |e| {
                            is_dragging.set(true);
                            last_mouse.set((e.client_coordinates().x, e.client_coordinates().y));
                        },
                        onmousemove: move |e| {
                            if is_dragging() {
                                let (lx, ly) = last_mouse();
                                let cx = e.client_coordinates().x;
                                let cy = e.client_coordinates().y;
                                let (clamped_x, clamped_y) = clamp_pan(
                                    pan_x() + (cx - lx),
                                    pan_y() + (cy - ly),
                                    zoom(),
                                    rendered_w,
                                    rendered_h,
                                    view_w,
                                    view_h,
                                );
                                pan_x.set(clamped_x);
                                pan_y.set(clamped_y);
                                last_mouse.set((cx, cy));
                            }
                        },
                        onmouseup: move |_| is_dragging.set(false),
                        onmouseleave: move |_| is_dragging.set(false),
                        onwheel: move |e| {
                            e.prevent_default();
                            let dy = e.delta().strip_units().y;
                            let step = -dy * 0.002;
                            let new_z = (zoom() + step).clamp(min_limit, 3.0);
                            zoom.set(new_z);
                            let (clamped_x, clamped_y) = clamp_pan(
                                pan_x(),
                                pan_y(),
                                new_z,
                                rendered_w,
                                rendered_h,
                                view_w,
                                view_h,
                            );
                            pan_x.set(clamped_x);
                            pan_y.set(clamped_y);
                        },
                        ontouchstart: move |e| {
                            let touches = e.touches();
                            if touches.len() >= 2 {
                                let t1 = &touches[0];
                                let t2 = &touches[1];
                                let dx = t1.client_coordinates().x - t2.client_coordinates().x;
                                let dy = t1.client_coordinates().y - t2.client_coordinates().y;
                                let dist = (dx * dx + dy * dy).sqrt();
                                pinch_start_dist.set(Some(dist));
                                pinch_start_zoom.set(zoom());
                                is_dragging.set(false);
                            } else if let Some(t) = touches.first() {
                                is_dragging.set(true);
                                last_mouse.set((t.client_coordinates().x, t.client_coordinates().y));
                                pinch_start_dist.set(None);
                            }
                        },
                        ontouchmove: move |e| {
                            let touches = e.touches();
                            if touches.len() >= 2 {
                                if let Some(start_dist) = pinch_start_dist() {
                                    if start_dist > 5.0 {
                                        let t1 = &touches[0];
                                        let t2 = &touches[1];
                                        let dx = t1.client_coordinates().x - t2.client_coordinates().x;
                                        let dy = t1.client_coordinates().y - t2.client_coordinates().y;
                                        let curr_dist = (dx * dx + dy * dy).sqrt();
                                        let factor = curr_dist / start_dist;
                                        let new_z = (pinch_start_zoom() * factor).clamp(min_limit, 3.0);
                                        zoom.set(new_z);
                                        let (clamped_x, clamped_y) = clamp_pan(
                                            pan_x(),
                                            pan_y(),
                                            new_z,
                                            rendered_w,
                                            rendered_h,
                                            view_w,
                                            view_h,
                                        );
                                        pan_x.set(clamped_x);
                                        pan_y.set(clamped_y);
                                    }
                                }
                            } else if is_dragging() && touches.len() == 1 {
                                if let Some(t) = touches.first() {
                                    let (lx, ly) = last_mouse();
                                    let cx = t.client_coordinates().x;
                                    let cy = t.client_coordinates().y;
                                    let (clamped_x, clamped_y) = clamp_pan(
                                        pan_x() + (cx - lx),
                                        pan_y() + (cy - ly),
                                        zoom(),
                                        rendered_w,
                                        rendered_h,
                                        view_w,
                                        view_h,
                                    );
                                    pan_x.set(clamped_x);
                                    pan_y.set(clamped_y);
                                    last_mouse.set((cx, cy));
                                }
                            }
                        },
                        ontouchend: move |e| {
                            let touches = e.touches();
                            if touches.len() < 2 {
                                pinch_start_dist.set(None);
                            }
                            if touches.is_empty() {
                                is_dragging.set(false);
                            } else if touches.len() == 1 {
                                if let Some(t) = touches.first() {
                                    is_dragging.set(true);
                                    last_mouse.set((t.client_coordinates().x, t.client_coordinates().y));
                                }
                            }
                        },
                        ontouchcancel: move |_| {
                            is_dragging.set(false);
                            pinch_start_dist.set(None);
                        },

                        // The underlying image in DOM
                        img {
                            id: "crop-source-image",
                            class: "crop-image-elem",
                            src: "{image_src}",
                            style: "{img_transform}",
                            draggable: "false",
                            onload: move |_| {
                                spawn(async move {
                                    let js = r#"
                                    (() => {
                                        const img = document.getElementById("crop-source-image");
                                        if (img && img.naturalWidth && img.naturalHeight) {
                                            dioxus.send(`${img.naturalWidth},${img.naturalHeight}`);
                                        } else {
                                            dioxus.send("0,0");
                                        }
                                    })()
                                    "#;
                                    let mut eval = document::eval(js);
                                    if let Ok(val) = eval.recv::<String>().await {
                                        let parts: Vec<&str> = val.split(',').collect();
                                        if parts.len() == 2 {
                                            if let (Ok(w), Ok(h)) = (parts[0].parse::<f64>(), parts[1].parse::<f64>()) {
                                                if w > 0.0 && h > 0.0 {
                                                    nat_w.set(w);
                                                    nat_h.set(h);
                                                }
                                            }
                                        }
                                    }
                                });
                            },
                        }

                        // Viewfinder cutout mask overlay
                        div { class: "crop-mask {mask_class}" }

                        // Viewfinder frame border
                        div {
                            class: "crop-frame {mask_class}",
                            style: "width: {view_w}px; height: {view_h}px;",
                        }
                    }
                }

                p { class: "crop-guide-text", {app.tr("crop_guide")} }

                // Zoom control bar
                div { class: "crop-zoom-bar",
                    span { class: "crop-zoom-label", {app.tr("crop_zoom")} }
                    button {
                        class: "btn-zoom-step",
                        onclick: move |_| {
                            let new_z = (zoom() - 0.1).clamp(min_limit, 3.0);
                            zoom.set(new_z);
                            let (clamped_x, clamped_y) = clamp_pan(
                                pan_x(),
                                pan_y(),
                                new_z,
                                rendered_w,
                                rendered_h,
                                view_w,
                                view_h,
                            );
                            pan_x.set(clamped_x);
                            pan_y.set(clamped_y);
                        },
                        "−"
                    }
                    input {
                        r#type: "range",
                        min: "{min_limit:.2}",
                        max: "3.0",
                        step: "0.01",
                        value: "{zoom()}",
                        oninput: move |e| {
                            if let Ok(v) = e.value().parse::<f64>() {
                                let new_z = v.clamp(min_limit, 3.0);
                                zoom.set(new_z);
                                let (clamped_x, clamped_y) = clamp_pan(
                                    pan_x(),
                                    pan_y(),
                                    new_z,
                                    rendered_w,
                                    rendered_h,
                                    view_w,
                                    view_h,
                                );
                                pan_x.set(clamped_x);
                                pan_y.set(clamped_y);
                            }
                        }
                    }
                    button {
                        class: "btn-zoom-step",
                        onclick: move |_| {
                            let new_z = (zoom() + 0.1).clamp(min_limit, 3.0);
                            zoom.set(new_z);
                            let (clamped_x, clamped_y) = clamp_pan(
                                pan_x(),
                                pan_y(),
                                new_z,
                                rendered_w,
                                rendered_h,
                                view_w,
                                view_h,
                            );
                            pan_x.set(clamped_x);
                            pan_y.set(clamped_y);
                        },
                        "+"
                    }
                }

                // Modal Actions
                div { class: "crop-modal-actions",
                    button {
                        class: "btn btn-outline",
                        disabled: is_processing(),
                        onclick: on_apply_original,
                        {app.tr("apply_original")}
                    }
                    button {
                        class: "btn btn-primary",
                        disabled: is_processing(),
                        onclick: on_apply,
                        if is_processing() {
                            {app.tr("loading")}
                        } else {
                            {app.tr("crop_apply")}
                        }
                    }
                }
            }
        }
    }
}

/// Fullscreen dim overlay with progress indicator for upload tasks.
#[component]
pub fn UploadProgressOverlay(progress_percent: Option<u8>) -> Element {
    let app = use_app();
    let pct = progress_percent.unwrap_or(75);

    rsx! {
        div { class: "upload-dim-overlay",
            div { class: "upload-card",
                div { class: "upload-icon-spinner spinner" }
                h4 { class: "upload-title", {app.tr("uploading_image")} }
                div { class: "upload-progress-track",
                    div {
                        class: "upload-progress-bar",
                        style: "width: {pct}%;",
                    }
                }
                span { class: "upload-pct-label", "{pct}%" }
            }
        }
    }
}
