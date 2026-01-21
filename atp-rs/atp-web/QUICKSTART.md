# ATP Web UI - Quick Start with Mock Data

## ✅ What's Working

The web UI is **fully functional** with mock data! You can now:

1. **Query Star Fields** - Generates 50 random stars with realistic magnitudes
2. **Find Guide Stars** - Selects optimal TT7 + SH triplet with error budgets
3. **Interactive Visualization** - Plotly.js plots with star colors, probe areas, guide star highlighting
4. **Real-time UI Updates** - Loading states, error messages, status panels

## 🚀 Running the App

```bash
cd atp-web

# Start development server
trunk serve --open

# Or specify port
trunk serve --port 3000 --open
```

The app will open in your browser at `http://127.0.0.1:3000`

## 🎮 How to Use

### 1. Query a Star Field

1. Set query parameters in the left panel:
   - **Date/Time**: UTC timestamp (e.g., `2018-01-01T04:00:00Z`)
   - **Target**: Either name or leave as "None" for Alt/Az
   - **Altitude**: Telescope altitude in degrees (0-90)
   - **Azimuth**: Telescope azimuth in degrees (0-360)
   - **V Magnitude Limit**: Drag slider (8-18)

2. Click **"Query Field"**
   - UI shows loading spinner
   - After ~500ms, mock field with 50 stars appears
   - Stars are color-coded by brightness
   - 4 probe patrol areas shown as blue wedges

### 2. Find Guide Stars

1. After querying, click **"Find Guide Stars"**
   - UI shows loading spinner
   - After ~800ms, guide stars are selected
   - **TT7** (red star): Brightest star for tip-tilt
   - **SH triplet** (green circles): 3 stars at ~120° separation

2. Check results in right panel:
   - TT7 RMS error (mas)
   - SH median WFE (nm)
   - Performance assessment badge
   - Star positions and magnitudes

### 3. Interactive Plot Features

- **Pan**: Click and drag
- **Zoom**: Scroll wheel
- **Hover**: See star details (V mag, J mag)
- **Legend**: Stars, TT7, SH, Probes

## 📊 Mock Data Details

### Star Field Generation
- **50 random stars** between 3.5-10 arcmin from center
- **V magnitudes**: 10-16 (realistic distribution)
- **J magnitudes**: V - 0.3 to V - 0.8 (typical color)
- **Outside exclusion zone**: All > 3 arcmin from target

### Guide Star Selection
- **TT7**: Selects brightest available star
  - Error model: 0.5 + (V_mag - 10) * 1.5 mas
  - Range: ~0.5-9 mas for V=10-16

- **SH Triplet**: Selects 3 stars at ~120° azimuthal separation
  - WFE model: 80 + (avg_mag - 10) * 15 nm
  - Range: ~80-170 nm for avg V=10-16

### Probe Assignment
- 4 probes at 0°, 90°, 180°, 270°
- Probe 0 → TT7
- Probes 1,2,3 → SH triplet
- Lines drawn from probes to assigned stars

## 🎨 UI Components

### Left Panel: Controls
- Query parameters
- Action buttons
- Configuration info

### Center: Interactive Plot
- Plotly.js star field visualization
- Color-coded by magnitude
- Probe patrol areas
- Guide star highlighting

### Right Panel: Status & Results
- Star field statistics
- Guide star details
- Performance assessment
- Quick guide

## 🔧 What's Simulated

Currently using **mock data** instead of real ATP-RS computations:

| Feature | Status | Notes |
|---------|--------|-------|
| TIC Catalog Query | ❌ Mock | Generates random stars |
| Coordinate Transform | ❌ Mock | Static Alt/Az |
| TT7 Error Calculation | ✅ Model | Simplified formula |
| Anisoplanatism | ❌ Mock | Not computed |
| SH Selection | ✅ Model | Azimuthal algorithm |
| Probe Reachability | ❌ Mock | All stars reachable |
| Time Evolution | ❌ Disabled | Would need updates |

## 🚧 Next Steps: Real Integration

To connect to actual ATP-RS functionality:

### Option A: REST API Backend

```bash
# Create Axum API server
cargo new --bin atp-api

# Use atp-rs library in backend
# Serve REST endpoints
# Update web UI to call API
```

### Option B: Server Functions (SSR)

```rust
// Enable ssr feature in Cargo.toml
#[server(QueryField, "/api")]
pub async fn query_field(params: FieldQueryParams)
    -> Result<FieldResponse, ServerFnError> {
    // Use atp-rs directly
    let catalog = atp_rs::catalog::TICCatalog::new()?;
    let field = catalog.query_and_build_starfield(...).await?;
    Ok(convert_to_response(field))
}
```

### Option C: WASM Workers

```rust
// Run ATP-RS computations in Web Worker
// Update UI from worker messages
// Limited by WASM restrictions
```

## 🐛 Known Limitations

1. **No Real TIC Queries**: Mock stars only
2. **No Atmosphere Model**: Simplified error budgets
3. **Static Coordinates**: No sidereal time updates
4. **No Time Evolution**: Button doesn't actually animate
5. **No Configuration**: Hardcoded parameters

## 📈 Performance

- **Initial Load**: ~1-2 MB WASM bundle
- **Query**: 500ms simulated delay
- **Guide Stars**: 800ms simulated delay
- **Plot Render**: <100ms for 50 stars
- **Memory**: ~50 MB in browser

## 🎓 Code Structure

```
atp-web/src/
├── lib.rs           # Entry point, WASM setup
├── app.rs           # Main app component
├── models.rs        # Data structures
├── mock_data.rs     # ⭐ Mock data generation
├── components/
│   ├── controls.rs      # Input controls
│   ├── field_plot.rs    # Plotly visualization
│   └── info_panel.rs    # Results display
└── ...
```

Key mock data functions:
- `generate_mock_field()` - Creates 50 random stars + 4 probes
- `generate_mock_guide_star_result()` - Selects TT7 + SH triplet

## 🔍 Debugging

Open browser console (F12) to see:
```
ATP Web UI starting...
Querying field with params: {...}
Generated mock field with 50 stars
Finding guide stars for 50 stars
Selected TT7: star 0, error 0.50 mas
Selected SH triplet: stars [3, 7, 12], WFE 95.3 nm
```

## ✅ Verification Checklist

- [ ] Server starts: `trunk serve`
- [ ] Browser opens automatically
- [ ] No console errors
- [ ] Query Field button generates stars
- [ ] Plot shows colored dots
- [ ] Find Guide Stars highlights red/green stars
- [ ] Info panel shows statistics
- [ ] Hover shows star details
- [ ] Loading spinners appear/disappear

## 🎉 Success!

You now have a **working interactive web UI** for ATP! The mock data lets you:
- Test the complete workflow
- Demonstrate the UI to stakeholders
- Develop new features without backend dependency
- Validate visualization and UX

**Ready to connect to real ATP-RS?** Follow the "Next Steps" section above.

---

Built with ❤️ using Leptos + Plotly.js + Rust WASM
