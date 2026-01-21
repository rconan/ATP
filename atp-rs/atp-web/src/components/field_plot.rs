//! Star field plot component using Plotly

use leptos::*;
use wasm_bindgen::prelude::*;
use crate::models::{FieldResponse, GuideStarResult};

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = Plotly)]
    fn newPlot(div_id: &str, data: JsValue, layout: JsValue);

    #[wasm_bindgen(js_namespace = Plotly)]
    fn react(div_id: &str, data: JsValue, layout: JsValue);
}

#[component]
pub fn FieldPlot(
    /// Star field data
    field: ReadSignal<Option<FieldResponse>>,
    /// Guide star selection result
    gs_result: ReadSignal<Option<GuideStarResult>>,
    /// Loading state
    loading: ReadSignal<bool>,
) -> impl IntoView {
    let plot_div_ref = create_node_ref::<html::Div>();

    // Update plot when data changes
    create_effect(move |_| {
        if let Some(field_data) = field.get() {
            update_plot(&field_data, gs_result.get());
        }
    });

    view! {
        <div class="field-plot-container">
            <div class="plot-header">
                <h2>"AGWS Star Field"</h2>
                {move || field.get().map(|f| view! {
                    <div class="plot-info">
                        <span>"Time: " {f.obs_time}</span>
                        <span>" | Alt: " {format!("{:.1}°", f.target_alt)}</span>
                        <span>", Az: " {format!("{:.1}°", f.target_az)}</span>
                    </div>
                })}
            </div>

            {move || if loading.get() {
                view! {
                    <div class="loading-overlay">
                        <div class="spinner"></div>
                        <p>"Loading star field..."</p>
                    </div>
                }.into_view()
            } else {
                view! {
                    <div
                        id="plotly-div"
                        node_ref=plot_div_ref
                        class="plotly-container"
                    >
                    </div>
                }.into_view()
            }}

            <div class="plot-legend">
                <div class="legend-item">
                    <span class="legend-marker star"></span>
                    <span>"Stars"</span>
                </div>
                <div class="legend-item">
                    <span class="legend-marker tt7"></span>
                    <span>"TT7 Guide Star"</span>
                </div>
                <div class="legend-item">
                    <span class="legend-marker sh"></span>
                    <span>"SH Guide Stars"</span>
                </div>
                <div class="legend-item">
                    <span class="legend-marker probe"></span>
                    <span>"Probes"</span>
                </div>
            </div>
        </div>
    }
}

/// Update Plotly plot with new data
fn update_plot(field: &FieldResponse, gs_result: Option<GuideStarResult>) {
    // Prepare data for Plotly
    let stars_x: Vec<f64> = field.stars.iter().map(|s| s.x).collect();
    let stars_y: Vec<f64> = field.stars.iter().map(|s| s.y).collect();
    let stars_text: Vec<String> = field
        .stars
        .iter()
        .map(|s| format!("V={:.1}, J={:.1}", s.v_mag, s.j_mag))
        .collect();
    let stars_color: Vec<f64> = field.stars.iter().map(|s| s.v_mag).collect();

    // Build Plotly traces
    let mut traces = Vec::new();

    // Stars scatter trace
    traces.push(serde_json::json!({
        "type": "scatter",
        "mode": "markers",
        "name": "Stars",
        "x": stars_x,
        "y": stars_y,
        "text": stars_text,
        "marker": {
            "size": 8,
            "color": stars_color,
            "colorscale": "Plasma",
            "colorbar": {
                "title": "V mag",
                "x": 1.05,
            },
            "cmin": field.stars.iter().map(|s| s.v_mag).fold(f64::INFINITY, f64::min),
            "cmax": field.stars.iter().map(|s| s.v_mag).fold(f64::NEG_INFINITY, f64::max),
        },
        "hovertemplate": "<b>Star</b><br>%{text}<extra></extra>",
    }));

    // Probes
    for probe in &field.probes {
        let patrol_radius = 21.0; // arcmin
        let theta = probe.idx as f64 * std::f64::consts::PI / 2.0;
        let arc_start = theta - 26.35_f64.to_radians();
        let arc_end = theta + 26.35_f64.to_radians();

        // Probe patrol area (wedge shape)
        let n_points = 50;
        let mut wedge_x = vec![probe.x];
        let mut wedge_y = vec![probe.y];

        for i in 0..=n_points {
            let angle = arc_start + (arc_end - arc_start) * i as f64 / n_points as f64;
            wedge_x.push(probe.x + patrol_radius * angle.cos());
            wedge_y.push(probe.y + patrol_radius * angle.sin());
        }
        wedge_x.push(probe.x);
        wedge_y.push(probe.y);

        traces.push(serde_json::json!({
            "type": "scatter",
            "mode": "lines",
            "name": format!("Probe {}", probe.idx),
            "x": wedge_x,
            "y": wedge_y,
            "fill": "toself",
            "fillcolor": "rgba(0, 0, 139, 0.05)",
            "line": {"color": "navy", "width": 1},
            "hoverinfo": "skip",
        }));

        // Probe center marker
        traces.push(serde_json::json!({
            "type": "scatter",
            "mode": "markers",
            "name": format!("Probe {} center", probe.idx),
            "x": vec![probe.x],
            "y": vec![probe.y],
            "marker": {"size": 12, "color": "navy", "symbol": "square"},
            "showlegend": false,
            "hovertemplate": format!("<b>Probe {}</b><extra></extra>", probe.idx),
        }));
    }

    // Guide stars (if selected)
    if let Some(result) = gs_result {
        // TT7 guide star
        if let Some(tt7_star) = field.stars.iter().find(|s| s.idx == result.tt7_star_idx) {
            traces.push(serde_json::json!({
                "type": "scatter",
                "mode": "markers",
                "name": "TT7 Guide Star",
                "x": vec![tt7_star.x],
                "y": vec![tt7_star.y],
                "marker": {
                    "size": 15,
                    "color": "red",
                    "symbol": "star",
                    "line": {"color": "darkred", "width": 2},
                },
                "hovertemplate": format!(
                    "<b>TT7 GS</b><br>V={:.1}<br>Error={:.2} mas<extra></extra>",
                    tt7_star.v_mag, result.tt7_error_mas
                ),
            }));

            // Line to TT7 probe
            if let Some(tt7_probe) = field.probes.iter().find(|p| p.idx == result.tt7_probe_idx) {
                traces.push(serde_json::json!({
                    "type": "scatter",
                    "mode": "lines",
                    "x": vec![tt7_probe.x, tt7_star.x],
                    "y": vec![tt7_probe.y, tt7_star.y],
                    "line": {"color": "red", "width": 3},
                    "showlegend": false,
                    "hoverinfo": "skip",
                }));
            }
        }

        // SH guide stars
        for (i, &star_idx) in result.sh_star_indices.iter().enumerate() {
            if let Some(sh_star) = field.stars.iter().find(|s| s.idx == star_idx) {
                traces.push(serde_json::json!({
                    "type": "scatter",
                    "mode": "markers",
                    "name": format!("SH GS {}", i + 1),
                    "x": vec![sh_star.x],
                    "y": vec![sh_star.y],
                    "marker": {
                        "size": 12,
                        "color": "green",
                        "symbol": "circle",
                        "line": {"color": "darkgreen", "width": 2},
                    },
                    "hovertemplate": format!(
                        "<b>SH GS {}</b><br>V={:.1}<extra></extra>",
                        i + 1, sh_star.v_mag
                    ),
                }));

                // Line to SH probe
                if let Some(probe_idx) = result.sh_probe_indices.get(i) {
                    if let Some(sh_probe) = field.probes.iter().find(|p| p.idx == *probe_idx) {
                        traces.push(serde_json::json!({
                            "type": "scatter",
                            "mode": "lines",
                            "x": vec![sh_probe.x, sh_star.x],
                            "y": vec![sh_probe.y, sh_star.y],
                            "line": {"color": "green", "width": 2},
                            "showlegend": false,
                            "hoverinfo": "skip",
                        }));
                    }
                }
            }
        }
    }

    // Exclusion circles
    traces.push(serde_json::json!({
        "type": "scatter",
        "mode": "lines",
        "name": "Exclusion zones",
        "x": circle_points(0.0, 3.0, 100).0,
        "y": circle_points(0.0, 3.0, 100).1,
        "line": {"color": "red", "width": 1, "dash": "dash"},
        "hoverinfo": "skip",
    }));

    traces.push(serde_json::json!({
        "type": "scatter",
        "mode": "lines",
        "name": "Search boundary",
        "x": circle_points(0.0, 10.0, 100).0,
        "y": circle_points(0.0, 10.0, 100).1,
        "line": {"color": "red", "width": 1, "dash": "dash"},
        "showlegend": false,
        "hoverinfo": "skip",
    }));

    // Layout
    let layout = serde_json::json!({
        "xaxis": {
            "title": "X (arcmin)",
            "range": [-25, 25],
            "scaleanchor": "y",
            "scaleratio": 1,
        },
        "yaxis": {
            "title": "Y (arcmin)",
            "range": [-25, 25],
        },
        "hovermode": "closest",
        "showlegend": false,
        "margin": {"l": 50, "r": 50, "t": 10, "b": 50},
        "plot_bgcolor": "#f8f9fa",
        "paper_bgcolor": "white",
    });

    // Update plot
    let traces_js = serde_wasm_bindgen::to_value(&traces).unwrap();
    let layout_js = serde_wasm_bindgen::to_value(&layout).unwrap();

    react("plotly-div", traces_js, layout_js);
}

/// Generate circle points for plotting
fn circle_points(center_x: f64, radius: f64, n: usize) -> (Vec<f64>, Vec<f64>) {
    let mut x = Vec::with_capacity(n + 1);
    let mut y = Vec::with_capacity(n + 1);

    for i in 0..=n {
        let theta = 2.0 * std::f64::consts::PI * i as f64 / n as f64;
        x.push(center_x + radius * theta.cos());
        y.push(radius * theta.sin());
    }

    (x, y)
}
