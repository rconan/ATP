//! Control panel component for query parameters

use leptos::*;
use crate::models::FieldQueryParams;

#[component]
pub fn Controls(
    /// Current query parameters
    params: ReadSignal<FieldQueryParams>,
    /// Callback when parameters change
    on_params_change: Callback<FieldQueryParams>,
    /// Callback for query button
    on_query: Callback<()>,
    /// Callback for find guide stars button
    on_find_gs: Callback<()>,
    /// Callback for time toggle
    on_toggle_time: Callback<()>,
    /// Whether time evolution is running
    time_running: ReadSignal<bool>,
    /// Whether any operation is loading
    loading: ReadSignal<bool>,
) -> impl IntoView {
    // Local state for inputs
    let (time_input, set_time_input) = create_signal(params.get().time.clone());
    let (time_res_input, set_time_res_input) = create_signal(params.get().time_resolution.to_string());
    let (target_input, set_target_input) = create_signal(
        params.get().target_name.clone().unwrap_or_else(|| "None".to_string())
    );
    let (alt_input, set_alt_input) = create_signal(
        params.get().altitude.map(|a| a.to_string()).unwrap_or_default()
    );
    let (az_input, set_az_input) = create_signal(
        params.get().azimuth.map(|a| a.to_string()).unwrap_or_default()
    );
    let (vmag_input, set_vmag_input) = create_signal(params.get().v_mag_limit.to_string());

    // Helper to build params from current inputs
    let build_params = move || -> FieldQueryParams {
        FieldQueryParams {
            time: time_input.get(),
            time_resolution: time_res_input.get().parse().unwrap_or(60.0),
            target_name: {
                let name = target_input.get();
                if name == "None" || name.is_empty() {
                    None
                } else {
                    Some(name)
                }
            },
            altitude: alt_input.get().parse().ok(),
            azimuth: az_input.get().parse().ok(),
            v_mag_limit: vmag_input.get().parse().unwrap_or(16.0),
            radius: params.get().radius,
            exclude_radius: params.get().exclude_radius,
        }
    };

    view! {
        <div class="controls">
            <h2>"Query Parameters"</h2>

            <div class="control-group">
                <label for="datetime">"Date and Time (UTC)"</label>
                <input
                    type="text"
                    id="datetime"
                    value=time_input
                    on:input=move |ev| {
                        set_time_input.set(event_target_value(&ev));
                    }
                    disabled=move || loading.get()
                />
            </div>

            <div class="control-group">
                <label for="time-res">"Time Resolution (seconds)"</label>
                <input
                    type="number"
                    id="time-res"
                    value=time_res_input
                    on:input=move |ev| {
                        set_time_res_input.set(event_target_value(&ev));
                    }
                    disabled=move || loading.get()
                />
            </div>

            <div class="control-group">
                <label for="target">"Target Name or Coordinates"</label>
                <input
                    type="text"
                    id="target"
                    value=target_input
                    placeholder="None or (ra,dec)"
                    on:input=move |ev| {
                        set_target_input.set(event_target_value(&ev));
                    }
                    disabled=move || loading.get()
                />
            </div>

            <div class="control-group">
                <label for="altitude">"Telescope Altitude (degrees)"</label>
                <input
                    type="number"
                    id="altitude"
                    value=alt_input
                    min="0"
                    max="90"
                    step="0.1"
                    on:input=move |ev| {
                        set_alt_input.set(event_target_value(&ev));
                    }
                    disabled=move || loading.get()
                />
            </div>

            <div class="control-group">
                <label for="azimuth">"Telescope Azimuth (degrees)"</label>
                <input
                    type="number"
                    id="azimuth"
                    value=az_input
                    min="0"
                    max="360"
                    step="0.1"
                    on:input=move |ev| {
                        set_az_input.set(event_target_value(&ev));
                    }
                    disabled=move || loading.get()
                />
            </div>

            <div class="control-group">
                <label for="vmag">"V Magnitude Limit"</label>
                <input
                    type="range"
                    id="vmag"
                    value=vmag_input
                    min="8"
                    max="18"
                    step="0.5"
                    on:input=move |ev| {
                        set_vmag_input.set(event_target_value(&ev));
                        on_params_change.call(build_params());
                    }
                    disabled=move || loading.get()
                />
                <span class="range-value">{vmag_input}</span>
            </div>

            <div class="button-group">
                <button
                    class="btn btn-primary"
                    on:click=move |_| {
                        on_params_change.call(build_params());
                        on_query.call(());
                    }
                    disabled=move || loading.get()
                >
                    {move || if loading.get() { "Querying..." } else { "Query Field" }}
                </button>

                <button
                    class="btn btn-secondary"
                    on:click=move |_| on_find_gs.call(())
                    disabled=move || loading.get()
                >
                    "Find Guide Stars"
                </button>

                <button
                    class="btn btn-toggle"
                    class:active=move || time_running.get()
                    on:click=move |_| on_toggle_time.call(())
                >
                    {move || if time_running.get() { "⏸ Pause Time" } else { "▶ Start Time" }}
                </button>
            </div>

            <div class="info-box">
                <h3>"Configuration"</h3>
                <ul>
                    <li>"Search radius: 10 arcmin"</li>
                    <li>"Exclude radius: 3 arcmin"</li>
                    <li>"Wavelength: 715 nm"</li>
                    <li>"Exposure: TT7=5ms, SH=30s"</li>
                </ul>
            </div>
        </div>
    }
}
