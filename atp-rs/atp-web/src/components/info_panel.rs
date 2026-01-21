//! Information panel showing field and guide star details

use leptos::*;
use crate::models::{FieldResponse, GuideStarResult};

#[component]
pub fn InfoPanel(
    /// Star field data
    field: ReadSignal<Option<FieldResponse>>,
    /// Guide star selection result
    gs_result: ReadSignal<Option<GuideStarResult>>,
    /// Error message
    error: ReadSignal<Option<String>>,
) -> impl IntoView {
    view! {
        <div class="info-panel">
            <h2>"Status"</h2>

            {move || error.get().map(|err| view! {
                <div class="alert alert-error">
                    <strong>"⚠ Error"</strong>
                    <p>{err}</p>
                </div>
            })}

            // Field information
            {move || field.get().map(|f| view! {
                <section class="info-section">
                    <h3>"Star Field"</h3>
                    <dl class="info-list">
                        <dt>"Stars in field:"</dt>
                        <dd>{f.stars.len()}</dd>

                        <dt>"V magnitude range:"</dt>
                        <dd>{
                            if !f.stars.is_empty() {
                                let min_v = f.stars.iter().map(|s| s.v_mag).fold(f64::INFINITY, f64::min);
                                let max_v = f.stars.iter().map(|s| s.v_mag).fold(f64::NEG_INFINITY, f64::max);
                                format!("{:.1} - {:.1}", min_v, max_v)
                            } else {
                                "N/A".to_string()
                            }
                        }</dd>

                        <dt>"Observatory time:"</dt>
                        <dd>{&f.obs_time}</dd>

                        <dt>"Sidereal time:"</dt>
                        <dd>{format!("{:.4} rad", f.sidereal_time)}</dd>

                        <dt>"Target coordinates:"</dt>
                        <dd>
                            "RA: " {format!("{:.2}°", f.target_ra)}
                            <br/>
                            "Dec: " {format!("{:.2}°", f.target_dec)}
                        </dd>

                        <dt>"Pointing:"</dt>
                        <dd>
                            "Alt: " {format!("{:.2}°", f.target_alt)}
                            <br/>
                            "Az: " {format!("{:.2}°", f.target_az)}
                        </dd>
                    </dl>
                </section>
            })}

            // Guide star results
            {move || gs_result.get().map(|result| {
                // Clone data needed for nested closures
                let field_stars = result.field.stars.clone();
                let tt7_idx = result.tt7_star_idx;
                let sh_indices = result.sh_star_indices.clone();

                view! {
                    <section class="info-section">
                        <h3>"Guide Stars"</h3>

                        <div class="gs-info tt7">
                            <h4>"TT7 Guide Star"</h4>
                            <dl class="info-list">
                                <dt>"Star index:"</dt>
                                <dd>{tt7_idx}</dd>

                                <dt>"Probe:"</dt>
                                <dd>{result.tt7_probe_idx}</dd>

                                <dt>"RMS error:"</dt>
                                <dd class="highlight">{format!("{:.2} mas", result.tt7_error_mas)}</dd>

                                {
                                    let stars_clone = field_stars.clone();
                                    move || stars_clone.iter()
                                        .find(|s| s.idx == tt7_idx)
                                        .map(|star| view! {
                                            <>
                                                <dt>"V magnitude:"</dt>
                                                <dd>{format!("{:.2}", star.v_mag)}</dd>

                                                <dt>"Position:"</dt>
                                                <dd>
                                                    "X: " {format!("{:.1}′", star.x)}
                                                    <br/>
                                                    "Y: " {format!("{:.1}′", star.y)}
                                                </dd>
                                            </>
                                        })
                                }
                            </dl>
                        </div>

                        <div class="gs-info sh">
                            <h4>"SH Guide Stars"</h4>
                            <dl class="info-list">
                                <dt>"Median WFE:"</dt>
                                <dd class="highlight">{format!("{:.1} nm RMS", result.sh_wfe_nm)}</dd>

                                <dt>"Star indices:"</dt>
                                <dd>{format!("{:?}", sh_indices)}</dd>

                                <dt>"Probes:"</dt>
                                <dd>{format!("{:?}", result.sh_probe_indices)}</dd>
                            </dl>

                            <table class="gs-table">
                                <thead>
                                    <tr>
                                        <th>"#"</th>
                                        <th>"V mag"</th>
                                        <th>"X (′)"</th>
                                        <th>"Y (′)"</th>
                                    </tr>
                                </thead>
                                <tbody>
                                    {
                                        sh_indices.iter().enumerate().map(|(i, &idx)| {
                                            field_stars.iter()
                                                .find(|s| s.idx == idx)
                                                .map(|star| view! {
                                                    <tr>
                                                        <td>{i + 1}</td>
                                                        <td>{format!("{:.1}", star.v_mag)}</td>
                                                        <td>{format!("{:.1}", star.x)}</td>
                                                        <td>{format!("{:.1}", star.y)}</td>
                                                    </tr>
                                                })
                                        }).collect::<Vec<_>>()
                                    }
                                </tbody>
                            </table>
                        </div>

                    <div class="performance-summary">
                        <h4>"Performance Assessment"</h4>
                        <div class="assessment">
                            {move || {
                                let quality = if result.tt7_error_mas < 5.0 && result.sh_wfe_nm < 150.0 {
                                    ("✓ Excellent", "success")
                                } else if result.tt7_error_mas < 10.0 && result.sh_wfe_nm < 200.0 {
                                    ("✓ Good", "good")
                                } else if result.tt7_error_mas < 20.0 && result.sh_wfe_nm < 300.0 {
                                    ("⚠ Acceptable", "warning")
                                } else {
                                    ("✗ Poor", "error")
                                };

                                view! {
                                    <div class=format!("quality-badge {}", quality.1)>
                                        {quality.0}
                                    </div>
                                }
                            }}
                        </div>
                    </div>
                </section>
                }
            })}

            // Help section
            <section class="info-section help">
                <h3>"Quick Guide"</h3>
                <ol>
                    <li>"Set target coordinates or name"</li>
                    <li>"Adjust V magnitude limit if needed"</li>
                    <li>"Click 'Query Field' to fetch stars from TIC"</li>
                    <li>"Click 'Find Guide Stars' to run selection"</li>
                    <li>"Use 'Start Time' to see field rotation"</li>
                </ol>

                <div class="help-note">
                    <strong>"Note:"</strong>
                    " TIC queries require internet connection and may take 10-30 seconds."
                </div>
            </section>
        </div>
    }
}
