# ATP Web UI

**Interactive Web Interface for AGWS Target Practice** - Built with Leptos + Plotly.js

A modern, full-Rust web application for interactive guide star selection, featuring real-time star field visualization, TIC catalog queries, and AGWS performance analysis.

## Features

- ✅ **Full Rust Stack**: Leptos framework compiled to WASM
- ✅ **Interactive Plotting**: Plotly.js for scientific visualization
- ✅ **Real-Time Updates**: Reactive UI with time evolution
- ✅ **TIC Catalog Integration**: Query MAST TIC catalog directly from browser
- ✅ **Guide Star Selection**: Complete TT7 + SH triplet workflow
- ✅ **Responsive Design**: Works on desktop and tablet
- ✅ **Type-Safe**: Full compile-time type checking

## Architecture

```
atp-web/
├── src/
│   ├── main.rs              # Entry point
│   ├── app.rs               # Main application component
│   ├── models.rs            # Shared data structures
│   ├── components/          # UI components
│   │   ├── controls.rs      # Input controls panel
│   │   ├── field_plot.rs    # Star field Plotly visualization
│   │   └── info_panel.rs    # Status and results display
│   └── server/              # Server functions (future)
├── style/
│   └── main.css             # Application styles
└── index.html               # HTML template
```

## Prerequisites

### System Requirements

- **Rust** 1.70+ (with wasm32-unknown-unknown target)
- **Node.js** 18+ (for development tools)
- **Trunk** (Rust WASM bundler)

### Installation

```bash
# Install wasm target
rustup target add wasm32-unknown-unknown

# Install Trunk
cargo install --locked trunk

# Install wasm-bindgen-cli (matching version in Cargo.lock)
cargo install wasm-bindgen-cli --version 0.2.89
```

## Development

### Quick Start

```bash
cd atp-web

# Run development server (with hot reload)
trunk serve --open

# Or specify port
trunk serve --port 8080 --open
```

The development server will:
- Compile Rust to WASM
- Bundle assets and inject into HTML
- Start local server at `http://127.0.0.1:8080`
- Watch for changes and hot reload

### Project Structure

**Client-Side Rendering (CSR)** - Default mode:
```bash
# Development
trunk serve

# Production build
trunk build --release
```

**Server-Side Rendering (SSR)** - Future enhancement:
```bash
cargo build --release --features ssr
./target/release/atp-web
```

## Building for Production

```bash
# Build optimized WASM bundle
trunk build --release

# Output in: dist/
#   - index.html
#   - atp-web-*.wasm
#   - atp-web-*.js
#   - main.css
```

### Deployment Options

#### 1. Static Hosting (Netlify/Vercel/GitHub Pages)

```bash
# Build and deploy dist/ folder
trunk build --release
# Upload dist/ to your hosting provider
```

#### 2. Docker Container

```dockerfile
FROM rust:1.75 as builder
RUN rustup target add wasm32-unknown-unknown
RUN cargo install trunk
WORKDIR /app
COPY . .
RUN cd atp-web && trunk build --release

FROM nginx:alpine
COPY --from=builder /app/atp-web/dist /usr/share/nginx/html
EXPOSE 80
CMD ["nginx", "-g", "daemon off;"]
```

#### 3. Self-Hosted with Caddy

```caddyfile
atp.example.com {
    root * /var/www/atp-web/dist
    file_server
    try_files {path} /index.html
    encode gzip
}
```

## Usage

### 1. Query Star Field

1. Enter **Date/Time** (UTC) or use default
2. Set **Target** coordinates or name (e.g., "M45" or "(56.75,24.12)")
3. Adjust **Altitude/Azimuth** for telescope pointing
4. Set **V Magnitude Limit** (8-18)
5. Click **Query Field**

The application will:
- Query TIC catalog via MAST API
- Filter by magnitude and exclusion radius
- Transform to local coordinates
- Display in interactive plot

### 2. Find Guide Stars

1. After querying field, click **Find Guide Stars**
2. Algorithm selects:
   - **TT7**: Optimal guide star for tip-tilt correction
   - **SH Triplet**: Three stars at ~120° separation for wavefront sensing
3. Results shown in:
   - Plot: Red (TT7), Green (SH)
   - Info panel: Error budgets, positions, performance

### 3. Time Evolution

1. Click **Start Time** to animate field rotation
2. Field updates at time resolution (default: 60s)
3. Click **Pause Time** to stop
4. Useful for checking probe reachability over time

## UI Components

### Controls Panel (Left)

- **Query Parameters**
  - Date/time input
  - Time resolution
  - Target name/coordinates
  - Alt/Az pointing
  - V magnitude limit slider

- **Action Buttons**
  - Query Field
  - Find Guide Stars
  - Start/Pause Time

- **Configuration Info**
  - Search/exclude radius
  - Wavelength
  - Exposure times

### Star Field Plot (Center)

- **Interactive Features**
  - Pan/zoom with mouse
  - Hover for star details
  - Color-coded by V magnitude

- **Elements**
  - Stars (colored dots)
  - Probes (wedge patrol areas)
  - TT7 guide star (red star)
  - SH guide stars (green circles)
  - Exclusion zones (dashed circles)

### Info Panel (Right)

- **Star Field Statistics**
  - Number of stars
  - Magnitude range
  - Observatory time/LST
  - Target coordinates

- **Guide Star Results**
  - TT7 error (mas)
  - SH WFE (nm RMS)
  - Star positions and magnitudes
  - Probe assignments

- **Performance Assessment**
  - Quality rating
  - Quick guide

## Customization

### Styling

Edit `style/main.css` to customize:
- Color scheme (CSS variables in `:root`)
- Layout (grid template)
- Component styles

### Configuration

Currently hardcoded. To make configurable:

1. Add settings panel component
2. Use Leptos `LocalStorage` for persistence
3. Pass config to server functions

### API Integration

For SSR mode, implement server functions:

```rust
// src/server/atp.rs
#[server(QueryField, "/api")]
pub async fn query_field(
    params: FieldQueryParams
) -> Result<FieldResponse, ServerFnError> {
    // Use atp-rs library
    let catalog = TICCatalog::new(Some(60))?;
    // ...
    Ok(response)
}
```

Then in components:
```rust
let query_field = create_server_action::<QueryField>();
query_field.dispatch(params);
```

## Performance

### WASM Bundle Size

- **Debug**: ~5-8 MB
- **Release**: ~800 KB - 1.2 MB (gzipped: ~300 KB)

### Optimization Tips

1. **Enable LTO** (already in Cargo.toml):
   ```toml
   [profile.wasm-release]
   lto = true
   opt-level = 'z'
   ```

2. **Use `wasm-opt`**:
   ```bash
   wasm-opt -Oz dist/*.wasm -o dist/optimized.wasm
   ```

3. **Lazy loading**:
   - Split large components
   - Use Leptos `<Suspense>` for async data

## Troubleshooting

### WASM Build Fails

```bash
# Clean and rebuild
cargo clean
trunk clean
trunk build
```

### Plotly Not Loading

Check browser console for CDN errors. If blocked:
1. Download plotly.js locally
2. Place in `public/` folder
3. Update `index.html` to use local file

### Hot Reload Not Working

```bash
# Ensure trunk is latest
cargo install --locked trunk --force

# Check port not in use
lsof -i :8080
```

### Type Errors with WASM Bindings

```bash
# Ensure wasm-bindgen versions match
cargo update wasm-bindgen
```

## Roadmap

- [ ] **SSR Mode**: Server-side rendering with Axum
- [ ] **Authentication**: User accounts and saved configurations
- [ ] **Batch Processing**: Multiple targets in parallel
- [ ] **Export**: PDF reports, CSV data tables
- [ ] **3D Visualization**: WebGL view of GMT+probes
- [ ] **Advanced Plotting**: D3.js custom visualizations
- [ ] **PWA**: Offline support, installable app
- [ ] **WebSocket**: Real-time collaboration

## Contributing

This web UI is part of the ATP-RS project. To contribute:

1. UI improvements: Edit components in `src/components/`
2. New features: Add to `app.rs` or create new components
3. Styling: Modify `style/main.css`
4. Testing: Add integration tests (future)

## License

Same as ATP-RS (check with GMT project)

## References

- **Leptos**: https://leptos.dev
- **Trunk**: https://trunkrs.dev
- **Plotly.js**: https://plotly.com/javascript/
- **ATP-RS**: https://github.com/rconan/atp-rs

---

**Status:** Prototype ready for development ✅

Built with ❤️ in Rust for the Giant Magellan Telescope
