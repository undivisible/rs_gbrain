# unthinkclaw integration

```bash
cd /path/to/unthinkclaw
ln -sf ../rs_gbrain vendor/rs_gbrain
```

In `Cargo.toml`:

```toml
rs_gbrain = { path = "../rs_gbrain", optional = true }

[features]
rs-gbrain = ["dep:rs_gbrain"]
```

Register tools that call `BrainEngine::open_default()` instead of shelling out to Bun gbrain.