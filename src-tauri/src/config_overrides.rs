use serde_json::Value;
use std::env;
use std::fs;
use std::path::PathBuf;
use std::sync::RwLock;
use tauri::{AppHandle, Emitter, Manager};

use once_cell::sync::Lazy;

// Global in-memory overrides (not persisted). When present, merged on top of stored settings.
pub static GLOBAL_OVERRIDES: Lazy<RwLock<Option<Value>>> = Lazy::new(|| RwLock::new(None));

fn deep_merge(dst: &mut Value, src: &Value) {
    match (dst, src) {
        (Value::Object(dmap), Value::Object(smap)) => {
            for (k, v) in smap {
                match dmap.get_mut(k) {
                    Some(existing) => deep_merge(existing, v),
                    None => {
                        dmap.insert(k.clone(), v.clone());
                    }
                }
            }
        }
        // For arrays/scalars: replace
        (d, s) => {
            *d = s.clone();
        }
    }
}

/// Return the first override file path if it exists.
pub fn resolve_override_path(app: &AppHandle) -> Option<PathBuf> {
    // 1) Environment variable
    if let Ok(p) = env::var("ULTRAWHISPER_CONFIG") {
        let path = PathBuf::from(p);
        if path.exists() {
            return Some(path);
        }
    }

    // 2) ~/.ultrawhisperrc.json
    if let Ok(home) = env::var("HOME").or_else(|_| env::var("USERPROFILE")) {
        let mut p = PathBuf::from(home);
        p.push(".ultrawhisperrc.json");
        if p.exists() {
            return Some(p);
        }
    }

    // 3) <app_data>/ultrawhisper/config.json
    if let Ok(mut app_dir) = app.path().app_data_dir() {
        app_dir.push("ultrawhisper");
        app_dir.push("config.json");
        if app_dir.exists() {
            return Some(app_dir);
        }
    }

    None
}

/// Load overrides JSON from disk.
fn load_overrides_file(path: &PathBuf) -> Option<Value> {
    match fs::read_to_string(path) {
        Ok(contents) => match serde_json::from_str::<Value>(&contents) {
            Ok(v) => Some(v),
            Err(e) => {
                log::warn!("Config override parse error for {:?}: {}", path, e);
                None
            }
        },
        Err(e) => {
            log::warn!("Config override read error for {:?}: {}", path, e);
            None
        }
    }
}

/// Apply overrides at startup. Emits `overrides-active` event with summary.
pub fn apply_startup_overrides(app: &AppHandle) {
    if let Some(path) = resolve_override_path(app) {
        if let Some(v) = load_overrides_file(&path) {
            {
                let mut guard = GLOBAL_OVERRIDES.write().expect("override lock");
                *guard = Some(v.clone());
            }
            // Log an explicit info line with path and affected keys
            let keys = collect_keys(&v);
            log::info!(
                "Config overrides active: path={:?} count={} keys={:?}",
                path,
                keys.len(),
                keys
            );
            emit_overrides_event(app, true, &v, Some(path));
            return;
        }
    }
    log::info!("Config overrides: none active");
    emit_overrides_event(app, false, &Value::Null, None);
}

fn emit_overrides_event(app: &AppHandle, active: bool, v: &Value, path: Option<PathBuf>) {
    let keys = collect_keys(v);
    let count = keys.len();
    let payload = serde_json::json!({
        "active": active,
        "keys": keys,
        "count": count,
        "path": path.map(|p| p.to_string_lossy().to_string()),
    });
    let _ = app.emit("overrides-active", payload);
}

fn collect_keys(v: &Value) -> Vec<String> {
    fn walk(prefix: &str, v: &Value, out: &mut Vec<String>) {
        match v {
            Value::Object(map) => {
                for (k, val) in map {
                    let key = if prefix.is_empty() {
                        k.clone()
                    } else {
                        format!("{}.{}", prefix, k)
                    };
                    walk(&key, val, out);
                }
            }
            _ => out.push(prefix.to_string()),
        }
    }
    let mut out = Vec::new();
    if v.is_object() {
        walk("", v, &mut out);
    }
    out
}

/// Merge overrides onto a settings JSON value in place.
pub fn merge_overrides(settings: &mut Value) {
    let guard = GLOBAL_OVERRIDES.read().expect("override lock");
    if let Some(overrides) = guard.as_ref() {
        deep_merge(settings, overrides);
    }
}

/// Reload overrides from disk at runtime and emit events.
pub fn reload_config_overrides(app: &AppHandle) {
    if let Some(path) = resolve_override_path(app) {
        let v_opt = load_overrides_file(&path);
        {
            let mut guard = GLOBAL_OVERRIDES.write().expect("override lock");
            *guard = v_opt.clone();
        }
        match v_opt {
            Some(v) => {
                let keys = collect_keys(&v);
                log::info!(
                    "Config overrides reloaded: path={:?} count={} keys={:?}",
                    path,
                    keys.len(),
                    keys
                );
                emit_overrides_event(app, true, &v, Some(path))
            }
            None => {
                log::info!("Config overrides reload: file present but invalid, overrides inactive: path={:?}", path);
                emit_overrides_event(app, false, &Value::Null, Some(path))
            }
        }
    } else {
        // Clear overrides if no file found
        {
            let mut guard = GLOBAL_OVERRIDES.write().expect("override lock");
            *guard = None;
        }
        log::info!("Config overrides reload: no file found, overrides cleared");
        emit_overrides_event(app, false, &Value::Null, None);
    }
}
