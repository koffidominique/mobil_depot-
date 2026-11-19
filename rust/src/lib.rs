//! DEPOTPRO MOBILE PREMIUM — moteur métier Rust partagé Android/iOS.
//! Interface C stable consommée par le TurboModule React Native.

mod commands;
mod db;
mod domain;

use domain::{field_str, field_str_opt, CoreError, CoreResult};
use rusqlite::{params, Connection};
use serde_json::{json, Value};
use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::sync::{Mutex, OnceLock};

static DB_STATE: OnceLock<Mutex<Option<Connection>>> = OnceLock::new();
static DB_PATH: OnceLock<Mutex<String>> = OnceLock::new();

fn state() -> &'static Mutex<Option<Connection>> {
    DB_STATE.get_or_init(|| Mutex::new(None))
}

fn execute_inner(input: &str) -> CoreResult<String> {
    let request: Value = serde_json::from_str(input)?;
    let command = field_str(&request, "command")?;
    let payload = request.get("payload").cloned().unwrap_or_else(|| json!({}));

    if command == "initialize_database" || command == "initializeDatabase" {
        let path = field_str(&payload, "path")?;
        let conn = db::open(path)?;
        let mut guard = state().lock().map_err(|_| CoreError::new("STATE_POISONED", "État SQLite verrouillé."))?;
        *guard = Some(conn);
        let mut path_guard = DB_PATH.get_or_init(|| Mutex::new(String::new())).lock()
            .map_err(|_| CoreError::new("STATE_POISONED", "Chemin SQLite verrouillé."))?;
        *path_guard = path.to_string();
        return Ok(json!({"ok": true, "data": {"database": path, "schema_version": 5}}).to_string());
    }

    if command == "backupDatabase" || command == "backup_database" {
        let mut guard = state().lock().map_err(|_| CoreError::new("STATE_POISONED", "État SQLite verrouillé."))?;
        let conn = guard
            .as_mut()
            .ok_or_else(|| CoreError::new("DB_NOT_INITIALIZED", "initializeDatabase doit être appelé en premier."))?;
        let current = DB_PATH.get_or_init(|| Mutex::new(String::new())).lock()
            .map_err(|_| CoreError::new("STATE_POISONED", "Chemin SQLite verrouillé."))?.clone();
        let destination = field_str_opt(&payload, "destination_path").map(str::to_string)
            .unwrap_or_else(|| format!("{current}.backup-{}", db::now_ms()));
        conn.execute("VACUUM INTO ?1", params![destination])?;
        return Ok(json!({"ok": true, "data": {"backup_path": destination, "bytes": std::fs::metadata(&destination)
            .map(|m| m.len() as i64).unwrap_or(0)}}).to_string());
    }

    if command == "restoreDatabase" || command == "restore_database" {
        let source = field_str(&payload, "source_path")?.to_string();
        let current = DB_PATH.get_or_init(|| Mutex::new(String::new())).lock()
            .map_err(|_| CoreError::new("STATE_POISONED", "Chemin SQLite verrouillé."))?.clone();
        if current.is_empty() {
            return Err(CoreError::new("DB_NOT_INITIALIZED", "initializeDatabase doit être appelé en premier."));
        }
        if !std::path::Path::new(&source).is_file() {
            return Err(CoreError::new("BACKUP_NOT_FOUND", "Fichier de sauvegarde introuvable."));
        }
        let safety = format!("{current}.before-restore-{}", db::now_ms());
        {
            let mut guard = state().lock().map_err(|_| CoreError::new("STATE_POISONED", "État SQLite verrouillé."))?;
            let conn = guard.as_mut().ok_or_else(|| CoreError::new("DB_NOT_INITIALIZED", "Base absente."))?;
            conn.execute("VACUUM INTO ?1", params![safety])?;
            *guard = None;
        }
        std::fs::copy(&source, &current).map_err(|e| CoreError::new("RESTORE_FAILED", e.to_string()))?;
        let reopened = db::open(&current)?;
        let mut guard = state().lock().map_err(|_| CoreError::new("STATE_POISONED", "État SQLite verrouillé."))?;
        *guard = Some(reopened);
        return Ok(json!({"ok": true, "data": {"restored_from": source, "safety_backup": safety}}).to_string());
    }

    let mut guard = state().lock().map_err(|_| CoreError::new("STATE_POISONED", "État SQLite verrouillé."))?;
    let conn = guard
        .as_mut()
        .ok_or_else(|| CoreError::new("DB_NOT_INITIALIZED", "initializeDatabase doit être appelé en premier."))?;
    let data = commands::dispatch(conn, command, &payload)?;
    Ok(json!({"ok": true, "data": data}).to_string())
}

fn error_response(error: CoreError) -> String {
    json!({"ok": false, "error": {"code": error.code, "message": error.message}}).to_string()
}

/// Exécute une commande métier. Le pointeur retourné doit être libéré avec `depot_free`.
///
/// # Safety
/// `input` doit être une chaîne C valide terminée par un NUL.
#[no_mangle]
pub unsafe extern "C" fn depot_execute(input: *const c_char) -> *mut c_char {
    let result = if input.is_null() {
        Err(CoreError::new("NULL_INPUT", "Entrée JSON absente."))
    } else {
        let raw = CStr::from_ptr(input).to_str().map_err(|_| CoreError::new("INVALID_UTF8", "Entrée UTF-8 invalide."));
        match raw {
            Ok(text) => execute_inner(text),
            Err(err) => Err(err),
        }
    };
    let json = match result {
        Ok(value) => value,
        Err(err) => error_response(err),
    };
    CString::new(json).map(|s| s.into_raw()).unwrap_or_else(|_| CString::new(error_response(CoreError::new("NUL_IN_OUTPUT", "Caractère NUL dans la réponse."))).unwrap().into_raw())
}

/// Libère une chaîne retournée par `depot_execute`.
///
/// # Safety
/// `ptr` doit avoir été obtenu depuis `depot_execute` et utilisé une seule fois.
#[no_mangle]
pub unsafe extern "C" fn depot_free(ptr: *mut c_char) {
    if !ptr.is_null() {
        drop(CString::from_raw(ptr));
    }
}

/// Version du moteur, utile pour le contrôle de compatibilité `schema_version`.
#[no_mangle]
pub extern "C" fn depot_version() -> *const c_char {
    concat!(env!("CARGO_PKG_VERSION"), "\0").as_ptr() as *const c_char
}

#[cfg(test)]
mod tests {
    use super::domain::*;
    use serde_json::json;

    #[test]
    fn mixed_case_rejects_incompatible_physical_profiles() {
        let payload = json!({
            "crate_type_id": "crate_b12_50",
            "is_half_case": false,
            "lines": [
                {"variant_id": "a", "variant_name": "A", "quantity_bottles": 6, "unit_price_fcfa": 600,
                 "compatibility_group_id": "G_B12_50", "bottle_profile_id": "bp_a", "is_reusable": true},
                {"variant_id": "b", "variant_name": "B", "quantity_bottles": 6, "unit_price_fcfa": 600,
                 "compatibility_group_id": "G_B12_50_ALT", "bottle_profile_id": "bp_b", "is_reusable": true}
            ]
        });
        let err = validate_mixed_case(&payload).unwrap_err();
        assert_eq!(err.code, "INCOMPATIBLE_CRATE_PROFILE");
    }

    #[test]
    fn sale_rejects_credit_above_limit() {
        let payload = json!({
            "role": "MANAGER", "offline_policy": "ALLOW_WITH_WARNING",
            "customer_id": "cust_1", "credit_limit_fcfa": 15000, "outstanding_balance_fcfa": 8000,
            "lines": [{"line_key": "l1", "variant_id": "v1", "unit": "BOTTLE", "unit_count": 10,
                       "unit_price_fcfa": 1000, "crate_capacity": 12, "pack_size": 12,
                       "allow_half_case": true, "available_stock_units": 100}],
            "mixed_cases": [], "discount_fcfa": 0,
            "payments": [{"method": "CASH", "amount_fcfa": 2000, "reference": null, "confirmed_by_provider": true}]
        });
        let err = validate_sale(&payload).unwrap_err();
        assert_eq!(err.code, "CREDIT_LIMIT_EXCEEDED");
    }
}

/// Pont JNI Android : le TurboModule Kotlin appelle directement cette fonction.
#[cfg(target_os = "android")]
mod android_jni {
    use jni::objects::{JClass, JString};
    use jni::sys::jstring;
    use jni::JNIEnv;

    #[no_mangle]
    pub extern "system" fn Java_com_depotpro_mobile_DepotCoreModule_nativeExecute(
        env: JNIEnv,
        _class: JClass,
        input: JString,
    ) -> jstring {
        let request: String = env
            .get_string(&input)
            .map(|s| s.into())
            .unwrap_or_else(|_| "{\"command\":\"\"}".to_string());
        let response = match super::execute_inner(&request) {
            Ok(value) => value,
            Err(err) => super::error_response(err),
        };
        env.new_string(response)
            .map(|s| s.into_inner())
            .unwrap_or(std::ptr::null_mut())
    }
}
