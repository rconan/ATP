//! Main ATP Web UI application

use leptos::*;
use leptos_meta::*;
use leptos_router::*;

use crate::components::{Controls, FieldPlot, InfoPanel};
use crate::models::{FieldQueryParams, FieldResponse, GuideStarResult};
use crate::mock_data::{generate_mock_field, generate_mock_guide_star_result};

/// Main ATP application component
#[component]
pub fn App() -> impl IntoView {
    provide_meta_context();

    view! {
        <Stylesheet id="leptos" href="/pkg/atp-web.css"/>
        <Title text="ATP-RS: AGWS Target Practice"/>
        <Meta name="description" content="Interactive guide star selection for GMT AGWS"/>

        <Router>
            <main class="atp-app">
                <Routes>
                    <Route path="" view=HomePage/>
                </Routes>
            </main>
        </Router>
    }
}

/// Home page with star field viewer
#[component]
fn HomePage() -> impl IntoView {
    // Application state
    let (query_params, set_query_params) = create_signal(FieldQueryParams::default());
    let (field_data, set_field_data) = create_signal::<Option<FieldResponse>>(None);
    let (gs_result, set_gs_result) = create_signal::<Option<GuideStarResult>>(None);
    let (loading, set_loading) = create_signal(false);
    let (error_msg, set_error_msg) = create_signal::<Option<String>>(None);
    let (time_running, set_time_running) = create_signal(false);

    // Prevent unused warnings for setters used in closures
    let _ = (set_query_params, set_field_data, set_gs_result, set_loading, set_error_msg, set_time_running);

    // Query star field
    let query_field = create_action(move |params: &FieldQueryParams| {
        let params = params.clone();
        async move {
            set_loading.set(true);
            set_error_msg.set(None);
            set_gs_result.set(None); // Clear previous results

            log::info!("Querying field with params: {:?}", params);

            // Simulate network delay
            gloo_timers::future::TimeoutFuture::new(500).await;

            // Generate mock star field
            let mock_field = generate_mock_field();
            log::info!("Generated mock field with {} stars", mock_field.stars.len());

            set_field_data.set(Some(mock_field));
            set_loading.set(false);
        }
    });

    // Find guide stars
    let find_guide_stars = create_action(move |_: &()| async move {
        set_loading.set(true);
        set_error_msg.set(None);

        if let Some(field) = field_data.get() {
            log::info!("Finding guide stars for {} stars", field.stars.len());

            // Simulate computation time
            gloo_timers::future::TimeoutFuture::new(800).await;

            // Generate mock guide star selection
            let result = generate_mock_guide_star_result(&field);
            log::info!(
                "Selected TT7: star {}, error {:.2} mas",
                result.tt7_star_idx,
                result.tt7_error_mas
            );
            log::info!(
                "Selected SH triplet: stars {:?}, WFE {:.1} nm",
                result.sh_star_indices,
                result.sh_wfe_nm
            );

            set_field_data.set(Some(result.field.clone()));
            set_gs_result.set(Some(result));
        } else {
            set_error_msg.set(Some("No star field loaded. Query field first.".to_string()));
        }

        set_loading.set(false);
    });

    // Time evolution toggle
    let toggle_time = move |_| {
        set_time_running.update(|running| *running = !*running);

        if time_running.get() {
            log::info!("Starting time evolution");
            // TODO: Set up interval to update field
        } else {
            log::info!("Stopping time evolution");
        }
    };

    view! {
        <div class="container">
            <header class="header">
                <h1>"ATP-RS: AGWS Target Practice"</h1>
                <p class="subtitle">"Interactive Guide Star Selection for GMT"</p>
            </header>

            <div class="main-content">
                // Left panel: Controls
                <aside class="controls-panel">
                    <Controls
                        params=query_params
                        on_params_change=Callback::new(move |new_params| {
                            set_query_params.set(new_params);
                        })
                        on_query=Callback::new(move |_| {
                            query_field.dispatch(query_params.get());
                        })
                        on_find_gs=Callback::new(move |_| {
                            find_guide_stars.dispatch(());
                        })
                        on_toggle_time=Callback::new(toggle_time)
                        time_running=time_running
                        loading=loading
                    />
                </aside>

                // Center: Star field plot
                <section class="plot-area">
                    <FieldPlot
                        field=field_data
                        gs_result=gs_result
                        loading=loading
                    />
                </section>

                // Right panel: Info
                <aside class="info-panel">
                    <InfoPanel
                        field=field_data
                        gs_result=gs_result
                        error=error_msg
                    />
                </aside>
            </div>

            {move || error_msg.get().map(|err| view! {
                <div class="error-banner">
                    <strong>"Error: "</strong> {err}
                </div>
            })}
        </div>
    }
}
