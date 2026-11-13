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
    let mut is_dragging = use_signal(|| false);
    let mut last_mouse = use_signal(|| (0.0f64, 0.0f64));
    let mut is_processing = use_signal(|| false);

    let (view_w, view_h, mask_class) = match shape {
        CropShape::CardRect => (220.0f64, 340.0f64, "mask-rect"),
        CropShape::Circle => (240.0f64, 240.0f64, "mask-circle"),
        CropShape::Square => (240.0f64, 240.0f64, "mask-square"),
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

                            // Calculate base scale to 'cover' the viewfinder
                            const scaleX = vw / img.naturalWidth;
                            const scaleY = vh / img.naturalHeight;
                            const baseScale = Math.max(scaleX, scaleY);
                            const currentScale = baseScale * zoom;

                            // Scale from screen pixels to natural image pixels
                            const toNatural = 1.0 / currentScale;
                            const sWidth = vw * toNatural;
                            const sHeight = vh * toNatural;

                            const centerImgX = (img.naturalWidth / 2.0) - (panX * toNatural);
                            const centerImgY = (img.naturalHeight / 2.0) - (panY * toNatural);

                            const sx = Math.max(0, centerImgX - (sWidth / 2.0));
                            const sy = Math.max(0, centerImgY - (sHeight / 2.0));
                            const sw = Math.min(img.naturalWidth - sx, sWidth);
                            const sh = Math.min(img.naturalHeight - sy, sHeight);

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

                            // Draw cropped region directly from already-rendered DOM image
                            ctx.drawImage(img, sx, sy, sw, sh, 0, 0, canvas.width, canvas.height);

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

    let img_transform = format!(
        "transform: translate({px}px, {py}px) scale({z});",
        px = pan_x(),
        py = pan_y(),
        z = zoom()
    );

    rsx! {
        div { class: "crop-modal-overlay",
            div { class: "crop-modal-container",
                div { class: "crop-modal-header",
                    h3 { {app.tr("crop_image")} }
                    button {
                        class: "crop-close-btn",
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
                                pan_x += cx - lx;
                                pan_y += cy - ly;
                                last_mouse.set((cx, cy));
                            }
                        },
                        onmouseup: move |_| is_dragging.set(false),
                        onmouseleave: move |_| is_dragging.set(false),
                        ontouchstart: move |e| {
                            if let Some(t) = e.touches().first() {
                                is_dragging.set(true);
                                last_mouse.set((t.client_coordinates().x, t.client_coordinates().y));
                            }
                        },
                        ontouchmove: move |e| {
                            if is_dragging() {
                                if let Some(t) = e.touches().first() {
                                    let (lx, ly) = last_mouse();
                                    let cx = t.client_coordinates().x;
                                    let cy = t.client_coordinates().y;
                                    pan_x += cx - lx;
                                    pan_y += cy - ly;
                                    last_mouse.set((cx, cy));
                                }
                            }
                        },
                        ontouchend: move |_| is_dragging.set(false),

                        // The underlying image in DOM
                        img {
                            id: "crop-source-image",
                            class: "crop-image-elem",
                            src: "{image_src}",
                            style: "{img_transform}",
                            draggable: "false",
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
                        onclick: move |_| zoom.set((zoom() - 0.2).max(1.0)),
                        "−"
                    }
                    input {
                        r#type: "range",
                        min: "1.0",
                        max: "3.0",
                        step: "0.05",
                        value: "{zoom()}",
                        oninput: move |e| {
                            if let Ok(v) = e.value().parse::<f64>() {
                                zoom.set(v);
                            }
                        }
                    }
                    button {
                        class: "btn-zoom-step",
                        onclick: move |_| zoom.set((zoom() + 0.2).min(3.0)),
                        "+"
                    }
                }

                // Modal Actions
                div { class: "crop-modal-actions",
                    button {
                        class: "btn btn-outline",
                        disabled: is_processing(),
                        onclick: move |_| on_cancel.call(()),
                        {app.tr("cancel")}
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
