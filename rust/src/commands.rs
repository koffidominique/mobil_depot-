//! Exécution des commandes métier exposées au front React Native.

use crate::db::{self, now_ms};
use crate::domain::*;
use rusqlite::{params, Connection, OptionalExtension};
use serde_json::{json, Value};

#[derive(Debug, Clone)]
pub struct Actor {
    pub user_id: String,
    pub tenant_id: String,
    pub branch_id: String,
    pub device_id: String,
    pub role: String,
}

pub fn actor(conn: &Connection, payload: &Value) -> CoreResult<Actor> {
    let auth = payload.get("auth").unwrap_or(payload);
    let token = field_str(auth, "token")?;
    let (user_id, branch_id, device_id, role) = db::session_user(conn, token)?;
    let tenant_id: String = conn.query_row(
        "SELECT tenant_id FROM users WHERE id = ?1",
        params![user_id],
        |r| r.get(0),
    )?;
    Ok(Actor { user_id, tenant_id, branch_id, device_id, role })
}

#[derive(Debug, Clone, Copy)]
struct SecuritySettings {
    session_duration_ms: i64,
    auto_lock_ms: i64,
    pin_required: bool,
    max_failed_attempts: i64,
    lockout_ms: i64,
    backup_encryption_required: bool,
}

fn security_settings(conn: &Connection, tenant: &str, branch: &str) -> CoreResult<SecuritySettings> {
    let read = |scope: &str| -> CoreResult<Option<SecuritySettings>> {
        conn.query_row(
            "SELECT session_duration_ms, auto_lock_ms, pin_required, max_failed_attempts,
                    lockout_ms, backup_encryption_required
             FROM security_settings WHERE tenant_id=?1 AND branch_id=?2",
            params![tenant, scope],
            |row| Ok(SecuritySettings {
                session_duration_ms: row.get(0)?,
                auto_lock_ms: row.get(1)?,
                pin_required: row.get::<_, i64>(2)? == 1,
                max_failed_attempts: row.get(3)?,
                lockout_ms: row.get(4)?,
                backup_encryption_required: row.get::<_, i64>(5)? == 1,
            }),
        )
        .optional()
        .map_err(CoreError::from)
    };
    if let Some(settings) = read(branch)? {
        return Ok(settings);
    }
    if let Some(settings) = read("*")? {
        return Ok(settings);
    }
    Ok(SecuritySettings {
        session_duration_ms: 8 * 60 * 60 * 1000,
        auto_lock_ms: 5 * 60 * 1000,
        pin_required: true,
        max_failed_attempts: 5,
        lockout_ms: 15 * 60 * 1000,
        backup_encryption_required: true,
    })
}

/// Droits configurables par établissement puis point de vente, avec repli `*`.
pub fn has_permission(
    conn: &Connection,
    tenant: &str,
    branch: &str,
    role: &str,
    permission: &str,
) -> CoreResult<bool> {
    let allowed: Option<i64> = conn
        .query_row(
            "SELECT allowed FROM role_permissions
             WHERE tenant_id=?1 AND role=?2 AND permission=?3 AND branch_id IN (?4,'*')
             ORDER BY CASE WHEN branch_id=?4 THEN 1 ELSE 0 END DESC LIMIT 1",
            params![tenant, role, permission, branch],
            |row| row.get(0),
        )
        .optional()?;
    Ok(allowed.unwrap_or(0) == 1)
}

pub fn check_permission(conn: &Connection, payload: &Value, permission: &str) -> CoreResult<bool> {
    let a = actor(conn, payload)?;
    has_permission(conn, &a.tenant_id, &a.branch_id, &a.role, permission)
}

fn assert_permission(conn: &Connection, a: &Actor, permission: &str) -> CoreResult<()> {
    if has_permission(conn, &a.tenant_id, &a.branch_id, &a.role, permission)? {
        Ok(())
    } else {
        let _ = db::audit_security(
            conn, &a.branch_id, &a.user_id, Some(&a.device_id),
            "PERMISSION_DENIED", permission, "WARN", "AUTHORIZATION",
        );
        Err(CoreError::new("PERMISSION_DENIED", "Droit insuffisant pour cette commande."))
    }
}

/// Contrôle central : aucune commande sensible n'atteint le métier sans ce filtre.
fn permission_for_command(command: &str) -> Option<&'static str> {
    match command {
        "authenticate" | "authenticateUser" | "initializeDatabase" | "initialize_database"
        | "renewSession" | "revokeSession" | "lockSession" | "unlockSession" | "changePin" => None,
        "getSecurityPolicy" | "checkPermission" | "getDashboard" | "get_dashboard" | "getInventory" | "get_inventory"
        | "getCustomers" | "get_customers" | "getSuppliers" | "get_suppliers" | "getSales"
        | "get_sales" | "getCustomerAccount" | "get_customer_account" | "getSupplierAccount"
        | "get_supplier_account" | "getStockMovements" | "get_stock_movements"
        | "getPurchaseVariance" | "get_purchase_variance" | "getPurchaseOrders"
        | "get_purchase_orders" | "getSaleReceipt" | "get_sale_receipt" | "getCashSession"
        | "get_cash_session" | "getReports" | "get_reports" | "getConflicts" | "get_conflicts"
        | "getCrateTypes" | "get_crate_types" | "getPackagingStock" | "get_packaging_stock"
        | "getPartnerStatement" | "get_partner_statement" | "getPackagingAccount"
        | "get_packaging_account" | "getSyncHistory" | "get_sync_history"
        | "getCrateStock" | "get_crate_stock" | "getCrateStockByType" | "get_crate_stock_by_type"
        | "getCustomerCrateBalance" | "get_customer_crate_balance"
        | "getSupplierCrateBalance" | "get_supplier_crate_balance"
        | "getCrateMovementHistory" | "get_crate_movement_history"
        | "getCrateValuationHistory" | "get_crate_valuation_history"
        | "getCrateMigrationReview" | "get_crate_migration_review"
        | "getSynchronizationStatus" | "get_synchronization_status" | "getPendingOperations"
        | "get_pending_operations" | "startSynchronization" | "start_synchronization"
        | "backupDatabase" | "backup_database" | "getAuditLog" | "listBranches" | "list_branches"
        | "listStockTransfers" | "list_stock_transfers" | "exportDesktopSnapshot"
        | "export_desktop_snapshot" | "importDesktopUpdates" => Some("read"),
        "createSale" | "create_sale" | "validateSale" | "validate_sale" | "createMixedOrder"
        | "create_mixed_order" | "createCustomer" | "create_customer" => Some("sales.write"),
        "recordPayment" | "record_payment" | "recordCashMovement" | "record_cash_movement"
        | "openCashSession" | "open_cash_session" | "closeCashSession" | "close_cash_session" => Some("cash.write"),
        "createPurchaseOrder" | "create_purchase_order" | "receivePurchase" | "receive_purchase"
        | "recordSupplierReturn" | "record_supplier_return"
        | "recordSupplierCrateExchange" | "record_supplier_crate_exchange" => Some("purchase.write"),
        // Retours physiques de casiers : accessibles au comptoir.
        "registerPackagingReturn" | "register_packaging_return"
        | "recordCustomerCrateReturn" | "record_customer_crate_return" => Some("sales.write"),
        "recordCrateAdjustment" | "record_crate_adjustment"
        | "resolveCrateMigrationReview" | "resolve_crate_migration_review"
        | "correctPackagingMovement" | "correct_packaging_movement" => Some("crate.adjust"),
        "recordCrateLoss" | "record_crate_loss" => Some("crate.loss"),
        "recordEmptyCrateSale" | "record_empty_crate_sale" => Some("crate.sell"),
        "configureCrateValuation" | "configure_crate_valuation" => Some("catalog.write"),
        "createCatalogProducer" | "create_catalog_producer"
        | "updateCatalogProducer" | "update_catalog_producer"
        | "createCatalogCategory" | "create_catalog_category"
        | "createCatalogBrand" | "create_catalog_brand"
        | "createCatalogProduct" | "create_catalog_product"
        | "activateCatalogProduct" | "activate_catalog_product"
        | "deactivateCatalogProduct" | "deactivate_catalog_product"
        | "updateCatalogVariant" | "update_catalog_variant"
        | "createBreweryCatalog" | "create_brewery_catalog"
        | "updateBreweryCatalog" | "update_brewery_catalog" => Some("catalog.write"),
        "previewCatalogImport" | "preview_catalog_import"
        | "importCatalogJson" | "import_catalog_json" => Some("catalog.import"),
        "recordInventoryCount" | "record_inventory_count" | "createStockTransfer"
        | "create_stock_transfer" => Some("inventory.write"),
        "saveProductVariant" | "save_product_variant" | "updateVariantPrices"
        | "update_variant_prices" | "archiveProductVariant" | "archive_product_variant"
        | "createCrateType" | "create_crate_type" | "updateCrateType" | "update_crate_type"
        | "configureDeviceQuota" | "configure_device_quota" => Some("catalog.write"),
        "resolveConflict" | "resolve_conflict" => Some("conflict.resolve"),
        "setSecurityPolicy" | "setRolePermission" | "revokeDevice" | "configureSyncServer"
        | "configure_sync_server" => Some("security.admin"),
        _ => Some("read"),
    }
}

pub fn authenticate(conn: &mut Connection, payload: &Value) -> CoreResult<Value> {
    let username = field_str(payload, "username")?;
    let pin = field_str(payload, "pin")?;
    let branch_id = field_str(payload, "branch_id")?;
    let device_id = field_str(payload, "device_id").unwrap_or(db::DEVICE_ID);
    let settings = security_settings(conn, db::TENANT_ID, branch_id)?;
    let now = now_ms();

    let locked: i64 = conn
        .query_row(
            "SELECT locked_until FROM login_attempts WHERE username=?1 AND device_id=?2",
            params![username, device_id],
            |row| row.get(0),
        )
        .optional()?
        .unwrap_or(0);
    if locked > now {
        let _ = db::audit_security(
            conn, branch_id, username, Some(device_id),
            "LOGIN_THROTTLED", "Tentatives verrouillées", "WARN", "AUTHENTICATION",
        );
        return Err(CoreError::new(
            "TOO_MANY_ATTEMPTS",
            format!("Trop de tentatives. Réessayez dans {} ms.", locked - now),
        ));
    }

    let row: Option<(String, String, String, String, i64)> = conn
        .query_row(
            "SELECT id, tenant_id, full_name, role, is_active FROM users WHERE username = ?1",
            params![username],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)),
        )
        .optional()?;
    let (user_id, tenant_id, full_name, role, active) = row
        .ok_or_else(|| CoreError::new("INVALID_CREDENTIALS", "Identifiant ou code PIN incorrect."))?;
    if active != 1 {
        return Err(CoreError::new("USER_DISABLED", "Utilisateur désactivé."));
    }

    let branch_allowed: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM user_branch_access WHERE user_id=?1 AND branch_id=?2",
            params![user_id, branch_id],
            |row| row.get(0),
        )?;
    let has_any_access: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM user_branch_access WHERE user_id=?1",
            params![user_id],
            |row| row.get(0),
        )?;
    if has_any_access > 0 && branch_allowed == 0 {
        return Err(CoreError::new("BRANCH_FORBIDDEN", "Accès refusé pour ce point de vente."));
    }

    let device: Option<(i64, Option<i64>)> = conn
        .query_row(
            "SELECT is_authorized, revoked_at FROM devices WHERE id=?1",
            params![device_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()?;
    match device {
        Some((authorized, revoked_at)) if authorized == 1 && revoked_at.is_none() => {}
        _ => return Err(CoreError::new("DEVICE_REVOKED", "Appareil révoqué pour ce dépôt.")),
    }

    let mut valid_pin = db::verify_user_pin(conn, &user_id, pin)?;
    if !settings.pin_required && !valid_pin {
        valid_pin = db::verify_user_pin(conn, &user_id, "")?;
    }
    if !valid_pin {
        let failures: i64 = conn
            .query_row(
                "SELECT failures FROM login_attempts WHERE username=?1 AND device_id=?2",
                params![username, device_id],
                |row| row.get(0),
            )
            .optional()?
            .unwrap_or(0)
            + 1;
        let until = if failures >= settings.max_failed_attempts { now + settings.lockout_ms } else { 0 };
        conn.execute(
            "INSERT INTO login_attempts (username, device_id, failures, locked_until, updated_at)
             VALUES (?1,?2,?3,?4,?5)
             ON CONFLICT(username, device_id) DO UPDATE SET
               failures=excluded.failures, locked_until=excluded.locked_until, updated_at=excluded.updated_at",
            params![username, device_id, failures, until, now],
        )?;
        let _ = db::audit_security(
            conn, branch_id, &user_id, Some(device_id),
            "LOGIN_FAILED", "Code PIN incorrect", "WARN", "AUTHENTICATION",
        );
        return Err(CoreError::new("INVALID_CREDENTIALS", "Identifiant ou code PIN incorrect."));
    }

    // Migration opportuniste : une authentification réussie convertit un hash legacy.
    let kdf: String = conn.query_row("SELECT pin_kdf FROM users WHERE id=?1", params![user_id], |r| r.get(0))?;
    if kdf == "SHA256_LEGACY" {
        db::upgrade_user_pin(conn, &user_id, pin, None)?;
    }

    conn.execute(
        "DELETE FROM login_attempts WHERE username=?1 AND device_id=?2",
        params![username, device_id],
    )?;
    let token = new_id("tok");
    let expires = now + settings.session_duration_ms;
    conn.execute(
        "INSERT INTO sessions (token, user_id, branch_id, device_id, expires_at, created_at, locked, last_activity_at)
         VALUES (?1,?2,?3,?4,?5,?6,0,?6)",
        params![token, user_id, branch_id, device_id, expires, now],
    )?;
    conn.execute(
        "UPDATE devices SET last_seen_at=?1 WHERE id=?2",
        params![now, device_id],
    )?;
    db::audit_security(
        conn, branch_id, &user_id, Some(device_id),
        "LOGIN", "Connexion mobile", "INFO", "AUTHENTICATION",
    )?;

    Ok(json!({
        "token": token,
        "user_id": user_id,
        "tenant_id": tenant_id,
        "full_name": full_name,
        "role": role,
        "branch_id": branch_id,
        "device_id": device_id,
        "expires_at": expires,
        "pin_required": settings.pin_required,
        "auto_lock_ms": settings.auto_lock_ms
    }))
}

/// Enrichit le panier avec les données faisant autorité dans SQLite local :
/// rôle, politique hors ligne, solde crédit et stock connu.
fn prepare_sale(conn: &Connection, payload: &Value) -> CoreResult<Value> {
    let a = actor(conn, payload)?;
    let mut enriched = payload.clone();
    enriched["role"] = json!(a.role);
    let policy: String = conn.query_row(
        "SELECT offline_policy FROM tenants WHERE id = ?1",
        params![a.tenant_id],
        |r| r.get(0),
    )?;
    enriched["offline_policy"] = json!(policy);

    // Fiche de clientèle : une vente sans identifiant peut désigner un client
    // par son nom (et éventuellement son téléphone). On retrouve alors sa fiche
    // pour appliquer son plafond ; sinon le client sera créé à l'écriture.
    if field_str_opt(&enriched, "customer_id").is_none() {
        if let Some(raw_name) = field_str_opt(&enriched, "customer_name") {
            let name = display_name(raw_name)?;
            let phone = phone_key(field_str_opt(&enriched, "customer_phone"))?;
            match find_customer(conn, &a.tenant_id, &name, &phone)? {
                Some(id) => enriched["customer_id"] = json!(id),
                None => {
                    enriched["credit_limit_fcfa"] = json!(0);
                    enriched["outstanding_balance_fcfa"] = json!(0);
                }
            }
        }
    }

    if let Some(customer_id) = field_str_opt(&enriched, "customer_id") {
        let customer: Option<(i64, i64, i64)> = conn
            .query_row(
                "SELECT credit_limit_fcfa, financial_balance_fcfa, is_archived FROM customers WHERE id = ?1",
                params![customer_id],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .optional()?;
        let (limit, balance, archived) = customer
            .ok_or_else(|| CoreError::new("CUSTOMER_NOT_FOUND", "Client introuvable."))?;
        if archived == 1 {
            return Err(CoreError::new("CUSTOMER_ARCHIVED", "Ce client archivé ne peut plus acheter."));
        }
        enriched["credit_limit_fcfa"] = json!(limit);
        enriched["outstanding_balance_fcfa"] = json!(balance);
    }

    // Stock disponible par boisson : lignes normales ET lignes des casiers
    // mixtes, pour que les alertes de stock portent sur la vraie référence.
    if let Some(cases) = enriched["mixed_cases"].as_array_mut() {
        for case in cases.iter_mut() {
            if let Some(lines) = case.get_mut("lines").and_then(Value::as_array_mut) {
                for line in lines.iter_mut() {
                    let variant = field_str(line, "variant_id")?.to_string();
                    let stock: i64 = conn
                        .query_row(
                            "SELECT full_units_qty FROM stock_balances WHERE branch_id = ?1 AND variant_id = ?2",
                            params![a.branch_id, variant],
                            |r| r.get(0),
                        )
                        .optional()?
                        .unwrap_or(0);
                    line["available_stock_units"] = json!(stock);
                }
            }
        }
    }

    if let Some(lines) = enriched["lines"].as_array_mut() {
        for line in lines.iter_mut() {
            let variant = field_str(line, "variant_id")?.to_string();
            let stock: i64 = conn
                .query_row(
                    "SELECT full_units_qty FROM stock_balances WHERE branch_id = ?1 AND variant_id = ?2",
                    params![a.branch_id, variant],
                    |r| r.get(0),
                )
                .optional()?
                .unwrap_or(0);
            line["available_stock_units"] = json!(stock);
        }
    }

    // Capacité de casier et autorisation de demi-casier : relues en base pour
    // chaque ligne, jamais reprises du client.
    if let Some(lines) = enriched["lines"].as_array_mut() {
        for line in lines.iter_mut() {
            let unit = line.get("unit").and_then(Value::as_str).unwrap_or_default();
            if unit != "CASE" && unit != "HALF_CASE" {
                continue;
            }
            if let Some(spec) = crate::crates::crate_spec_owned_for_line(conn, line) {
                line["crate_type_id"] = json!(spec.id);
                line["crate_capacity"] = json!(spec.capacity);
                line["allow_half_case"] = json!(spec.allow_half);
            }
        }
    }

    // Formats de casiers mixtes : résolus en base (catalogues, formats créés par
    // l'OWNER), puis transmis au domaine qui reste pur. Un format inconnu est
    // simplement absent : le domaine le refusera explicitement (CRATE_NOT_FOUND).
    if let Some(cases) = enriched.get("mixed_cases").and_then(Value::as_array) {
        let mut specs = Vec::new();
        for case in cases {
            if let Some(id) = case.get("crate_type_id").and_then(Value::as_str) {
                if let Ok(spec) = crate::crates::crate_spec_owned_from_db(conn, id) {
                    specs.push(json!({
                        "id": spec.id,
                        "code": spec.code,
                        "capacity": spec.capacity,
                        "group": spec.group,
                        "allow_half": spec.allow_half,
                        "replacement_fcfa": spec.replacement_fcfa
                    }));
                }
            }
        }
        if !specs.is_empty() {
            enriched["resolved_crate_specs"] = json!(specs);
        }
    }
    Ok(enriched)
}

pub fn validate_sale(conn: &Connection, payload: &Value) -> CoreResult<Value> {
    let enriched = prepare_sale(conn, payload)?;
    validate_sale_rules(&enriched)
}

pub fn create_sale(conn: &mut Connection, payload: &Value) -> CoreResult<Value> {
    let now = now_ms();
    let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
    let a = actor(&tx, payload)?;

    // Idempotence : la vente écrit plusieurs tables (vente, lignes, paiements,
    // stock, casiers, caisse, audit, outbox). Un rejeu avec la même clé — double
    // appui, nouvelle tentative après coupure réseau — renvoie la vente déjà
    // enregistrée au lieu d'en créer une seconde.
    let request_key = field_str_opt(payload, "request_key").map(str::to_string);
    if let Some(key) = request_key.as_deref() {
        let previous: Option<String> = tx
            .query_row(
                "SELECT payload_json FROM sync_outbox WHERE branch_id = ?1 AND idempotency_key = ?2",
                params![a.branch_id, key],
                |r| r.get(0),
            )
            .optional()?;
        if let Some(stored_json) = previous {
            let stored: Value = serde_json::from_str(&stored_json).unwrap_or(Value::Null);
            return Ok(json!({
                "sale_id": stored.get("sale_id").cloned().unwrap_or(Value::Null),
                "receipt_number": stored.get("receipt_number").cloned().unwrap_or(Value::Null),
                "total_fcfa": stored.get("total_fcfa").cloned().unwrap_or(json!(0)),
                "credit_fcfa": stored.get("credit_fcfa").cloned().unwrap_or(json!(0)),
                "customer_id": stored.get("customer_id").cloned().unwrap_or(Value::Null),
                "customer_name": stored.get("customer_name").cloned().unwrap_or(Value::Null),
                "customer_created": false,
                "outbox_status": "PENDING",
                "idempotent": true,
                "crate_movements": [],
                "crate_balance_is_physical": true
            }));
        }
    }

    let mut enriched = prepare_sale(&tx, payload)?;
    let result = validate_sale_rules(&enriched)?;
    // Fiche de clientèle obligatoire : un client inconnu est enregistré dans la
    // même transaction que la vente, puis rattaché à la vente, aux mouvements de
    // casiers et au suivi (achats, crédit, soldes).
    let mut customer_created = false;
    let customer_id = match field_str_opt(&enriched, "customer_id") {
        Some(id) => Some(id.to_string()),
        None => {
            let raw_name = field_str(&enriched, "customer_name")?.to_string();
            let phone = field_str_opt(&enriched, "customer_phone").map(str::to_string);
            let (id, created) = ensure_customer(
                &tx, &a, &raw_name, phone.as_deref(), field_str_opt(&enriched, "customer_neighborhood"),
            )?;
            customer_created = created;
            if created {
                db::audit(
                    &tx, &a.branch_id, &a.user_id, "CUSTOMER_CREATED",
                    &format!("Fiche client {} ouverte à la vente", display_name(&raw_name)?),
                )?;
            }
            Some(id)
        }
    };
    if let Some(id) = customer_id.as_deref() {
        // Les mouvements de casiers de la vente s'imputent à ce client.
        enriched["customer_id"] = json!(id);
    }
    let sale_id = new_id("sale");
    // Deux ventes dans la même milliseconde ne doivent pas partager un numéro.
    let suffix = sale_id.rsplit('_').next().unwrap_or("0000");
    let receipt = format!("RC-{}-{}", now, &suffix[..suffix.len().min(4)]);

    let total = field_i64(&result, "total_fcfa")?;
    let paid = field_i64(&result, "confirmed_paid_fcfa")?;
    let pending = field_i64(&result, "pending_mobile_money_fcfa")?;
    let credit = field_i64(&result, "credit_fcfa")?;
    let customer_name: String = match customer_id.as_deref() {
        Some(id) => conn_query_customer_name(&tx, id)?,
        None => "Client Comptoir".to_string(),
    };

    tx.execute(
        "INSERT INTO sales (id, receipt_number, tenant_id, branch_id, customer_id, customer_name, user_id, cashier_name, device_id, price_tier, subtotal_fcfa, discount_fcfa, total_fcfa, paid_confirmed_fcfa, pending_mobile_money_fcfa, credit_fcfa, is_offline, has_stock_warning, sync_status, occurred_at) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,'PENDING',?19)",
        params![
            sale_id, receipt, a.tenant_id, a.branch_id, customer_id, customer_name, a.user_id,
            actor_full_name(&tx, &a.user_id)?, a.device_id,
            field_str(payload, "price_tier").unwrap_or("RETAIL"),
            field_i64(&result, "subtotal_fcfa")?, field_i64(&result, "discount_fcfa")?, total, paid, pending,
            credit, if field_bool(payload, "is_online", true) { 0 } else { 1 },
            if field_array(&result, "offline_stock_warnings")?.is_empty() { 0 } else { 1 }, now
        ],
    )?;

    for line in field_array(&enriched, "lines")? {
        let count = field_i64(line, "unit_count")?;
        let physical = physical_units(
            field_str(line, "unit")?, count, field_i64(line, "crate_capacity")?,
            field_i64(line, "pack_size")?, field_bool(line, "allow_half_case", false),
        )?;
        let line_total = checked_mul(field_i64(line, "unit_price_fcfa")?, count, "ligne")?;
        tx.execute(
            "INSERT INTO sale_lines (id, sale_id, variant_id, variant_name, unit_code, unit_count, physical_units, unit_price_fcfa, line_total_fcfa, is_mixed) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,0)",
            params![new_id("sline"), sale_id, field_str(line, "variant_id")?, field_str(line, "variant_name")?, field_str(line, "unit")?, count, physical, field_i64(line, "unit_price_fcfa")?, line_total],
        )?;
        // Sortie de stock réelle : la quantité physique quitte le dépôt, même si
        // la vente est à crédit. Un variant jamais approvisionné reçoit une ligne
        // négative plutôt qu'une erreur SQL (le décalage reste visible).
        tx.execute(
            "INSERT INTO stock_balances (branch_id, variant_id, full_units_qty, min_alert_units, server_revision)
             VALUES (?1,?2,?3,12,1)
             ON CONFLICT(branch_id,variant_id) DO UPDATE SET
               full_units_qty = stock_balances.full_units_qty - ?4,
               server_revision = server_revision + 1",
            params![a.branch_id, field_str(line, "variant_id")?, -physical, physical],
        )?;
        // Journal de stock : c'est ici que la sortie se lit (le journal de caisse
        // ne doit contenir que des montants).
        tx.execute(
            "INSERT INTO stock_movements (id, branch_id, variant_id, movement_type, qty, reason, document_id, user_id, device_id, occurred_at)
             VALUES (?1,?2,?3,'SALE_OUT',?4,?5,?6,?7,?8,?9)",
            params![
                new_id("smov"), a.branch_id, field_str(line, "variant_id")?, -physical,
                format!("Vente {receipt}"), sale_id, a.user_id, a.device_id, now
            ],
        )?;
    }

    let mixed_cases: Vec<Value> = enriched
        .get("mixed_cases")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    for case in &mixed_cases {
        let crate_id = field_str(case, "crate_type_id")?.to_string();
        let spec = resolved_crate_spec(&enriched, &crate_id)?;
        let validated = validate_mixed_case_with(case, &spec)?;
        for line in field_array(case, "lines")? {
            let qty = field_i64(line, "quantity_bottles")?;
            let variant = field_str(line, "variant_id")?;
            tx.execute(
                "INSERT INTO stock_balances (branch_id, variant_id, full_units_qty, min_alert_units, server_revision)
                 VALUES (?1,?2,?3,12,1)
                 ON CONFLICT(branch_id,variant_id) DO UPDATE SET
                   full_units_qty = stock_balances.full_units_qty - ?4,
                   server_revision = server_revision + 1",
                params![a.branch_id, variant, -qty, qty],
            )?;
            tx.execute(
                "INSERT INTO sale_lines (id, sale_id, variant_id, variant_name, unit_code, unit_count, physical_units, unit_price_fcfa, line_total_fcfa, is_mixed) VALUES (?1,?2,?3,?4,'BOTTLE',?5,?6,?7,?8,1)",
                params![new_id("sline"), sale_id, variant, field_str(line, "variant_name")?, qty, qty, field_i64(line, "unit_price_fcfa")?, checked_mul(field_i64(line, "unit_price_fcfa")?, qty, "mixte")?],
            )?;
            tx.execute(
                "INSERT INTO stock_movements (id, branch_id, variant_id, movement_type, qty, reason, document_id, user_id, device_id, occurred_at)
                 VALUES (?1,?2,?3,'SALE_OUT',?4,?5,?6,?7,?8,?9)",
                params![
                    new_id("smov"), a.branch_id, variant, -qty,
                    format!("Vente {receipt} (casier mixte)"), sale_id, a.user_id, a.device_id, now
                ],
            )?;
        }
        let _ = field_i64(&validated, "total_drinks_fcfa")?;
    }

    for payment in field_array(payload, "payments")? {
        let amount = field_i64(payment, "amount_fcfa")?;
        if amount <= 0 || field_str(payment, "method")? == "CREDIT" {
            continue;
        }
        let status = if field_str(payment, "method")? == "MOBILE_MONEY"
            && (!field_bool(payment, "confirmed_by_provider", false)
                || field_str_opt(payment, "reference").is_none())
        {
            "PENDING_CONFIRMATION"
        } else {
            "CONFIRMED"
        };
        tx.execute(
            "INSERT INTO payments (id, branch_id, partner_type, partner_id, document_id, method, reference, status, amount_fcfa, occurred_at) VALUES (?1,?2,'CUSTOMER',?3,?4,?5,?6,?7,?8,?9)",
            params![new_id("pay"), a.branch_id, customer_id, sale_id, field_str(payment, "method")?, field_str_opt(payment, "reference"), status, amount, now],
        )?;
        if field_str(payment, "method")? == "CASH" && status == "CONFIRMED" {
            if let Some(session) = open_cash_session(&tx, &a.branch_id)? {
                tx.execute(
                    "UPDATE cash_sessions SET expected_closing_fcfa = expected_closing_fcfa + ?1 WHERE id = ?2",
                    params![amount, session],
                )?;
                // Traçabilité : les rapports somment les mouvements de caisse
                // positifs, une vente en espèces doit donc y figurer.
                tx.execute(
                    "INSERT INTO cash_movements (id, session_id, branch_id, movement_type, signed_amount_fcfa, label, occurred_at) VALUES (?1,?2,?3,'IN',?4,?5,?6)",
                    params![
                        new_id("cmv"), session, a.branch_id, amount,
                        format!("Encaissement vente {receipt}"), now
                    ],
                )?;
            }
        }
    }

    if let Some(id) = customer_id.as_deref() {
        tx.execute(
            "UPDATE customers SET total_purchases_fcfa = total_purchases_fcfa + ?1, total_paid_fcfa = total_paid_fcfa + ?2, financial_balance_fcfa = financial_balance_fcfa + ?3 WHERE id = ?4",
            params![total, paid, credit, id],
        )?;
    }

    // Casiers plastiques : journal physique séparé, dans la même transaction.
    // Aucune attente d'emballage, aucune bouteille, aucune dette n'est créée ici.
    let crate_outcome = crates::apply_sale_crate_movements(&tx, &a, &sale_id, &receipt, &enriched)?;

    let command = if !mixed_cases.is_empty() {
        "MIXED_SALE_CREATE"
    } else {
        "SALE_CREATE"
    };
    let summary = format!("{receipt} • {customer_name} ({total} FCFA)");
    let outbox_payload = json!({
        "sale_id": sale_id, "receipt_number": receipt, "total_fcfa": total, "credit_fcfa": credit,
        "customer_id": customer_id, "customer_created": customer_created,
        "customer_name": customer_name
    });
    let base = base_revision(&tx, &a.branch_id)?;
    db::enqueue(
        &tx, &a.tenant_id, &a.branch_id, &a.device_id, &a.user_id, command, &summary,
        &outbox_payload.to_string(), base,
        &request_key.clone().unwrap_or_else(|| format!("idem_{sale_id}")),
    )?;
    db::audit(&tx, &a.branch_id, &a.user_id, command, &summary)?;
    tx.commit()?;

    Ok(json!({
        "sale_id": sale_id,
        "receipt_number": receipt,
        "total_fcfa": total,
        "credit_fcfa": credit,
        "customer_id": customer_id,
        "customer_name": customer_name,
        "customer_created": customer_created,
        "outbox_status": "PENDING",
        "crate_movements": crate_outcome["crate_movements"].clone(),
        "crate_balance_is_physical": true
    }))
}

fn conn_query_customer_name(conn: &Connection, id: &str) -> CoreResult<String> {
    conn.query_row("SELECT name FROM customers WHERE id = ?1", params![id], |r| r.get(0))
        .map_err(|_| CoreError::new("CUSTOMER_NOT_FOUND", "Client introuvable."))
}

fn actor_full_name(conn: &Connection, id: &str) -> CoreResult<String> {
    conn.query_row("SELECT full_name FROM users WHERE id = ?1", params![id], |r| r.get(0))
        .map_err(|_| CoreError::new("USER_NOT_FOUND", "Utilisateur introuvable."))
}

/// Téléphone réduit à ses chiffres : « 07 07 12 34 56 » et « +2250707123456 »
/// désignent la même fiche client.
pub(crate) fn normalized_phone(raw: &str) -> String {
    let digits: String = raw.chars().filter(|c| c.is_ascii_digit()).collect();
    // Indicatif ivoirien : « +225 07 07 12 34 56 » et « 07 07 12 34 56 » sont le
    // même numéro, donc la même fiche.
    for prefix in ["00225", "225"] {
        if let Some(rest) = digits.strip_prefix(prefix) {
            if rest.chars().count() == 10 {
                return rest.to_string();
            }
        }
    }
    digits
}

/// Nom réduit à une forme comparable : espaces compactés, casse ignorée.
pub(crate) fn normalized_name(raw: &str) -> String {
    raw.split_whitespace().collect::<Vec<_>>().join(" ").to_uppercase()
}

/// Nom tel qu'il sera enregistré : espaces compactés, casse d'origine conservée.
pub(crate) fn display_name(raw: &str) -> CoreResult<String> {
    let clean = raw.split_whitespace().collect::<Vec<_>>().join(" ");
    if clean.chars().count() < 2 {
        return Err(CoreError::new(
            "CUSTOMER_NAME_REQUIRED",
            "Le nom du client est obligatoire pour ouvrir une fiche de clientèle.",
        ));
    }
    Ok(clean)
}

/// Téléphone utilisable comme clé d'identité : vide accepté, sinon au moins 8 chiffres.
pub(crate) fn phone_key(raw: Option<&str>) -> CoreResult<String> {
    let digits = raw.map(normalized_phone).unwrap_or_default();
    if !digits.is_empty() && digits.chars().count() < 8 {
        return Err(CoreError::new(
            "INVALID_PHONE",
            "Téléphone trop court : saisissez le numéro complet ou laissez vide.",
        ));
    }
    Ok(digits)
}

/// Cherche la fiche existante : téléphone d'abord (identité forte), puis nom exact.
/// Aucune fiche n'est dupliquée pour un même client.
pub(crate) fn find_customer(
    conn: &Connection,
    tenant: &str,
    name: &str,
    phone: &str,
) -> CoreResult<Option<String>> {
    let wanted_name = normalized_name(name);
    let mut stmt = conn.prepare(
        "SELECT id, name, phone FROM customers WHERE tenant_id = ?1
         ORDER BY is_archived ASC, rowid ASC",
    )?;
    let mut rows = stmt.query(params![tenant])?;
    let mut same_name: Option<String> = None;
    while let Some(row) = rows.next()? {
        let id: String = row.get(0)?;
        let existing_name: String = row.get(1)?;
        let existing_phone: String = row.get(2)?;
        if !phone.is_empty() && normalized_phone(&existing_phone) == phone {
            return Ok(Some(id));
        }
        if same_name.is_none() && !wanted_name.is_empty() && normalized_name(&existing_name) == wanted_name {
            same_name = Some(id);
        }
    }
    Ok(same_name)
}

/// Fiche de clientèle unique : retrouve le client ou l'ouvre.
/// Retourne `(customer_id, created)`.
pub(crate) fn ensure_customer(
    conn: &Connection,
    a: &Actor,
    raw_name: &str,
    raw_phone: Option<&str>,
    neighborhood: Option<&str>,
) -> CoreResult<(String, bool)> {
    let name = display_name(raw_name)?;
    let phone = phone_key(raw_phone)?;
    if let Some(id) = find_customer(conn, &a.tenant_id, &name, &phone)? {
        return Ok((id, false));
    }
    let id = new_id("cust");
    // Un nouveau comptoir n'obtient aucun crédit tant que l'OWNER ne l'a pas
    // autorisé : la fiche sert au suivi, pas à l'endettement automatique.
    conn.execute(
        "INSERT INTO customers (id, tenant_id, name, phone, neighborhood, customer_type, default_price_tier,
            credit_limit_fcfa, total_purchases_fcfa, total_paid_fcfa, financial_balance_fcfa, is_archived)
         VALUES (?1,?2,?3,?4,?5,'DEPOT','RETAIL',0,0,0,0,0)",
        params![id, a.tenant_id, name, phone, neighborhood.unwrap_or("").trim()],
    )?;
    Ok((id, true))
}

pub(crate) fn open_cash_session(conn: &Connection, branch: &str) -> CoreResult<Option<String>> {
    Ok(conn
        .query_row(
            "SELECT id FROM cash_sessions WHERE branch_id = ?1 AND status = 'OPEN' LIMIT 1",
            params![branch],
            |r| r.get(0),
        )
        .optional()?)
}

fn base_revision(conn: &Connection, branch: &str) -> i64 {
    conn.query_row(
        "SELECT COALESCE(MAX(server_revision), 1) FROM stock_balances WHERE branch_id = ?1",
        params![branch],
        |r| r.get(0),
    )
    .unwrap_or(1)
}

pub fn create_mixed_order(conn: &Connection, payload: &Value) -> CoreResult<Value> {
    let a = actor(conn, payload)?;
    // Commande de **vérification** : le casier mixte n'est enregistré qu'au
    // moment de la vente (`createSale`), qui revalide et écrit tout dans une
    // seule transaction. Rien n'est créé ici : ni vente, ni stock, ni mouvement.
    let crate_id = field_str(payload, "crate_type_id")?.to_string();
    let spec = crate::crates::crate_spec_owned_from_db(conn, &crate_id)?;
    let mut validated = validate_mixed_case_with(payload, &spec)?;
    // Compose-t-on ou commande-t-on ? La composition ne crée rien : la fiche de
    // clientèle est exigée et ouverte au moment de la vente (`createSale`). On
    // annonce simplement ce que la vente fera de l'identité fournie.
    let (customer_id, will_create, required) = match field_str_opt(payload, "customer_id") {
        Some(id) => (Some(id.to_string()), false, false),
        None => match field_str_opt(payload, "customer_name") {
            Some(raw_name) => {
                let name = display_name(raw_name)?;
                let phone = phone_key(field_str_opt(payload, "customer_phone"))?;
                match find_customer(conn, &a.tenant_id, &name, &phone)? {
                    Some(id) => (Some(id), false, false),
                    None => (None, true, false),
                }
            }
            None => (None, false, true),
        },
    };
    validated["customer_id"] = json!(customer_id);
    validated["customer_will_be_created"] = json!(will_create);
    validated["customer_required"] = json!(required);
    Ok(validated)
}

/// Ouverture (ou récupération) explicite d'une fiche de clientèle.
///
/// L'identité est unique : le téléphone, s'il est fourni, sert de clé ; sinon le
/// nom exact départage. Un client déjà connu n'obtient jamais une seconde fiche.
pub fn create_customer(conn: &mut Connection, payload: &Value) -> CoreResult<Value> {
    let a = actor(conn, payload)?;
    let raw_name = field_str(payload, "name")
        .or_else(|_| field_str(payload, "customer_name"))?
        .to_string();
    let phone = field_str_opt(payload, "phone")
        .or_else(|| field_str_opt(payload, "customer_phone"))
        .map(str::to_string);
    let neighborhood = field_str_opt(payload, "neighborhood").map(str::to_string);
    let name = display_name(&raw_name)?;
    let phone_key_value = phone_key(phone.as_deref())?;

    let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
    let (customer_id, created) = ensure_customer(
        &tx, &a, &name, phone.as_deref(), neighborhood.as_deref(),
    )?;
    if created {
        db::audit(
            &tx, &a.branch_id, &a.user_id, "CUSTOMER_CREATED",
            &format!("Fiche client {name} ouverte"),
        )?;
        let outbox_payload = json!({
            "entity_type": "customer", "customer_id": customer_id,
            "name": name, "phone": phone_key_value, "neighborhood": neighborhood
        });
        db::enqueue(
            &tx, &a.tenant_id, &a.branch_id, &a.device_id, &a.user_id, "createCustomer",
            &format!("Fiche client {name}"), &outbox_payload.to_string(),
            base_revision(&tx, &a.branch_id), &format!("customer_{customer_id}"),
        )?;
    }
    let (stored_name, stored_phone, credit_limit, balance): (String, String, i64, i64) = tx.query_row(
        "SELECT name, phone, credit_limit_fcfa, financial_balance_fcfa FROM customers WHERE id = ?1",
        params![customer_id],
        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
    )?;
    tx.commit()?;
    Ok(json!({
        "customer_id": customer_id,
        "created": created,
        "name": stored_name,
        "phone": stored_phone,
        "credit_limit_fcfa": credit_limit,
        "outstanding_fcfa": balance
    }))
}

/// Compatibilité de l'ancienne commande `registerPackagingReturn`.
///
/// Depuis la migration 11, seuls les **casiers plastiques** circulent :
/// - un retour client passe par le journal physique des casiers ;
/// - une remise fournisseur passe par le compte physique fournisseur ;
/// - une bouteille n'est plus suivie et la commande est refusée explicitement
///   (aucune dette, aucun stock, aucun retour de bouteille n'est créé).
pub fn register_packaging_return(conn: &mut Connection, payload: &Value) -> CoreResult<Value> {
    let partner_type = field_str(payload, "partner_type")?.to_string();
    let item_kind = field_str(payload, "item_kind")?.to_string();
    if item_kind != "CRATE" {
        return Err(CoreError::new(
            "BOTTLE_PACKAGING_NOT_TRACKED",
            "Les bouteilles, canettes et PET ne sont plus suivis : seul le casier plastique possède un stock et un solde physique. Utilisez la référence d'un type de casier.",
        ));
    }
    let partner_id = field_str(payload, "partner_id")?.to_string();
    let ref_id = field_str(payload, "ref_id")?.to_string();
    let qty = field_i64(payload, "qty")?;
    let key = field_str_opt(payload, "request_key")
        .map(str::to_string)
        .unwrap_or_else(|| format!("legacy_return:{partner_type}:{partner_id}:{ref_id}:{qty}"));
    let mut delegated = payload.clone();
    delegated["request_key"] = json!(key);
    let result = if partner_type == "CUSTOMER" {
        crate::crates::record_customer_crate_return(conn, &delegated)?
    } else if partner_type == "SUPPLIER" {
        delegated["supplier_id"] = json!(partner_id);
        delegated["lines"] = json!([{
            "crate_type_id": ref_id,
            "empty_returned_qty": qty,
            "reason": field_str_opt(payload, "reason").unwrap_or("Casiers remis au fournisseur")
        }]);
        crate::crates::record_supplier_crate_exchange(conn, &delegated)?
    } else {
        return Err(CoreError::new("INVALID_PARTNER", "Type de partenaire inconnu."));
    };
    Ok(json!({
        "status": "RECORDED",
        "deprecated_command": true,
        "engine": "PHYSICAL_CRATES",
        "result": result
    }))
}

pub fn create_purchase_order(conn: &mut Connection, payload: &Value) -> CoreResult<Value> {
    let a = actor(conn, payload)?;
    if !can_manage_purchases(&a.role) {
        return Err(CoreError::new("ROLE_FORBIDDEN", "Rôle non autorisé à créer un achat."));
    }
    let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
    let lines = field_array(payload, "lines")?;
    if lines.is_empty() {
        return Err(CoreError::new("EMPTY_ORDER", "Ajoutez au moins une ligne."));
    }
    let mut total = 0i64;
    let mut order_lines: Vec<(String, i64, i64)> = Vec::new();
    let mut seen_variants = std::collections::HashSet::new();
    for line in lines {
        let variant = field_str(line, "variant_id")?;
        if !seen_variants.insert(variant.to_string()) {
            return Err(CoreError::new("DUPLICATE_ORDER_LINE", "Variante en double dans la commande."));
        }
        let qty = field_i64(line, "quantity")?;
        if qty <= 0 {
            return Err(CoreError::new("INVALID_QUANTITY", "Quantité commandée positive requise."));
        }
        let price: i64 = tx.query_row(
            "SELECT wholesale_case_fcfa FROM product_variants WHERE id = ?1",
            params![variant],
            |r| r.get(0),
        )?;
        total = checked_add(total, checked_mul(price, qty, "bon de commande")?, "total commande")?;
        order_lines.push((variant.to_string(), qty, price));
    }
    let order_id = new_id("po");
    let reference = format!("BC-{}", &order_id[order_id.len() - 6..]);
    tx.execute(
        "INSERT INTO purchase_orders (id, branch_id, supplier_id, reference, total_fcfa, status, created_at) VALUES (?1,?2,?3,?4,?5,'ORDERED',?6)",
        params![order_id, a.branch_id, field_str(payload, "supplier_id")?, reference, total, now_ms()],
    )?;
    for (variant, qty, price) in &order_lines {
        tx.execute(
            "INSERT INTO purchase_order_lines (id, order_id, variant_id, ordered_qty, unit_price_fcfa) VALUES (?1,?2,?3,?4,?5)",
            params![new_id("pol"), order_id, variant, qty, price],
        )?;
    }
    db::enqueue(
        &tx, &a.tenant_id, &a.branch_id, &a.device_id, &a.user_id, "PURCHASE_ORDER_CREATE",
        &format!("{reference} ({total} FCFA)"), &json!({"order_id": order_id, "total_fcfa": total}).to_string(),
        1, &new_id("idem"),
    )?;
    tx.commit()?;
    Ok(json!({"order_id": order_id, "reference": reference, "total_fcfa": total}))
}

pub fn receive_purchase(conn: &mut Connection, payload: &Value) -> CoreResult<Value> {
    let a = actor(conn, payload)?;
    if !can_manage_purchases(&a.role) {
        return Err(CoreError::new("ROLE_FORBIDDEN", "Rôle non autorisé à réceptionner un achat."));
    }
    let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
    let order_id = field_str(payload, "order_id")?;
    let supplier: String = tx.query_row(
        "SELECT supplier_id FROM purchase_orders WHERE id = ?1",
        params![order_id], |r| r.get(0),
    )?;
    let receipt_id = new_id("prec");
    let mut received_total = 0i64;
    let mut seen_variants = std::collections::HashSet::new();
    for line in field_array(payload, "lines")? {
        let variant_line = field_str(line, "variant_id")?;
        if !seen_variants.insert(variant_line.to_string()) {
            return Err(CoreError::new("DUPLICATE_RECEPTION_LINE", "Variante en double dans la réception."));
        }
        let qty = field_i64(line, "quantity")?;
        let damaged = field_i64_or(line, "damaged_qty", 0);
        if qty < 0 || damaged < 0 || damaged > qty {
            return Err(CoreError::new("INVALID_RECEPTION_QTY", "Quantités reçues/endommagées incohérentes."));
        }
        let variant = field_str(line, "variant_id")?;
        let ordered_price: i64 = tx.query_row(
            "SELECT wholesale_case_fcfa FROM product_variants WHERE id = ?1",
            params![variant], |r| r.get(0),
        )?;
        let price = field_i64_or(line, "unit_price_fcfa", ordered_price);
        if price < 0 {
            return Err(CoreError::new("INVALID_UNIT_PRICE", "Prix facturé invalide."));
        }
        let good_qty = qty - damaged;
        let reason = field_str_opt(line, "reason").unwrap_or("Réception fournisseur");
        received_total = checked_add(received_total, checked_mul(price, qty, "réception")?, "total réception")?;
        tx.execute(
            "INSERT INTO purchase_reception_lines (id, receipt_id, order_id, variant_id, received_qty, damaged_qty, unit_price_fcfa, reason)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8)",
            params![new_id("prl"), receipt_id, order_id, variant, qty, damaged, price, reason],
        )?;
        if good_qty > 0 {
            let current: i64 = tx.query_row(
                "SELECT full_units_qty FROM stock_balances WHERE branch_id = ?1 AND variant_id = ?2",
                params![a.branch_id, variant], |r| r.get(0),
            ).optional()?.unwrap_or(0);
            let pack_size: i64 = tx.query_row(
                "SELECT pack_size FROM product_variants WHERE id = ?1", params![variant], |r| r.get(0),
            )?;
            tx.execute(
                "INSERT INTO stock_balances (branch_id, variant_id, full_units_qty, min_alert_units, server_revision)
                 VALUES (?1,?2,?3,12,1)
                 ON CONFLICT(branch_id,variant_id) DO UPDATE SET full_units_qty=excluded.full_units_qty,
                 server_revision=server_revision+1",
                params![a.branch_id, variant, current + checked_mul(pack_size, good_qty, "réception stock")?],
            )?;
            tx.execute(
                "INSERT INTO stock_movements (id, branch_id, variant_id, movement_type, qty, reason, document_id, user_id, device_id, occurred_at)
                 VALUES (?1,?2,?3,'PURCHASE_RECEIPT',?4,?5,?6,?7,?8,?9)",
                params![new_id("smov"), a.branch_id, variant, checked_mul(pack_size, good_qty, "mouvement réception")?,
                    reason, receipt_id, a.user_id, a.device_id, now_ms()],
            )?;
        }
    }
    // Casiers physiques de la réception : pleins reçus et vides remis au
    // fournisseur sont deux mouvements distincts. Les bouteilles ne sont plus
    // suivies (aucun stock, aucune dette).
    let crate_exchange = crates::apply_purchase_crate_exchange(&tx, &a, &supplier, &receipt_id, payload)?;
    tx.execute(
        "UPDATE suppliers SET financial_debt_fcfa = financial_debt_fcfa + ?1, total_purchased_fcfa = total_purchased_fcfa + ?1 WHERE id = ?2",
        params![received_total, supplier],
    )?;
    let ordered: i64 = tx.query_row(
        "SELECT COALESCE(SUM(ordered_qty),0) FROM purchase_order_lines WHERE order_id=?1",
        params![order_id], |r| r.get(0),
    ).unwrap_or(0);
    let received: i64 = tx.query_row(
        "SELECT COALESCE(SUM(received_qty),0) FROM purchase_reception_lines WHERE order_id=?1",
        params![order_id], |r| r.get(0),
    ).unwrap_or(0);
    let status = if ordered > 0 && received >= ordered { "RECEIVED" } else { "PARTIAL_RECEIVED" };
    tx.execute("UPDATE purchase_orders SET status = ?1 WHERE id = ?2", params![status, order_id])?;
    tx.execute(
        "INSERT INTO purchase_receipts (id, order_id, branch_id, total_fcfa, received_at) VALUES (?1,?2,?3,?4,?5)",
        params![receipt_id, order_id, a.branch_id, received_total, now_ms()],
    )?;
    db::enqueue(
        &tx, &a.tenant_id, &a.branch_id, &a.device_id, &a.user_id, "PURCHASE_RECEIVE",
        &format!("Réception {order_id} ({received_total} FCFA)"), &json!({"order_id": order_id, "total_fcfa": received_total}).to_string(),
        1, &new_id("idem"),
    )?;
    tx.commit()?;
    Ok(json!({
        "order_id": order_id,
        "receipt_id": receipt_id,
        "received_fcfa": received_total,
        "status": status,
        "remaining_qty": (ordered - received).max(0),
        "crate_exchange": crate_exchange
    }))
}

/// Retour de marchandise au fournisseur : stock et dette fournisseur ajustés dans la même transaction.
pub fn record_supplier_return(conn: &mut Connection, payload: &Value) -> CoreResult<Value> {
    let a = actor(conn, payload)?;
    if !can_manage_purchases(&a.role) {
        return Err(CoreError::new("ROLE_FORBIDDEN", "Rôle non autorisé à enregistrer un retour fournisseur."));
    }
    let supplier = field_str(payload, "supplier_id")?;
    let reason = field_str(payload, "reason")?.trim().to_string();
    if reason.len() < 3 {
        return Err(CoreError::new("REASON_REQUIRED", "Un motif de retour est obligatoire."));
    }
    let lines = field_array(payload, "lines")?;
    if lines.is_empty() {
        return Err(CoreError::new("EMPTY_RETURN", "Au moins une ligne de retour est requise."));
    }
    let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
    let supplier_exists: i64 = tx.query_row(
        "SELECT COUNT(*) FROM suppliers WHERE id=?1", params![supplier], |r| r.get(0),
    )?;
    if supplier_exists == 0 {
        return Err(CoreError::new("SUPPLIER_UNKNOWN", "Fournisseur inconnu."));
    }
    let return_id = new_id("sret");
    tx.execute(
        "INSERT INTO supplier_returns (id, branch_id, supplier_id, reason, status, created_at)
         VALUES (?1,?2,?3,?4,'REGISTERED',?5)",
        params![return_id, a.branch_id, supplier, reason, now_ms()],
    )?;
    let mut total_fcfa = 0i64;
    for line in lines {
        let variant = field_str(line, "variant_id")?;
        let qty = field_i64(line, "qty")?;
        let price = field_i64(line, "unit_price_fcfa")?;
        if qty <= 0 || price < 0 {
            return Err(CoreError::new("INVALID_RETURN_QTY", "Quantité positive et prix valide requis."));
        }
        let (pack_size, current): (i64, i64) = tx.query_row(
            "SELECT v.pack_size, COALESCE(s.full_units_qty,0)
             FROM product_variants v LEFT JOIN stock_balances s
               ON s.branch_id=?1 AND s.variant_id=v.id WHERE v.id=?2",
            params![a.branch_id, variant],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )?;
        let units = checked_mul(pack_size, qty, "retour fournisseur")?;
        if units > current {
            return Err(CoreError::new("STOCK_INSUFFICIENT", "Stock insuffisant pour ce retour."));
        }
        total_fcfa = checked_add(total_fcfa, checked_mul(price, qty, "montant retour")?, "total retour")?;
        tx.execute(
            "UPDATE stock_balances SET full_units_qty=full_units_qty-?1, server_revision=server_revision+1
             WHERE branch_id=?2 AND variant_id=?3",
            params![units, a.branch_id, variant],
        )?;
        tx.execute(
            "INSERT INTO stock_movements (id, branch_id, variant_id, movement_type, qty, reason, document_id, user_id, device_id, occurred_at)
             VALUES (?1,?2,?3,'SUPPLIER_RETURN',?4,?5,?6,?7,?8,?9)",
            params![new_id("smov"), a.branch_id, variant, -units, reason, return_id, a.user_id, a.device_id, now_ms()],
        )?;
    }
    tx.execute(
        "UPDATE suppliers SET financial_debt_fcfa=MAX(0, financial_debt_fcfa-?1) WHERE id=?2",
        params![total_fcfa, supplier],
    )?;
    db::audit(&tx, &a.branch_id, &a.user_id, "SUPPLIER_RETURN", &format!("{return_id}: {total_fcfa} FCFA"))?;
    db::enqueue(
        &tx, &a.tenant_id, &a.branch_id, &a.device_id, &a.user_id, "SUPPLIER_RETURN",
        &format!("Retour {supplier} ({total_fcfa} FCFA)"), &json!({
            "return_id": return_id, "supplier_id": supplier, "lines": lines, "total_fcfa": total_fcfa
        }).to_string(), 1, &new_id("idem"),
    )?;
    tx.commit()?;
    Ok(json!({ "return_id": return_id, "total_fcfa": total_fcfa, "status": "REGISTERED" }))
}

/// Assistant d'écart : jamais de validation automatique des quantités manquantes.
pub fn get_purchase_variance(conn: &Connection, payload: &Value) -> CoreResult<Value> {
    let _ = actor(conn, payload)?;
    let order_id = field_str(payload, "order_id")?;
    let mut stmt = conn.prepare(
        "SELECT pol.variant_id, v.name, pol.ordered_qty, pol.unit_price_fcfa,
                COALESCE((SELECT SUM(prl.received_qty) FROM purchase_reception_lines prl
                          WHERE prl.order_id=pol.order_id AND prl.variant_id=pol.variant_id),0),
                COALESCE((SELECT SUM(prl.damaged_qty) FROM purchase_reception_lines prl
                          WHERE prl.order_id=pol.order_id AND prl.variant_id=pol.variant_id),0),
                COALESCE((SELECT SUM(prl.unit_price_fcfa*prl.received_qty) FROM purchase_reception_lines prl
                          WHERE prl.order_id=pol.order_id AND prl.variant_id=pol.variant_id),0)
         FROM purchase_order_lines pol
         LEFT JOIN product_variants v ON v.id=pol.variant_id
         WHERE pol.order_id=?1 ORDER BY pol.variant_id",
    )?;
    let rows = stmt.query_map(params![order_id], |r| {
        let ordered = r.get::<_, i64>(2)?;
        let received = r.get::<_, i64>(4)?;
        let damaged = r.get::<_, i64>(5)?;
        let ordered_price = r.get::<_, i64>(3)?;
        let ordered_amount = checked_mul(ordered_price, ordered, "montant commandé")?;
        let invoiced = r.get::<_, i64>(6)?;
        let status = if received >= ordered && ordered > 0 { "COMPLETE" } else { "PARTIAL" };
        Ok(json!({
            "variant_id": r.get::<_, String>(0)?,
            "variant_name": r.get::<_, Option<String>>(1)?.unwrap_or_default(),
            "ordered_qty": ordered,
            "received_qty": received,
            "missing_qty": (ordered - received).max(0),
            "damaged_qty": damaged,
            "ordered_amount_fcfa": ordered_amount,
            "invoiced_amount_fcfa": invoiced,
            "price_difference_fcfa": invoiced - checked_mul(ordered_price, received, "écart prix")?,
            "status": status
        }))
    })?;
    let mut lines = Vec::new();
    for row in rows { lines.push(row?); }
    Ok(json!({ "order_id": order_id, "lines": lines, "auto_validated": false }))
}

pub fn record_payment(conn: &mut Connection, payload: &Value) -> CoreResult<Value> {
    let a = actor(conn, payload)?;
    let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
    let partner_type = field_str(payload, "partner_type")?;
    let partner_id = field_str(payload, "partner_id")?;
    let amount = field_i64(payload, "amount_fcfa")?;
    if amount <= 0 {
        return Err(CoreError::new("INVALID_PAYMENT", "Montant positif requis."));
    }
    let method = field_str(payload, "method")?;
    let status = if method == "MOBILE_MONEY" && field_str_opt(payload, "reference").is_none() {
        "PENDING_CONFIRMATION"
    } else {
        "CONFIRMED"
    };
    tx.execute(
        "INSERT INTO payments (id, branch_id, partner_type, partner_id, document_id, method, reference, status, amount_fcfa, occurred_at) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)",
        params![new_id("pay"), a.branch_id, partner_type, partner_id, new_id("doc"), method, field_str_opt(payload, "reference"), status, amount, now_ms()],
    )?;
    let column = if partner_type == "CUSTOMER" { "financial_balance_fcfa" } else { "financial_debt_fcfa" };
    let sql = format!("UPDATE {partner_table} SET {column} = MAX(0, {column} - ?1) WHERE id = ?2",
        partner_table = if partner_type == "CUSTOMER" { "customers" } else { "suppliers" });
    tx.execute(&sql, params![amount, partner_id])?;
    if method == "CASH" && status == "CONFIRMED" {
        if let Some(session) = open_cash_session(&tx, &a.branch_id)? {
            tx.execute(
                "UPDATE cash_sessions SET expected_closing_fcfa = expected_closing_fcfa + ?1 WHERE id = ?2",
                params![amount, session],
            )?;
        }
    }
    db::enqueue(
        &tx, &a.tenant_id, &a.branch_id, &a.device_id, &a.user_id, "PAYMENT_RECORD",
        &format!("Règlement {partner_id} ({amount} FCFA)"), &payload.to_string(), 1, &new_id("idem"),
    )?;
    tx.commit()?;
    Ok(json!({"status": status, "amount_fcfa": amount}))
}

pub fn get_dashboard(conn: &Connection, payload: &Value) -> CoreResult<Value> {
    let a = actor(conn, payload)?;
    let day_start = now_ms() - 24 * 60 * 60 * 1000;
    let ca: i64 = conn.query_row(
        "SELECT COALESCE(SUM(total_fcfa),0) FROM sales WHERE branch_id = ?1 AND occurred_at >= ?2 AND occurred_at <= ?3",
        params![a.branch_id, day_start, to_ms], |r| r.get(0),
    )?;
    let sales_count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM sales WHERE branch_id = ?1 AND occurred_at >= ?2",
        params![a.branch_id, day_start], |r| r.get(0),
    )?;
    let credits: i64 = conn.query_row(
        "SELECT COALESCE(SUM(financial_balance_fcfa),0) FROM customers", [], |r| r.get(0),
    )?;
    let debts: i64 = conn.query_row(
        "SELECT COALESCE(SUM(financial_debt_fcfa),0) FROM suppliers", [], |r| r.get(0),
    )?;
    let stock_value: i64 = conn.query_row(
        "SELECT COALESCE(SUM(s.full_units_qty * v.retail_bottle_fcfa),0) FROM stock_balances s JOIN product_variants v ON v.id = s.variant_id WHERE s.branch_id = ?1",
        params![a.branch_id], |r| r.get(0),
    )?;
    let low_stock: i64 = conn.query_row(
        "SELECT COUNT(*) FROM stock_balances WHERE branch_id = ?1 AND full_units_qty <= min_alert_units",
        params![a.branch_id], |r| r.get(0),
    )?;
    let pending: i64 = conn.query_row(
        "SELECT COUNT(*) FROM sync_outbox WHERE status IN ('PENDING','FAILED','SYNCING')", [], |r| r.get(0),
    )?;
    let collected: i64 = conn.query_row(
        "SELECT COALESCE(SUM(amount_fcfa),0) FROM payments WHERE status='CONFIRMED' AND occurred_at >= ?1",
        params![day_start], |r| r.get(0),
    )?;
    let expenses: i64 = conn.query_row(
        "SELECT COALESCE(SUM(amount_fcfa),0) FROM expenses WHERE branch_id=?1 AND occurred_at >= ?2",
        params![a.branch_id, day_start], |r| r.get(0),
    )?;
    let credits_granted: i64 = conn.query_row(
        "SELECT COALESCE(SUM(credit_fcfa),0) FROM sales WHERE branch_id=?1 AND occurred_at >= ?2",
        params![a.branch_id, day_start], |r| r.get(0),
    )?;
    let out_of_stock: i64 = conn.query_row(
        "SELECT COUNT(*) FROM stock_balances WHERE branch_id=?1 AND full_units_qty <= 0",
        params![a.branch_id], |r| r.get(0),
    )?;
    // Alertes casiers : soldes physiques clients non soldés. L'ancien compteur
    // d'obligations d'emballages est remplacé, pas simplement renommé.
    let packaging_alerts: i64 = conn.query_row(
        "SELECT COUNT(*) FROM partner_crate_balances
         WHERE branch_id=?1 AND partner_type='CUSTOMER' AND balance_qty > 0",
        params![a.branch_id], |r| r.get(0),
    )?;
    let crates_at_customers: i64 = conn.query_row(
        "SELECT COALESCE(SUM(partner_delta),0) FROM crate_movements
         WHERE branch_id=?1 AND partner_type='CUSTOMER'",
        params![a.branch_id], |r| r.get(0),
    )?;
    let crates_at_suppliers: i64 = conn.query_row(
        "SELECT COALESCE(SUM(partner_delta),0) FROM crate_movements
         WHERE branch_id=?1 AND partner_type='SUPPLIER'",
        params![a.branch_id], |r| r.get(0),
    )?;
    let (crate_full_qty, crate_empty_qty): (i64, i64) = conn.query_row(
        "SELECT COALESCE(SUM(full_qty),0), COALESCE(SUM(empty_qty),0)
         FROM crate_stock WHERE branch_id=?1",
        params![a.branch_id], |r| Ok((r.get(0)?, r.get(1)?)),
    )?;
    let crate_stock_value: i64 = conn.query_row(
        "SELECT COALESCE(SUM((COALESCE(cs.full_qty,0) + COALESCE(cs.empty_qty,0)) * ct.reference_value_fcfa), 0)
         FROM crate_types ct LEFT JOIN crate_stock cs
           ON cs.crate_id = ct.id AND cs.branch_id = ?1
         WHERE ct.is_physical_crate = 1",
        params![a.branch_id], |r| r.get(0),
    )?;
    let crate_untracked_qty: i64 = conn.query_row(
        "SELECT COALESCE(SUM(untracked_qty),0) FROM crate_movements WHERE branch_id=?1",
        params![a.branch_id], |r| r.get(0),
    )?;
    let confirmed_ca: i64 = conn.query_row(
        "SELECT COALESCE(SUM(total_fcfa),0) FROM sales WHERE branch_id=?1 AND occurred_at >= ?2 AND sync_status='SYNCED'",
        params![a.branch_id, day_start], |r| r.get(0),
    )?;
    Ok(json!({
        "daily_revenue_fcfa": ca,
        "daily_sales_count": sales_count,
        "receivables_fcfa": credits,
        "supplier_debts_fcfa": debts,
        "stock_value_fcfa": stock_value,
        "low_stock_alerts": low_stock,
        "pending_sync_operations": pending,
        "collected_fcfa": collected,
        "expenses_fcfa": expenses,
        "credits_granted_fcfa": credits_granted,
        "out_of_stock_alerts": out_of_stock,
        "packaging_alerts": packaging_alerts,
        "crate_alerts": packaging_alerts,
        "crate_full_qty": crate_full_qty,
        "crate_empty_qty": crate_empty_qty,
        "crates_at_customers_qty": crates_at_customers,
        "crates_at_suppliers_qty": crates_at_suppliers,
        "crate_stock_value_fcfa": crate_stock_value,
        "crate_untracked_full_qty": crate_untracked_qty,
        "crate_value_is_receivable": false,
        "legacy_packaging_engine_active": false,
        "confirmed_revenue_fcfa": confirmed_ca,
        "local_revenue_fcfa": ca - confirmed_ca,
        "branch_id": a.branch_id
    }))
}

pub fn get_inventory(conn: &Connection, payload: &Value) -> CoreResult<Value> {
    let a = actor(conn, payload)?;
    let mut stmt = conn.prepare(
        "SELECT v.id, v.name, v.brand, v.volume_label, v.container_type, v.material, v.pack_size, v.retail_bottle_fcfa, v.retail_case_fcfa, v.wholesale_case_fcfa, v.bottle_profile_id, v.crate_type_id, bp.compatibility_group_id, s.full_units_qty, s.min_alert_units, v.category FROM stock_balances s JOIN product_variants v ON v.id = s.variant_id LEFT JOIN bottle_profiles bp ON bp.id = v.bottle_profile_id WHERE s.branch_id = ?1 AND v.is_active = 1 ORDER BY v.name",
    )?;
    let rows = stmt.query_map(params![a.branch_id], |r| {
        Ok(json!({
            "variant_id": r.get::<_, String>(0)?,
            "name": r.get::<_, String>(1)?,
            "brand": r.get::<_, String>(2)?,
            "volume_label": r.get::<_, String>(3)?,
            "container_type": r.get::<_, String>(4)?,
            "material": r.get::<_, String>(5)?,
            "pack_size": r.get::<_, i64>(6)?,
            "retail_bottle_fcfa": r.get::<_, i64>(7)?,
            "retail_case_fcfa": r.get::<_, i64>(8)?,
            "wholesale_case_fcfa": r.get::<_, i64>(9)?,
            "bottle_profile_id": r.get::<_, Option<String>>(10)?,
            "crate_type_id": r.get::<_, Option<String>>(11)?,
            "compatibility_group_id": r.get::<_, Option<String>>(12)?,
            "full_units_qty": r.get::<_, i64>(13)?,
            "min_alert_units": r.get::<_, i64>(14)?,
            "category": r.get::<_, Option<String>>(15)?
        }))
    })?;
    let mut items = Vec::new();
    for row in rows { items.push(row?); }
    let empty_bottles: i64 = conn.query_row("SELECT COALESCE(SUM(qty),0) FROM empty_bottle_stock WHERE branch_id = ?1", params![a.branch_id], |r| r.get(0))?;
    let empty_crates: i64 = conn.query_row("SELECT COALESCE(SUM(empty_qty),0) FROM crate_stock WHERE branch_id = ?1", params![a.branch_id], |r| r.get(0))?;
    Ok(json!({"full_drinks": items, "empty_bottles_qty": empty_bottles, "empty_crates_qty": empty_crates}))
}

pub fn get_customer_account(conn: &Connection, payload: &Value) -> CoreResult<Value> {
    let _ = actor(conn, payload)?;
    let id = field_str(payload, "customer_id")?;
    let row: Option<(String, i64, i64, i64, i64)> = conn.query_row(
        "SELECT name, credit_limit_fcfa, total_purchases_fcfa, total_paid_fcfa, financial_balance_fcfa FROM customers WHERE id = ?1",
        params![id], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)),
    ).optional()?;
    let (name, limit, purchases, paid, balance) = row.ok_or_else(|| CoreError::new("CUSTOMER_NOT_FOUND", "Client introuvable."))?;
    let available = (limit - balance).max(0);
    let mut stmt = conn.prepare("SELECT item_kind, ref_id, direction, qty, occurred_at FROM partner_packaging_ledger WHERE partner_type = 'CUSTOMER' AND partner_id = ?1 ORDER BY occurred_at DESC LIMIT 50")?;
    let ledger: Vec<Value> = stmt.query_map(params![id], |r| Ok(json!({
        "item_kind": r.get::<_, String>(0)?, "ref_id": r.get::<_, String>(1)?,
        "direction": r.get::<_, String>(2)?, "qty": r.get::<_, i64>(3)?, "occurred_at": r.get::<_, i64>(4)?,
        "is_legacy": true
    })))?.collect::<Result<_, _>>()?;
    let crate_account = crate::crates::get_customer_crate_balance(conn, &{
        let mut scoped = payload.clone();
        scoped["customer_id"] = json!(id);
        scoped
    })?;
    Ok(json!({
        // Compte commercial : ventes, paiements, créances. Jamais de casiers.
        "customer_id": id, "name": name, "credit_limit_fcfa": limit,
        "outstanding_fcfa": balance, "available_credit_fcfa": available,
        "total_purchases_fcfa": purchases, "total_paid_fcfa": paid,
        // Compte physique des casiers : suivi séparé, valeur indicative.
        "crate_account": crate_account,
        "crate_balance_qty": crate_account["total_balance_qty"].clone(),
        "crate_indicative_value_fcfa": crate_account["indicative_value_fcfa"].clone(),
        "crate_balance_is_financial_debt": false,
        // Historique gelé de l'ancien moteur, en lecture seule.
        "packaging_ledger": ledger,
        "legacy_packaging_ledger": true
    }))
}

pub fn get_customers(conn: &Connection, payload: &Value) -> CoreResult<Value> {
    let _ = actor(conn, payload)?;
    let mut stmt = conn.prepare(
        "SELECT id, name, phone, neighborhood, credit_limit_fcfa, financial_balance_fcfa FROM customers WHERE is_archived = 0 ORDER BY name",
    )?;
    let items: Vec<Value> = stmt.query_map([], |r| Ok(json!({
        "id": r.get::<_, String>(0)?, "name": r.get::<_, String>(1)?,
        "phone": r.get::<_, String>(2)?, "neighborhood": r.get::<_, String>(3)?,
        "credit_limit_fcfa": r.get::<_, i64>(4)?,
        "financial_balance_fcfa": r.get::<_, i64>(5)?
    })))?.collect::<Result<_, _>>()?;
    Ok(json!({"customers": items}))
}

pub fn get_suppliers(conn: &Connection, payload: &Value) -> CoreResult<Value> {
    let _ = actor(conn, payload)?;
    let mut stmt = conn.prepare(
        "SELECT id, name, phone, financial_debt_fcfa FROM suppliers WHERE is_archived = 0 ORDER BY name",
    )?;
    let items: Vec<Value> = stmt.query_map([], |r| Ok(json!({
        "id": r.get::<_, String>(0)?, "name": r.get::<_, String>(1)?,
        "phone": r.get::<_, String>(2)?, "debt_fcfa": r.get::<_, i64>(3)?
    })))?.collect::<Result<_, _>>()?;
    Ok(json!({"suppliers": items}))
}

pub fn get_sales(conn: &Connection, payload: &Value) -> CoreResult<Value> {
    let a = actor(conn, payload)?;
    let mut stmt = conn.prepare(
        "SELECT id, receipt_number, customer_name, total_fcfa, credit_fcfa, sync_status, occurred_at FROM sales WHERE branch_id = ?1 ORDER BY occurred_at DESC LIMIT 30",
    )?;
    let items: Vec<Value> = stmt.query_map(params![a.branch_id], |r| Ok(json!({
        "id": r.get::<_, String>(0)?, "receipt_number": r.get::<_, String>(1)?,
        "customer_name": r.get::<_, String>(2)?, "total_fcfa": r.get::<_, i64>(3)?,
        "credit_fcfa": r.get::<_, i64>(4)?, "sync_status": r.get::<_, String>(5)?,
        "occurred_at": r.get::<_, i64>(6)?
    })))?.collect::<Result<_, _>>()?;
    Ok(json!({"sales": items}))
}

pub fn get_supplier_account(conn: &Connection, payload: &Value) -> CoreResult<Value> {
    let _ = actor(conn, payload)?;
    let id = field_str(payload, "supplier_id")?;
    let row: Option<(String, i64, i64, i64)> = conn.query_row(
        "SELECT name, total_purchased_fcfa, total_paid_fcfa, financial_debt_fcfa FROM suppliers WHERE id = ?1",
        params![id], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
    ).optional()?;
    let (name, purchased, paid, debt) = row.ok_or_else(|| CoreError::new("SUPPLIER_NOT_FOUND", "Fournisseur introuvable."))?;
    let crate_account = crate::crates::get_supplier_crate_balance(conn, &{
        let mut scoped = payload.clone();
        scoped["supplier_id"] = json!(id);
        scoped
    })?;
    Ok(json!({
        // Compte commercial : achats, paiements, dette. Jamais de casiers.
        "supplier_id": id, "name": name, "total_purchased_fcfa": purchased,
        "total_paid_fcfa": paid, "debt_fcfa": debt,
        // Compte physique des casiers : casiers pleins reçus, vides remis, écart.
        "crate_account": crate_account,
        "crate_balance_qty": crate_account["total_balance_qty"].clone(),
        "crate_indicative_value_fcfa": crate_account["indicative_value_fcfa"].clone(),
        "crate_balance_is_financial_debt": false
    }))
}

/// Inventaire physique : la correction est une écriture justifiée, jamais une
/// modification silencieuse du stock.
pub fn record_inventory_count(conn: &mut Connection, payload: &Value) -> CoreResult<Value> {
    let a = actor(conn, payload)?;
    if !matches!(a.role.as_str(), "OWNER" | "ADMIN" | "MANAGER") {
        return Err(CoreError::new("ROLE_FORBIDDEN", "Inventaire réservé aux rôles de gestion."));
    }
    let variant_id = field_str(payload, "variant_id")?;
    let counted = field_i64(payload, "counted_qty")?;
    if counted < 0 {
        return Err(CoreError::new("INVALID_QUANTITY", "Quantité comptée invalide."));
    }
    let reason = field_str(payload, "reason")?.trim().to_string();
    if reason.len() < 3 {
        return Err(CoreError::new("REASON_REQUIRED", "Un motif d'inventaire est obligatoire."));
    }
    let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
    let current: i64 = tx.query_row(
        "SELECT full_units_qty FROM stock_balances WHERE branch_id=?1 AND variant_id=?2",
        params![a.branch_id, variant_id], |r| r.get(0),
    ).optional()?.unwrap_or(0);
    let difference = counted - current;
    let id = new_id("inv");
    tx.execute(
        "INSERT INTO inventory_counts (id, branch_id, variant_id, counted_qty, difference_qty, reason, user_id, device_id, occurred_at)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9)",
        params![id, a.branch_id, variant_id, counted, difference, reason, a.user_id, a.device_id, now_ms()],
    )?;
    tx.execute(
        "INSERT INTO stock_movements (id, branch_id, variant_id, movement_type, qty, reason, document_id, user_id, device_id, occurred_at)
         VALUES (?1,?2,?3,'INVENTORY_ADJUSTMENT',?4,?5,?6,?7,?8,?9)",
        params![new_id("smov"), a.branch_id, variant_id, difference, reason, id, a.user_id, a.device_id, now_ms()],
    )?;
    tx.execute(
        "INSERT INTO stock_balances (branch_id, variant_id, full_units_qty, min_alert_units, server_revision)
         VALUES (?1,?2,?3,12,1)
         ON CONFLICT(branch_id, variant_id) DO UPDATE SET full_units_qty=excluded.full_units_qty,
         server_revision=server_revision+1",
        params![a.branch_id, variant_id, counted],
    )?;
    db::enqueue(
        &tx, &a.tenant_id, &a.branch_id, &a.device_id, &a.user_id, "INVENTORY_ADJUSTMENT",
        &format!("{variant_id}: {current} -> {counted}"), &payload.to_string(), 1, &new_id("idem"),
    )?;
    db::audit(&tx, &a.branch_id, &a.user_id, "INVENTORY_ADJUSTMENT", &format!("{variant_id}: {difference}"))?;
    tx.commit()?;
    Ok(json!({
        "inventory_id": id, "variant_id": variant_id,
        "previous_qty": current, "counted_qty": counted, "difference_qty": difference
    }))
}

/// Historique des mouvements de stock, consultable par référence.
pub fn get_stock_movements(conn: &Connection, payload: &Value) -> CoreResult<Value> {
    let a = actor(conn, payload)?;
    let variant_id = field_str_opt(payload, "variant_id");
    let sql = "SELECT id, variant_id, movement_type, qty, reason, document_id, user_id, device_id, occurred_at
               FROM stock_movements WHERE branch_id=?1
               AND (?2 IS NULL OR variant_id=?2) ORDER BY occurred_at DESC LIMIT 100";
    let mut stmt = conn.prepare(sql)?;
    let rows = stmt.query_map(params![a.branch_id, variant_id], |r| Ok(json!({
        "movement_id": r.get::<_, String>(0)?,
        "variant_id": r.get::<_, String>(1)?,
        "movement_type": r.get::<_, String>(2)?,
        "qty": r.get::<_, i64>(3)?,
        "reason": r.get::<_, String>(4)?,
        "document_id": r.get::<_, String>(5)?,
        "user_id": r.get::<_, String>(6)?,
        "device_id": r.get::<_, String>(7)?,
        "occurred_at": r.get::<_, i64>(8)?
    })))?;
    let mut movements = Vec::new();
    for row in rows { movements.push(row?); }
    Ok(json!({ "movements": movements }))
}

pub fn get_pending_operations(conn: &Connection, payload: &Value) -> CoreResult<Value> {
    let a = actor(conn, payload)?;
    let limit = field_i64_or(payload, "limit", 50).clamp(1, 200);
    let after = field_i64_or(payload, "after_sequence", 0);
    let mut stmt = conn.prepare(
        "SELECT operation_id, tenant_id, branch_id, device_id, user_id, local_sequence, schema_version, command_type, summary, payload_json, base_revision, idempotency_key, status, attempt_count, server_sequence
         FROM sync_outbox
         WHERE status IN ('PENDING','FAILED') AND next_retry_at <= ?1 AND local_sequence > ?2
         ORDER BY CASE command_type
             WHEN 'saveProductVariant' THEN 0 WHEN 'createCrateType' THEN 0
             WHEN 'updateVariantPrices' THEN 0 WHEN 'archiveProductVariant' THEN 0
             WHEN 'createPurchaseOrder' THEN 1 WHEN 'receivePurchase' THEN 2
             WHEN 'createMixedOrder' THEN 3 WHEN 'createSale' THEN 3
             WHEN 'registerPackagingReturn' THEN 4 WHEN 'recordPayment' THEN 4
             WHEN 'openCashSession' THEN 4 WHEN 'recordCashMovement' THEN 4
             WHEN 'closeCashSession' THEN 4 ELSE 5 END,
             local_sequence ASC LIMIT ?3",
    )?;
    let now = now_ms();
    let items: Vec<Value> = stmt.query_map(params![now, after, limit], |r| Ok(json!({
        "operation_id": r.get::<_, String>(0)?, "tenant_id": r.get::<_, String>(1)?,
        "branch_id": r.get::<_, String>(2)?, "device_id": r.get::<_, String>(3)?,
        "user_id": r.get::<_, String>(4)?, "local_sequence": r.get::<_, i64>(5)?,
        "schema_version": r.get::<_, i64>(6)?, "command_type": r.get::<_, String>(7)?,
        "summary": r.get::<_, String>(8)?, "payload_json": r.get::<_, String>(9)?,
        "base_revision": r.get::<_, i64>(10)?, "idempotency_key": r.get::<_, String>(11)?,
        "status": r.get::<_, String>(12)?, "attempt_count": r.get::<_, i64>(13)?,
        "server_sequence": r.get::<_, Option<i64>>(14)?
    })))?.collect::<Result<_, _>>()?;
    Ok(json!({"branch_id": a.branch_id, "operations": items}))
}

pub fn start_synchronization(conn: &mut Connection, payload: &Value) -> CoreResult<Value> {
    let a = actor(conn, payload)?;
    let pending = get_pending_operations(conn, payload)?;
    let ops = field_array(&pending, "operations")?.clone();
    let ids: Vec<String> = ops.iter().filter_map(|o| o.get("operation_id").and_then(Value::as_str).map(str::to_string)).collect();
    if !ids.is_empty() {
        let placeholders = vec!["?"; ids.len()].join(",");
        let sql = format!("UPDATE sync_outbox SET status = 'SYNCING', attempt_count = attempt_count + 1, next_retry_at = 0 WHERE operation_id IN ({placeholders}) AND status IN ('PENDING','FAILED')");
        let bound: Vec<&dyn rusqlite::ToSql> = ids.iter().map(|id| id.as_str() as &dyn rusqlite::ToSql).collect();
        conn.execute(&sql, bound.as_slice())?;
    }
    let cursor: (i64, String) = conn.query_row(
        "SELECT server_sequence, server_url FROM sync_cursors WHERE branch_id = ?1",
        params![a.branch_id],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )?;
    let last_sequence = ops.last().and_then(|op| op.get("local_sequence").and_then(Value::as_i64)).unwrap_or(0);
    let limit = field_i64_or(payload, "limit", 50).clamp(1, 200);
    Ok(json!({
        "operations": ops,
        "server_sequence": cursor.0,
        "server_url": cursor.1,
        "branch_id": a.branch_id,
        "device_id": a.device_id,
        "has_more": ops.len() as i64 == limit,
        "next_after_sequence": last_sequence
    }))
}

pub fn configure_sync_server(conn: &mut Connection, payload: &Value) -> CoreResult<Value> {
    let a = actor(conn, payload)?;
    let url = field_str(payload, "server_url")?.trim().to_string();
    if !url.is_empty() && !url.starts_with("https://") {
        return Err(CoreError::new("HTTPS_REQUIRED", "L'URL du serveur Axum doit utiliser HTTPS."));
    }
    conn.execute(
        "UPDATE sync_cursors SET server_url = ?1 WHERE branch_id = ?2",
        params![url, a.branch_id],
    )?;
    Ok(json!({"server_url": url, "https_only": true}))
}

pub fn get_synchronization_status(conn: &Connection, payload: &Value) -> CoreResult<Value> {
    let a = actor(conn, payload)?;
    let counts = conn.query_row(
        "SELECT SUM(CASE WHEN status='PENDING' THEN 1 ELSE 0 END), SUM(CASE WHEN status='SYNCING' THEN 1 ELSE 0 END), SUM(CASE WHEN status='SYNCED' THEN 1 ELSE 0 END), SUM(CASE WHEN status='CONFLICT' THEN 1 ELSE 0 END), SUM(CASE WHEN status='FAILED' THEN 1 ELSE 0 END) FROM sync_outbox",
        [], |r| Ok((r.get::<_, i64>(0)?.unwrap_or(0), r.get::<_, i64>(1)?.unwrap_or(0), r.get::<_, i64>(2)?.unwrap_or(0), r.get::<_, i64>(3)?.unwrap_or(0), r.get::<_, i64>(4)?.unwrap_or(0))),
    )?;
    let cursor: (i64, String, Option<i64>) = conn.query_row(
        "SELECT server_sequence, server_url, last_sync_at FROM sync_cursors WHERE branch_id = ?1",
        params![a.branch_id],
        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
    ).unwrap_or((0, String::new(), None));
    let unconfirmed: i64 = conn.query_row(
        "SELECT COUNT(*) FROM sales WHERE is_offline = 1 AND sync_status = 'PENDING'",
        [], |r| r.get(0),
    ).unwrap_or(0);
    let policy: String = conn.query_row(
        "SELECT offline_policy FROM tenants WHERE id = ?1", params![a.tenant_id], |r| r.get(0),
    ).unwrap_or_else(|_| "ALLOW_WITH_WARNING".to_string());
    Ok(json!({
        "pending": counts.0, "syncing": counts.1, "synced": counts.2,
        "conflicts": counts.3, "failed": counts.4,
        "server_sequence": cursor.0, "server_url": cursor.1,
        "last_sync_at": cursor.2, "branch_id": a.branch_id, "device_id": a.device_id,
        "offline_policy": policy, "unconfirmed_offline_sales": unconfirmed
    }))
}

pub fn apply_sync_ack(conn: &mut Connection, payload: &Value) -> CoreResult<Value> {
    let _ = actor(conn, payload)?;
    let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
    let now = now_ms();
    for ack in field_array(payload, "results")? {
        let op_id = field_str(ack, "operation_id")?;
        let status = field_str(ack, "status")?;
        let attempt: i64 = tx.query_row(
            "SELECT attempt_count FROM sync_outbox WHERE operation_id = ?1", params![op_id], |r| r.get(0),
        ).unwrap_or(0);
        let next_retry = backoff_ms(attempt, now);
        match status {
            "ACCEPTED" => {
                tx.execute(
                    "UPDATE sync_outbox SET status='SYNCED', synced_at=?1, last_error=NULL, server_sequence=COALESCE(?2, server_sequence) WHERE operation_id=?3",
                    params![now, ack.get("server_sequence").and_then(Value::as_i64), op_id],
                )?;
            }
            "CONFLICT" => {
                tx.execute(
                    "UPDATE sync_outbox SET status='CONFLICT', last_error=?1, server_sequence=COALESCE(?2, server_sequence) WHERE operation_id=?3",
                    params![field_str_opt(ack, "message").unwrap_or("Conflit serveur"), ack.get("server_sequence").and_then(Value::as_i64), op_id],
                )?;
                tx.execute(
                    "INSERT INTO sync_conflicts (id, operation_id, conflict_type, server_reason, status, created_at) VALUES (?1,?2,?3,?4,'OPEN',?5)",
                    params![new_id("conf"), op_id, field_str_opt(ack, "conflict_type").unwrap_or("STOCK_INSUFFICIENT_CENTRAL"), field_str_opt(ack, "message").unwrap_or("Conflit détecté côté serveur"), now],
                )?;
            }
            _ => {
                tx.execute(
                    "UPDATE sync_outbox SET status='FAILED', last_error=?1, next_retry_at=?2 WHERE operation_id=?3",
                    params![field_str_opt(ack, "message").unwrap_or("Rejet serveur"), next_retry, op_id],
                )?;
            }
        }
    }
    if let Some(seq) = payload.get("server_sequence").and_then(Value::as_i64) {
        let branch = field_str(payload.get("auth").unwrap_or(payload), "branch_id")?;
        tx.execute(
            "UPDATE sync_cursors SET server_sequence = MAX(server_sequence, ?1), last_sync_at = ?2 WHERE branch_id = ?3",
            params![seq, now, branch],
        )?;
    }
    tx.commit()?;
    Ok(json!({"applied": field_array(payload, "results")?.len()}))
}

/// Version du contrat de réplication descendante. Une version supérieure est refusée
/// sans écriture partielle : la migration doit d'abord être appliquée sur l'appareil.
pub const PULL_SCHEMA_VERSION: i64 = 1;

fn apply_remote_entity(
    tx: &Connection,
    entity_type: &str,
    entity_id: &str,
    version: i64,
    data: &Value,
    tenant_id: &str,
    branch_id: &str,
    tombstone: bool,
) -> CoreResult<()> {
    match entity_type {
        "product_variant" => {
            if tombstone {
                tx.execute("UPDATE product_variants SET is_active = 0 WHERE id = ?1", params![entity_id])?;
            } else {
                tx.execute(
                    "INSERT INTO product_variants (id, tenant_id, name, brand, sku, barcode, volume_label, container_type, material, bottle_profile_id, crate_type_id, pack_size, retail_case_fcfa, wholesale_case_fcfa, retail_bottle_fcfa, price_revision, is_active)
                     VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,1)
                     ON CONFLICT(id) DO UPDATE SET name=excluded.name, brand=excluded.brand, sku=excluded.sku,
                     barcode=excluded.barcode, volume_label=excluded.volume_label, container_type=excluded.container_type,
                     material=excluded.material, bottle_profile_id=excluded.bottle_profile_id,
                     crate_type_id=excluded.crate_type_id, pack_size=excluded.pack_size,
                     retail_case_fcfa=excluded.retail_case_fcfa, wholesale_case_fcfa=excluded.wholesale_case_fcfa,
                     retail_bottle_fcfa=excluded.retail_bottle_fcfa, price_revision=excluded.price_revision, is_active=1",
                    params![
                        entity_id, field_str_opt(data, "tenant_id").unwrap_or(tenant_id), field_str(data, "name")?,
                        field_str(data, "brand")?, field_str(data, "sku")?,
                        field_str_opt(data, "barcode"), field_str(data, "volume_label")?,
                        field_str(data, "container_type")?, field_str(data, "material")?,
                        field_str_opt(data, "bottle_profile_id"), field_str_opt(data, "crate_type_id"),
                        field_i64_or(data, "pack_size", 1), field_i64_or(data, "retail_case_fcfa", 0),
                        field_i64_or(data, "wholesale_case_fcfa", 0), field_i64_or(data, "retail_bottle_fcfa", 0),
                        version
                    ],
                )?;
            }
        }
        "customer" => {
            if tombstone {
                tx.execute("UPDATE customers SET is_archived = 1 WHERE id = ?1", params![entity_id])?;
            } else {
                tx.execute(
                    "INSERT INTO customers (id, tenant_id, name, phone, neighborhood, customer_type, default_price_tier, credit_limit_fcfa, financial_balance_fcfa, is_archived)
                     VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,0)
                     ON CONFLICT(id) DO UPDATE SET name=excluded.name, phone=excluded.phone,
                     neighborhood=excluded.neighborhood, customer_type=excluded.customer_type,
                     default_price_tier=excluded.default_price_tier, credit_limit_fcfa=excluded.credit_limit_fcfa,
                     financial_balance_fcfa=excluded.financial_balance_fcfa, is_archived=0",
                    params![
                        entity_id, field_str_opt(data, "tenant_id").unwrap_or(tenant_id), field_str(data, "name")?,
                        field_str(data, "phone")?, field_str(data, "neighborhood")?,
                        field_str(data, "customer_type")?, field_str(data, "default_price_tier")?,
                        field_i64_or(data, "credit_limit_fcfa", 0), field_i64_or(data, "financial_balance_fcfa", 0)
                    ],
                )?;
            }
        }
        "supplier" => {
            if tombstone {
                tx.execute("UPDATE suppliers SET is_archived = 1 WHERE id = ?1", params![entity_id])?;
            } else {
                tx.execute(
                    "INSERT INTO suppliers (id, tenant_id, name, phone, contact_name, city, financial_debt_fcfa, is_archived)
                     VALUES (?1,?2,?3,?4,?5,?6,?7,0)
                     ON CONFLICT(id) DO UPDATE SET name=excluded.name, phone=excluded.phone,
                     contact_name=excluded.contact_name, city=excluded.city,
                     financial_debt_fcfa=excluded.financial_debt_fcfa, is_archived=0",
                    params![
                        entity_id, field_str_opt(data, "tenant_id").unwrap_or(tenant_id), field_str(data, "name")?,
                        field_str(data, "phone")?, field_str(data, "contact_name")?,
                        field_str(data, "city")?, field_i64_or(data, "financial_debt_fcfa", 0)
                    ],
                )?;
            }
        }
        "stock_balance" | "stock_adjustment" => {
            let qty = if tombstone { 0 } else { field_i64(data, "full_units_qty")? };
            tx.execute(
                "INSERT INTO stock_balances (branch_id, variant_id, full_units_qty, min_alert_units, server_revision)
                 VALUES (?1,?2,?3,?4,?5)
                 ON CONFLICT(branch_id, variant_id) DO UPDATE SET full_units_qty=excluded.full_units_qty,
                 server_revision=excluded.server_revision",
                params![branch_id, entity_id, qty, field_i64_or(data, "min_alert_units", 12), version],
            )?;
        }
        "conflict_resolution" => {
            let conflict_id = field_str(data, "conflict_id")?;
            let resolution = field_str(data, "resolution")?;
            tx.execute(
                "UPDATE sync_conflicts SET status = ?1, resolution = ?2, resolved_at = ?3 WHERE id = ?4",
                params![resolution, resolution, now_ms(), conflict_id],
            )?;
        }
        other => return Err(CoreError::new("UNSUPPORTED_ENTITY_TYPE", format!("Type d'entité non réplicable : {other}"))),
    }
    Ok(())
}

/// Applique les mises à jour descendantes de façon atomique et versionnée.
/// Un événement déjà appliqué est ignoré ; un curseur n'avance qu'après commit.
pub fn apply_sync_updates(conn: &mut Connection, payload: &Value) -> CoreResult<Value> {
    let a = actor(conn, payload)?;
    let updates = field_array(payload, "updates")?.clone();
    let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
    let now = now_ms();
    let mut applied = 0usize;
    let mut skipped = 0usize;
    let mut max_sequence = 0i64;
    for update in &updates {
        let sequence = field_i64(update, "server_sequence")?;
        let entity_type = field_str(update, "entity_type")?;
        let entity_id = field_str(update, "entity_id")?;
        let version = field_i64(update, "version")?;
        let schema = field_i64_or(update, "schema_version", PULL_SCHEMA_VERSION);
        if schema > PULL_SCHEMA_VERSION {
            return Err(CoreError::new("PULL_SCHEMA_UNSUPPORTED", "Version du schéma de réplication non prise en charge."));
        }
        if let Some(tenant) = field_str_opt(update, "tenant_id") {
            if tenant != a.tenant_id {
                return Err(CoreError::new("TENANT_MISMATCH", "Événement reçu pour un autre établissement."));
            }
        }
        if let Some(branch) = field_str_opt(update, "branch_id") {
            if branch != a.branch_id {
                return Err(CoreError::new("BRANCH_MISMATCH", "Événement reçu pour un autre point de vente."));
            }
        }
        let current: Option<(i64, i64)> = tx.query_row(
            "SELECT version, last_sequence FROM replication_state WHERE entity_type = ?1 AND entity_id = ?2",
            params![entity_type, entity_id], |r| Ok((r.get(0)?, r.get(1)?)),
        ).optional()?;
        if let Some((known_version, known_sequence)) = current {
            if version <= known_version || sequence <= known_sequence {
                skipped += 1;
                continue;
            }
        }
        let data = update.get("payload").unwrap_or(update);
        apply_remote_entity(
            &tx, entity_type, entity_id, version, data,
            &a.tenant_id, &a.branch_id, field_bool(update, "tombstone", false),
        )?;
        tx.execute(
            "INSERT INTO replication_state (entity_type, entity_id, version, tombstone, last_sequence, updated_at)
             VALUES (?1,?2,?3,?4,?5,?6)
             ON CONFLICT(entity_type, entity_id) DO UPDATE SET version=excluded.version,
             tombstone=excluded.tombstone, last_sequence=excluded.last_sequence, updated_at=excluded.updated_at",
            params![entity_type, entity_id, version, if field_bool(update, "tombstone", false) { 1 } else { 0 }, sequence, now],
        )?;
        applied += 1;
        max_sequence = max_sequence.max(sequence);
    }
    if let Some(cursor) = payload.get("server_sequence").and_then(Value::as_i64) {
        max_sequence = max_sequence.max(cursor);
    }
    if max_sequence > 0 {
        tx.execute(
            "UPDATE sync_cursors SET server_sequence = MAX(server_sequence, ?1), last_sync_at = ?2 WHERE branch_id = ?3",
            params![max_sequence, now, a.branch_id],
        )?;
    }
    tx.commit()?;
    Ok(json!({"applied": applied, "skipped": skipped, "server_sequence": max_sequence}))
}

/// Résolution contrôlée : jamais de suppression d'opération ni de réécriture historique.
pub fn resolve_conflict(conn: &mut Connection, payload: &Value) -> CoreResult<Value> {
    let a = actor(conn, payload)?;
    if !matches!(a.role.as_str(), "OWNER" | "ADMIN" | "MANAGER") {
        return Err(CoreError::new("ROLE_FORBIDDEN", "Seuls OWNER, ADMIN et MANAGER peuvent résoudre un conflit."));
    }
    let conflict_id = field_str(payload, "conflict_id")?;
    let resolution = field_str(payload, "resolution")?;
    if !matches!(resolution, "ACKNOWLEDGE_SERVER" | "ESCALATE") {
        return Err(CoreError::new("INVALID_RESOLUTION", "Résolution inconnue."));
    }
    let status = if resolution == "ESCALATE" { "ESCALATED" } else { "RESOLVED" };
    let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
    let existing: Option<String> = tx.query_row(
        "SELECT status FROM sync_conflicts WHERE id = ?1", params![conflict_id], |r| r.get(0),
    ).optional()?;
    let existing = existing.ok_or_else(|| CoreError::new("CONFLICT_NOT_FOUND", "Conflit introuvable."))?;
    if existing != "OPEN" {
        return Err(CoreError::new("CONFLICT_ALREADY_RESOLVED", "Ce conflit a déjà été traité."));
    }
    let rows = tx.execute(
        "UPDATE sync_conflicts SET status = ?1, resolution = ?2, resolved_by = ?3, resolved_at = ?4 WHERE id = ?5",
        params![status, resolution, a.user_id, now_ms(), conflict_id],
    )?;
    db::audit(&tx, &a.branch_id, &a.user_id, "RESOLVE_CONFLICT", &format!("{conflict_id}: {resolution}"))?;
    tx.commit()?;
    if rows == 0 {
        return Err(CoreError::new("CONFLICT_NOT_FOUND", "Conflit introuvable."));
    }
    Ok(json!({"conflict_id": conflict_id, "status": status, "operation_kept": true}))
}

/// Quota de stock réservé par appareil : la vente hors ligne ne peut pas dépasser
/// la part attribuée, sans jamais garantir un stock global hors ligne.
pub fn configure_device_quota(conn: &mut Connection, payload: &Value) -> CoreResult<Value> {
    let a = actor(conn, payload)?;
    if !matches!(a.role.as_str(), "OWNER" | "ADMIN" | "MANAGER") {
        return Err(CoreError::new("ROLE_FORBIDDEN", "Seuls OWNER, ADMIN et MANAGER peuvent configurer les quotas."));
    }
    let branch_id = field_str(payload, "branch_id")?;
    let variant_id = field_str(payload, "variant_id")?;
    let device_id = field_str(payload, "device_id")?;
    let reserved = field_i64(payload, "reserved_qty")?;
    if reserved < 0 {
        return Err(CoreError::new("INVALID_QUOTA", "Le quota doit être positif ou nul."));
    }
    conn.execute(
        "INSERT INTO device_stock_quotas (branch_id, variant_id, device_id, reserved_qty, updated_at)
         VALUES (?1,?2,?3,?4,?5)
         ON CONFLICT(branch_id, variant_id, device_id) DO UPDATE SET reserved_qty=excluded.reserved_qty, updated_at=excluded.updated_at",
        params![branch_id, variant_id, device_id, reserved, now_ms()],
    )?;
    db::audit(conn, &a.branch_id, &a.user_id, "CONFIGURE_DEVICE_QUOTA", &format!("{variant_id}/{device_id}: {reserved}"))?;
    Ok(json!({"branch_id": branch_id, "variant_id": variant_id, "device_id": device_id, "reserved_qty": reserved}))
}

// ---- LOT 6/9/10 : catalogue, achats, caisse et rapports locaux ----

/// Référentiel des casiers physiques avec leurs trois valeurs distinctes.
pub fn get_crate_types(conn: &Connection, payload: &Value) -> CoreResult<Value> {
    let a = actor(conn, payload)?;
    let mut stmt = conn.prepare(
        "SELECT ct.id, ct.code, ct.capacity, ct.compatibility_group_id, ct.allow_half_case,
                ct.replacement_value_fcfa, COALESCE(cs.empty_qty, 0),
                ct.name, ct.compartments, ct.dimensions, ct.deposit_value_fcfa,
                ct.physical_profile_id, ct.is_reusable,
                ct.acquisition_value_fcfa, ct.reference_value_fcfa, ct.empty_sale_price_fcfa,
                ct.status, ct.is_physical_crate, COALESCE(cs.full_qty, 0)
         FROM crate_types ct
         LEFT JOIN crate_stock cs ON cs.crate_id = ct.id AND cs.branch_id = ?1
         ORDER BY ct.code",
    )?;
    let rows = stmt.query_map(params![a.branch_id], |r| {
        let reference: i64 = r.get(14)?;
        let full: i64 = r.get(18)?;
        let empty: i64 = r.get(6)?;
        Ok(json!({
            "crate_type_id": r.get::<_, String>(0)?,
            "code": r.get::<_, String>(1)?,
            "capacity": r.get::<_, i64>(2)?,
            "compatibility_group_id": r.get::<_, String>(3)?,
            "allow_half_case": r.get::<_, i64>(4)? == 1,
            "replacement_value_fcfa": r.get::<_, i64>(5)?,
            "empty_crates_qty": empty,
            "name": r.get::<_, Option<String>>(7)?.unwrap_or_default(),
            "compartments": r.get::<_, i64>(8)?,
            "dimensions": r.get::<_, Option<String>>(9)?,
            "deposit_value_fcfa": r.get::<_, i64>(10)?,
            "deposit_value_is_legacy": true,
            "physical_profile_id": r.get::<_, Option<String>>(11)?,
            "is_reusable": r.get::<_, i64>(12)? == 1,
            "acquisition_value_fcfa": r.get::<_, i64>(13)?,
            "reference_value_fcfa": reference,
            "empty_sale_price_fcfa": r.get::<_, i64>(15)?,
            "status": r.get::<_, String>(16)?,
            "is_physical_crate": r.get::<_, i64>(17)? == 1,
            "full_crates_qty": full,
            "stock_value_fcfa": (full + empty).saturating_mul(reference)
        }))
    })?;
    let mut items = Vec::new();
    for row in rows { items.push(row?); }
    Ok(json!({
        "crate_types": items,
        "engine": "PHYSICAL_CRATES",
        "note": "Le casier est un bien physique suivi. Sa valeur est distincte du prix de la boisson."
    }))
}

/// Compte d'un partenaire : le **solde physique des casiers** (nouveau moteur)
/// est présenté à part de l'historique d'emballages gelé de l'ancien moteur.
/// Aucune valeur financière n'est créée : le solde physique n'est jamais une
/// créance et n'entre jamais dans le compte commercial.
pub fn get_packaging_account(conn: &Connection, payload: &Value) -> CoreResult<Value> {
    let a = actor(conn, payload)?;
    let partner_type = field_str(payload, "partner_type")?;
    let partner_id = field_str(payload, "partner_id")?;
    if !matches!(partner_type, "CUSTOMER" | "SUPPLIER") {
        return Err(CoreError::new("INVALID_PARTNER", "Type de partenaire inconnu."));
    }
    let physical = if partner_type == "CUSTOMER" {
        let mut scoped = payload.clone();
        scoped["customer_id"] = json!(partner_id);
        crate::crates::get_customer_crate_balance(conn, &scoped)?
    } else {
        let mut scoped = payload.clone();
        scoped["supplier_id"] = json!(partner_id);
        crate::crates::get_supplier_crate_balance(conn, &scoped)?
    };

    let mut stmt = conn.prepare(
        "SELECT item_kind, ref_id, SUM(expected_qty), SUM(received_qty)
         FROM packaging_expectations
         WHERE branch_id=?1 AND partner_type=?2 AND partner_id=?3
         GROUP BY item_kind, ref_id ORDER BY item_kind, ref_id",
    )?;
    let rows = stmt.query_map(params![a.branch_id, partner_type, partner_id], |r| {
        let expected: i64 = r.get(2)?;
        let received: i64 = r.get(3)?;
        let status = if received >= expected { "RETURNED" } else if received > 0 { "PARTIAL" } else { "EXPECTED" };
        Ok(json!({
            "item_kind": r.get::<_, String>(0)?,
            "ref_id": r.get::<_, String>(1)?,
            "expected_qty": expected,
            "received_qty": received,
            "outstanding_qty": expected - received,
            "status": status,
            "financial_value_fcfa": 0
        }))
    })?;
    let mut legacy_balances = Vec::new();
    for row in rows { legacy_balances.push(row?); }

    let mut stmt = conn.prepare(
        "SELECT id, item_kind, ref_id, direction, qty, document_id, reason, occurred_at,
                COALESCE(user_id,''), COALESCE(device_id,''), COALESCE(reversal_of,''), engine
         FROM partner_packaging_ledger
         WHERE branch_id=?1 AND partner_type=?2 AND partner_id=?3
         ORDER BY occurred_at DESC LIMIT 200",
    )?;
    let rows = stmt.query_map(params![a.branch_id, partner_type, partner_id], |r| Ok(json!({
        "movement_id": r.get::<_, String>(0)?,
        "item_kind": r.get::<_, String>(1)?,
        "ref_id": r.get::<_, String>(2)?,
        "direction": r.get::<_, String>(3)?,
        "qty": r.get::<_, i64>(4)?,
        "document_id": r.get::<_, String>(5)?,
        "reason": r.get::<_, String>(6)?,
        "occurred_at": r.get::<_, i64>(7)?,
        "user_id": r.get::<_, String>(8)?,
        "device_id": r.get::<_, String>(9)?,
        "reversal_of": r.get::<_, String>(10)?,
        "engine": r.get::<_, String>(11)?
    })))?;
    let mut legacy_movements = Vec::new();
    for row in rows { legacy_movements.push(row?); }

    Ok(json!({
        "partner_type": partner_type,
        "partner_id": partner_id,
        "engine": "PHYSICAL_CRATES",
        "crate_account": physical.clone(),
        "balances": physical["balances"].clone(),
        "movements": physical["movements"].clone(),
        "total_balance_qty": physical["total_balance_qty"].clone(),
        "indicative_value_fcfa": physical["indicative_value_fcfa"].clone(),
        "financial_value_fcfa": 0,
        "has_billing_rule": false,
        "is_financial_debt": false,
        "counts_as_receivable": false,
        "is_immutable_ledger": true,
        "legacy": {
            "engine": "PACKAGING_OBLIGATIONS_0015",
            "state": "FROZEN",
            "balances": legacy_balances,
            "movements": legacy_movements,
            "bottle_tracking": "DISABLED",
            "converted_to_financial_debt": false,
            "note": "Historique conservé en lecture seule. Aucune nouvelle obligation, aucune conversion en dette, aucun retour de bouteilles n'est possible."
        }
    }))
}

/// Toute correction est une écriture inverse : aucun mouvement (physique ou
/// historique) n'est réécrit ni supprimé.
pub fn correct_packaging_movement(conn: &mut Connection, payload: &Value) -> CoreResult<Value> {
    let a = actor(conn, payload)?;
    if !matches!(a.role.as_str(), "OWNER" | "ADMIN" | "MANAGER") {
        return Err(CoreError::new("ROLE_FORBIDDEN", "Correction réservée aux rôles de gestion."));
    }
    let movement_id = field_str(payload, "movement_id")?;
    let reason = field_str(payload, "reason")?.trim().to_string();
    if reason.len() < 3 {
        return Err(CoreError::new("REASON_REQUIRED", "Un motif de correction est obligatoire."));
    }
    let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
    let original = tx.query_row(
        "SELECT crate_id, movement_type, full_before, full_after, empty_before, empty_after,
                direction, partner_type, partner_id, partner_delta
         FROM crate_movements WHERE id=?1 AND branch_id=?2",
        params![movement_id, a.branch_id],
        |r| Ok((
            r.get::<_, String>(0)?, r.get::<_, String>(1)?,
            r.get::<_, i64>(2)?, r.get::<_, i64>(3)?,
            r.get::<_, i64>(4)?, r.get::<_, i64>(5)?,
            r.get::<_, String>(6)?, r.get::<_, Option<String>>(7)?,
            r.get::<_, Option<String>>(8)?, r.get::<_, i64>(9)?,
        )),
    ).optional()?;
    if let Some((crate_id, movement_type, full_before, full_after, empty_before, empty_after,
                direction, partner_type, partner_id, partner_delta)) = original {
        let already: i64 = tx.query_row(
            "SELECT COUNT(*) FROM crate_movements WHERE reversal_of=?1",
            params![movement_id], |r| r.get(0),
        )?;
        if already > 0 {
            return Err(CoreError::new("MOVEMENT_ALREADY_REVERSED", "Ce mouvement a déjà été corrigé."));
        }
        let full_delta = full_before - full_after;
        let empty_delta = empty_before - empty_after;
        let qty = full_delta.abs().max(empty_delta.abs()).max(1);
        let inverse_type = if direction == "OUT" { "adjustment_in" } else { "adjustment_out" };
        let document_id = new_id("crate_corr");
        let (movement, _) = crate::crates::apply_movement(
            &tx,
            &a,
            &crate::crates::CrateMove {
                crate_id: &crate_id,
                movement_type: inverse_type,
                qty,
                full_delta,
                empty_delta,
                partner_type: partner_type.as_deref(),
                partner_id: partner_id.as_deref(),
                partner_delta: -partner_delta,
                document_id: &document_id,
                idempotency_key: &format!("correction:{movement_id}"),
                reason: &format!("Correction de {movement_type} : {reason}"),
                origin: "DEPOT",
                destination: "DEPOT",
                source: "RUNTIME",
                reversal_of: Some(movement_id),
            },
        )?;
        db::audit(&tx, &a.branch_id, &a.user_id, "CORRECT_CRATE_MOVEMENT", movement_id)?;
        tx.commit()?;
        return Ok(json!({
            "engine": "PHYSICAL_CRATES",
            "correction_id": movement["movement_id"].clone(),
            "reversal_of": movement_id,
            "original_kept": true,
            "movement": movement
        }));
    }

    // Mouvement purement historique de l'ancien moteur : correction tracée, sans
    // effet sur le stock physique et sans remise en route du moteur gelé.
    let legacy: Option<(String, String, String, String, i64, String)> = tx.query_row(
        "SELECT partner_type, partner_id, item_kind, ref_id, qty, direction
         FROM partner_packaging_ledger WHERE id=?1",
        params![movement_id],
        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?)),
    ).optional()?;
    let (partner_type, partner_id, item_kind, ref_id, qty, direction) =
        legacy.ok_or_else(|| CoreError::new("MOVEMENT_NOT_FOUND", "Mouvement introuvable."))?;
    let already: i64 = tx.query_row(
        "SELECT COUNT(*) FROM partner_packaging_ledger WHERE reversal_of=?1",
        params![movement_id], |r| r.get(0),
    )?;
    if already > 0 {
        return Err(CoreError::new("MOVEMENT_ALREADY_REVERSED", "Ce mouvement a déjà été corrigé."));
    }
    let inverse = if direction == "IN_FROM_PARTNER" { "OUT_TO_PARTNER" } else { "IN_FROM_PARTNER" };
    let correction_id = new_id("pkgcorr");
    tx.execute(
        "INSERT INTO partner_packaging_ledger
         (id, branch_id, partner_type, partner_id, item_kind, ref_id, direction, qty, document_id, reason, occurred_at, user_id, device_id, reversal_of, engine)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,'LEGACY_CORRECTION')",
        params![correction_id, a.branch_id, partner_type, partner_id, item_kind, ref_id,
            inverse, qty, new_id("doc"), reason, now_ms(), a.user_id, a.device_id, movement_id],
    )?;
    db::audit(&tx, &a.branch_id, &a.user_id, "CORRECT_LEGACY_PACKAGING", movement_id)?;
    tx.commit()?;
    Ok(json!({
        "engine": "LEGACY_0015_READ_ONLY",
        "correction_id": correction_id,
        "reversal_of": movement_id,
        "original_kept": true,
        "affects_physical_stock": false
    }))
}

/// Bons de commande fournisseur avec montant déjà réceptionné.
pub fn get_purchase_orders(conn: &Connection, payload: &Value) -> CoreResult<Value> {
    let a = actor(conn, payload)?;
    let mut stmt = conn.prepare(
        "SELECT po.id, po.reference, po.total_fcfa, po.status, po.created_at, s.name,
                COALESCE((SELECT SUM(total_fcfa) FROM purchase_receipts pr WHERE pr.order_id = po.id), 0)
         FROM purchase_orders po JOIN suppliers s ON s.id = po.supplier_id
         WHERE po.branch_id = ?1 ORDER BY po.created_at DESC LIMIT 100",
    )?;
    let rows = stmt.query_map(params![a.branch_id], |r| {
        Ok(json!({
            "order_id": r.get::<_, String>(0)?,
            "reference": r.get::<_, String>(1)?,
            "total_fcfa": r.get::<_, i64>(2)?,
            "status": r.get::<_, String>(3)?,
            "created_at": r.get::<_, i64>(4)?,
            "supplier_name": r.get::<_, String>(5)?,
            "received_fcfa": r.get::<_, i64>(6)?
        }))
    })?;
    let mut orders = Vec::new();
    for row in rows { orders.push(row?); }
    Ok(json!({ "orders": orders }))
}

/// Reçu détaillé d'une vente : lignes, montants et état de synchronisation.
pub fn get_sale_receipt(conn: &Connection, payload: &Value) -> CoreResult<Value> {
    let a = actor(conn, payload)?;
    let sale_id = field_str(payload, "sale_id")?;
    let row: Option<(String, String, Option<String>, i64, i64, i64, i64, i64, i64, i64, String)> = conn.query_row(
        "SELECT receipt_number, customer_name, customer_id, subtotal_fcfa, discount_fcfa, total_fcfa,
                paid_confirmed_fcfa, pending_mobile_money_fcfa, credit_fcfa, occurred_at, sync_status
         FROM sales WHERE id = ?1 AND branch_id = ?2",
        params![sale_id, a.branch_id],
        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?,
            r.get(6)?, r.get(7)?, r.get(8)?, r.get(9)?, r.get(10)?)),
    ).optional()?;
    let (receipt, customer, customer_id, subtotal, discount, total, paid, pending, credit, occurred_at, sync_status) =
        row.ok_or_else(|| CoreError::new("SALE_NOT_FOUND", "Vente introuvable sur cet établissement."))?;
    let mut stmt = conn.prepare(
        "SELECT variant_name, unit_code, unit_count, physical_units, unit_price_fcfa, line_total_fcfa
         FROM sale_lines WHERE sale_id = ?1 ORDER BY id",
    )?;
    let lines: Vec<Value> = stmt.query_map(params![sale_id], |r| {
        Ok(json!({
            "variant_name": r.get::<_, String>(0)?,
            "unit": r.get::<_, String>(1)?,
            "unit_count": r.get::<_, i64>(2)?,
            "physical_units": r.get::<_, i64>(3)?,
            "unit_price_fcfa": r.get::<_, i64>(4)?,
            "line_total_fcfa": r.get::<_, i64>(5)?
        }))
    })?.collect::<Result<_, _>>()?;
    Ok(json!({
        "sale_id": sale_id,
        "receipt_number": receipt,
        "customer_name": customer,
        "customer_id": customer_id,
        "occurred_at": occurred_at,
        "subtotal_fcfa": subtotal,
        "discount_fcfa": discount,
        "total_fcfa": total,
        "paid_confirmed_fcfa": paid,
        "pending_mobile_money_fcfa": pending,
        "credit_fcfa": credit,
        "sync_status": sync_status,
        "lines": lines,
        "is_receipt_not_invoice": true
    }))
}

/// Session de caisse ouverte du point de vente, ou `null`.
pub fn get_cash_session(conn: &Connection, payload: &Value) -> CoreResult<Value> {
    let a = actor(conn, payload)?;
    let row: Option<(String, i64, i64, i64)> = conn.query_row(
        "SELECT id, opening_float_fcfa, expected_closing_fcfa, opened_at FROM cash_sessions
         WHERE branch_id = ?1 AND status = 'OPEN' ORDER BY opened_at DESC LIMIT 1",
        params![a.branch_id],
        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
    ).optional()?;
    match row {
        Some((id, float, expected, opened_at)) => Ok(json!({
            "session_id": id, "opening_float_fcfa": float,
            "expected_closing_fcfa": expected, "opened_at": opened_at
        })),
        None => Ok(json!({ "session_id": null, "opening_float_fcfa": 0, "expected_closing_fcfa": 0, "opened_at": 0 })),
    }
}

pub fn open_cash_session(conn: &mut Connection, payload: &Value) -> CoreResult<Value> {
    let a = actor(conn, payload)?;
    checkout_permission(&a.role)?;
    let float = field_i64(payload, "opening_float_fcfa")?;
    if float < 0 { return Err(CoreError::new("INVALID_FLOAT", "Le fond de caisse ne peut pas être négatif.")); }
    let existing = get_cash_session(conn, payload)?;
    if existing.get("session_id").and_then(Value::as_str).is_some() {
        return Ok(existing);
    }
    let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
    let session_id = new_id("cash");
    tx.execute(
        "INSERT INTO cash_sessions (id, branch_id, opened_by, opening_float_fcfa, expected_closing_fcfa, status, opened_at) VALUES (?1,?2,?3,?4,?4,'OPEN',?5)",
        params![session_id, a.branch_id, a.user_id, float, now_ms()],
    )?;
    tx.execute(
        "INSERT INTO cash_movements (id, session_id, branch_id, movement_type, signed_amount_fcfa, label, occurred_at) VALUES (?1,?2,?3,'OPEN',?4,'Ouverture de caisse',?5)",
        params![new_id("cmv"), session_id, a.branch_id, float, now_ms()],
    )?;
    db::audit(&tx, &a.branch_id, &a.user_id, "CASH_OPEN", &format!("Fond de caisse {float} FCFA"))?;
    tx.commit()?;
    Ok(json!({ "session_id": session_id, "opening_float_fcfa": float, "expected_closing_fcfa": float }))
}

/// Écritures de caisse immuables : encaissement, décaissement ou dépense.
pub fn record_cash_movement(conn: &mut Connection, payload: &Value) -> CoreResult<Value> {
    let a = actor(conn, payload)?;
    checkout_permission(&a.role)?;
    let session = get_cash_session(conn, payload)?;
    let session_id = session.get("session_id").and_then(Value::as_str)
        .ok_or_else(|| CoreError::new("CASH_CLOSED", "Ouvrez d'abord la caisse."))?.to_string();
    let kind = field_str(payload, "movement_type")?;
    if !matches!(kind, "IN" | "OUT" | "EXPENSE") {
        return Err(CoreError::new("INVALID_MOVEMENT", "Type de mouvement inconnu."));
    }
    let amount = field_i64(payload, "amount_fcfa")?;
    if amount <= 0 { return Err(CoreError::new("INVALID_AMOUNT", "Montant positif requis.")); }
    let label = field_str(payload, "label").unwrap_or("Mouvement de caisse");
    let expected_before = field_i64(&session, "expected_closing_fcfa")?;
    let signed = if kind == "IN" { amount } else { -amount };
    let expected = if kind == "IN" {
        checked_add(expected_before, amount, "caisse")?
    } else {
        checked_sub(expected_before, amount, "caisse")?
    };
    let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
    tx.execute(
        "INSERT INTO cash_movements (id, session_id, branch_id, movement_type, signed_amount_fcfa, label, occurred_at) VALUES (?1,?2,?3,?4,?5,?6,?7)",
        params![new_id("cmv"), session_id, a.branch_id, kind, signed, label, now_ms()],
    )?;
    if kind == "EXPENSE" {
        let category = field_str(payload, "category").unwrap_or("AUTRE");
        tx.execute(
            "INSERT INTO expenses (id, branch_id, category, amount_fcfa, description, occurred_at) VALUES (?1,?2,?3,?4,?5,?6)",
            params![new_id("exp"), a.branch_id, category, amount, label, now_ms()],
        )?;
    }
    tx.execute("UPDATE cash_sessions SET expected_closing_fcfa = ?1 WHERE id = ?2", params![expected, session_id])?;
    db::audit(&tx, &a.branch_id, &a.user_id, "CASH_MOVEMENT", &format!("{kind} {amount} FCFA · {label}"))?;
    tx.commit()?;
    Ok(json!({ "status": "RECORDED", "expected_closing_fcfa": expected }))
}

pub fn close_cash_session(conn: &mut Connection, payload: &Value) -> CoreResult<Value> {
    let a = actor(conn, payload)?;
    checkout_permission(&a.role)?;
    let session = get_cash_session(conn, payload)?;
    let session_id = session.get("session_id").and_then(Value::as_str)
        .ok_or_else(|| CoreError::new("CASH_CLOSED", "Aucune caisse ouverte."))?.to_string();
    let counted = field_i64(payload, "counted_fcfa")?;
    if counted < 0 { return Err(CoreError::new("INVALID_COUNT", "Le montant compté ne peut pas être négatif.")); }
    let expected = field_i64(&session, "expected_closing_fcfa")?;
    let variance = checked_sub(counted, expected, "écart de caisse")?;
    let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
    tx.execute(
        "UPDATE cash_sessions SET status = 'CLOSED', counted_fcfa = ?1 WHERE id = ?2",
        params![counted, session_id],
    )?;
    db::enqueue(
        &tx, &a.tenant_id, &a.branch_id, &a.device_id, &a.user_id, "CASH_CLOSE",
        &format!("Clôture {session_id} · écart {variance} FCFA"),
        &json!({ "session_id": session_id, "counted_fcfa": counted, "variance_fcfa": variance }).to_string(),
        1, &new_id("idem"),
    )?;
    db::audit(&tx, &a.branch_id, &a.user_id, "CASH_CLOSE", &format!("Compté {counted}, attendu {expected}, écart {variance}"))?;
    tx.commit()?;
    Ok(json!({ "status": "CLOSED", "variance_fcfa": variance, "counted_fcfa": counted }))
}

/// Indicateurs calculés uniquement depuis les données locales durables.
pub fn get_reports(conn: &Connection, payload: &Value) -> CoreResult<Value> {
    let a = actor(conn, payload)?;
    let to_ms = field_i64_or(payload, "to_ms", now_ms());
    let from_ms = field_i64_or(payload, "from_ms", to_ms - 24 * 60 * 60 * 1000);
    if from_ms < 0 || to_ms < from_ms {
        return Err(CoreError::new("INVALID_DATE_RANGE", "Période de rapport invalide."));
    }
    let day_start = from_ms;
    let revenue: i64 = conn.query_row(
        "SELECT COALESCE(SUM(total_fcfa),0) FROM sales WHERE branch_id = ?1 AND occurred_at >= ?2",
        params![a.branch_id, day_start], |r| r.get(0),
    )?;
    let cash_collected: i64 = conn.query_row(
        "SELECT COALESCE(SUM(signed_amount_fcfa),0) FROM cash_movements WHERE branch_id = ?1 AND occurred_at >= ?2 AND occurred_at <= ?3 AND signed_amount_fcfa > 0",
        params![a.branch_id, day_start, to_ms], |r| r.get(0),
    )?;
    let credits: i64 = conn.query_row(
        "SELECT COALESCE(SUM(credit_fcfa),0) FROM sales WHERE branch_id = ?1 AND occurred_at >= ?2 AND occurred_at <= ?3",
        params![a.branch_id, day_start, to_ms], |r| r.get(0),
    )?;
    let expenses: i64 = conn.query_row(
        "SELECT COALESCE(SUM(amount_fcfa),0) FROM expenses WHERE branch_id = ?1 AND occurred_at >= ?2 AND occurred_at <= ?3",
        params![a.branch_id, day_start, to_ms], |r| r.get(0),
    )?;
    let receivables: i64 = conn.query_row(
        "SELECT COALESCE(SUM(financial_balance_fcfa),0) FROM customers", [], |r| r.get(0),
    )?;
    let supplier_debts: i64 = conn.query_row(
        "SELECT COALESCE(SUM(financial_debt_fcfa),0) FROM suppliers", [], |r| r.get(0),
    )?;
    let stock_value: i64 = conn.query_row(
        "SELECT COALESCE(SUM(s.full_units_qty * v.retail_bottle_fcfa),0) FROM stock_balances s JOIN product_variants v ON v.id = s.variant_id WHERE s.branch_id = ?1",
        params![a.branch_id], |r| r.get(0),
    )?;
    let by_product: Vec<Value> = {
        let mut stmt = conn.prepare(
            "SELECT variant_name, SUM(line_total_fcfa), SUM(physical_units) FROM sale_lines sl
             JOIN sales s ON s.id = sl.sale_id WHERE s.branch_id = ?1 AND s.occurred_at >= ?2 AND s.occurred_at <= ?3
               AND sl.is_packaging = 0
             GROUP BY variant_name ORDER BY SUM(line_total_fcfa) DESC LIMIT 20",
        )?;
        stmt.query_map(params![a.branch_id, day_start, to_ms], |r| Ok(json!({
            "variant_name": r.get::<_, String>(0)?,
            "revenue_fcfa": r.get::<_, i64>(1)?,
            "bottles_sold": r.get::<_, i64>(2)?
        })))?.collect::<Result<_, _>>()?
    };
    let crate_revenue: i64 = conn.query_row(
        "SELECT COALESCE(SUM(packaging_sales_fcfa),0) FROM sales
         WHERE branch_id = ?1 AND occurred_at >= ?2 AND occurred_at <= ?3",
        params![a.branch_id, day_start, to_ms], |r| r.get(0),
    )?;
    Ok(json!({
        "from_ms": from_ms,
        "to_ms": to_ms,
        "revenue_fcfa": revenue,
        "crate_sales_fcfa": crate_revenue,
        "crate_sales_are_commercial": true,
        "cash_collected_fcfa": cash_collected,
        "credits_fcfa": credits,
        "expenses_fcfa": expenses,
        "estimated_margin_fcfa": checked_sub(cash_collected, expenses, "marge estimée")?,
        "receivables_fcfa": receivables,
        "supplier_debts_fcfa": supplier_debts,
        "stock_value_fcfa": stock_value,
        "by_product": by_product,
        "confirms_revenue_not_cash": true
    }))
}

/// Conflits ouverts, jamais résolus silencieusement.
pub fn get_conflicts(conn: &Connection, payload: &Value) -> CoreResult<Value> {
    let _ = actor(conn, payload)?;
    let mut stmt = conn.prepare(
        "SELECT id, operation_id, conflict_type, server_reason, status, created_at
         FROM sync_conflicts ORDER BY created_at DESC LIMIT 100",
    )?;
    let rows = stmt.query_map(params![], |r| Ok(json!({
        "conflict_id": r.get::<_, String>(0)?,
        "operation_id": r.get::<_, String>(1)?,
        "conflict_type": r.get::<_, String>(2)?,
        "server_reason": r.get::<_, String>(3)?,
        "status": r.get::<_, String>(4)?,
        "created_at": r.get::<_, i64>(5)?
    })))?;
    let mut conflicts = Vec::new();
    for row in rows { conflicts.push(row?); }
    Ok(json!({ "conflicts": conflicts }))
}

/// Historique local des synchronisations et décisions : visible même hors ligne.
pub fn get_sync_history(conn: &Connection, payload: &Value) -> CoreResult<Value> {
    let a = actor(conn, payload)?;
    let mut stmt = conn.prepare(
        "SELECT operation_id, command_type, summary, status, attempt_count, created_at, synced_at, server_sequence, last_error
         FROM sync_outbox ORDER BY COALESCE(synced_at, created_at) DESC LIMIT 100",
    )?;
    let rows = stmt.query_map([], |r| Ok(json!({
        "operation_id": r.get::<_, String>(0)?,
        "command_type": r.get::<_, String>(1)?,
        "summary": r.get::<_, String>(2)?,
        "status": r.get::<_, String>(3)?,
        "attempt_count": r.get::<_, i64>(4)?,
        "created_at": r.get::<_, i64>(5)?,
        "synced_at": r.get::<_, Option<i64>>(6)?,
        "server_sequence": r.get::<_, Option<i64>>(7)?,
        "last_error": r.get::<_, Option<String>>(8)?
    })))?;
    let mut operations = Vec::new();
    for row in rows { operations.push(row?); }
    let mut stmt = conn.prepare(
        "SELECT action, details, occurred_at FROM audit_logs
         WHERE branch_id=?1 AND (action LIKE '%SYNC%' OR action LIKE '%CONFLICT%' OR action LIKE '%QUOTA%')
         ORDER BY occurred_at DESC LIMIT 100",
    )?;
    let rows = stmt.query_map(params![a.branch_id], |r| Ok(json!({
        "action": r.get::<_, String>(0)?,
        "details": r.get::<_, String>(1)?,
        "occurred_at": r.get::<_, i64>(2)?
    })))?;
    let mut audit = Vec::new();
    for row in rows { audit.push(row?); }
    Ok(json!({ "operations": operations, "audit": audit }))
}

/// Un lot resté `SYNCING` après une coupure réseau redevient réessayable.
pub fn reset_stale_syncing(conn: &mut Connection, payload: &Value) -> CoreResult<Value> {
    let _ = actor(conn, payload)?;
    let stale_before = payload.get("stale_before_ms").and_then(Value::as_i64)
        .unwrap_or_else(|| now_ms() - 10 * 60 * 1000);
    let changed = conn.execute(
        "UPDATE sync_outbox SET status = 'FAILED', last_error = 'Synchronisation interrompue — réessai', next_retry_at = ?1
         WHERE status = 'SYNCING' AND created_at < ?2",
        params![now_ms(), stale_before],
    )?;
    Ok(json!({ "reset": changed }))
}

// ---- PARTIE 1 : catalogue, prix et relevés partenaires ----

fn assert_catalog_role(role: &str) -> CoreResult<()> {
    if matches!(role, "OWNER" | "ADMIN" | "MANAGER") {
        Ok(())
    } else {
        Err(CoreError::new("ROLE_FORBIDDEN", "Rôle non autorisé à modifier le catalogue."))
    }
}

fn assert_non_negative_prices(retail_case: i64, wholesale: i64, bottle: i64) -> CoreResult<()> {
    if retail_case < 0 || wholesale < 0 || bottle < 0 {
        Err(CoreError::new("INVALID_PRICE", "Les prix FCFA doivent être positifs ou nuls."))
    } else {
        Ok(())
    }
}

/// Création ou modification d'une variante. Les règles de prix restent dans le moteur.
pub fn save_product_variant(conn: &mut Connection, payload: &Value) -> CoreResult<Value> {
    let a = actor(conn, payload)?;
    assert_catalog_role(&a.role)?;
    let pack_size = field_i64(payload, "pack_size")?;
    if pack_size <= 0 {
        return Err(CoreError::new("INVALID_PACK_SIZE", "La contenance doit être positive."));
    }
    let retail = field_i64(payload, "retail_case_fcfa")?;
    let wholesale = field_i64(payload, "wholesale_case_fcfa")?;
    let bottle = field_i64(payload, "retail_bottle_fcfa")?;
    assert_non_negative_prices(retail, wholesale, bottle)?;
    let name = field_str(payload, "name")?.trim().to_string();
    let sku = field_str(payload, "sku")?.trim().to_string();
    if name.is_empty() || sku.is_empty() {
        return Err(CoreError::new("INVALID_PRODUCT", "Nom et référence requis."));
    }
    let profile = field_str_opt(payload, "bottle_profile_id").map(str::to_string);
    let crate_id = field_str_opt(payload, "crate_type_id").map(str::to_string);
    let is_returnable = field_bool(payload, "is_returnable", false);
    if is_returnable && profile.is_none() && crate_id.is_none() {
        return Err(CoreError::new(
            "RETURNABLE_WITHOUT_PHYSICAL_PROFILE",
            "Un produit retournable doit avoir un profil de bouteille ou un type de casier.",
        ));
    }
    let category = field_str_opt(payload, "category").map(str::to_string);
    let description = field_str_opt(payload, "description").map(str::to_string);
    let photo_path = field_str_opt(payload, "photo_path").map(str::to_string);
    let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
    let variant_id = match field_str_opt(payload, "variant_id").map(str::to_string) {
        Some(id) => {
            let exists: i64 = tx.query_row(
                "SELECT COUNT(*) FROM product_variants WHERE id = ?1 AND tenant_id = ?2",
                params![id, a.tenant_id], |r| r.get(0),
            )?;
            if exists == 0 {
                return Err(CoreError::new("VARIANT_NOT_FOUND", "Variante introuvable."));
            }
            tx.execute(
                "UPDATE product_variants SET name=?2, brand=?3, sku=?4, volume_label=?5, container_type=?6, material=?7, bottle_profile_id=?8, crate_type_id=?9, pack_size=?10, retail_case_fcfa=?11, wholesale_case_fcfa=?12, retail_bottle_fcfa=?13, category=?14, description=?15, photo_path=?16, is_returnable=?17, price_revision=price_revision+1 WHERE id=?1",
                params![id, name, field_str(payload, "brand")?, sku, field_str(payload, "volume_label")?,
                    field_str(payload, "container_type")?, field_str(payload, "material")?, profile, crate_id,
                    pack_size, retail, wholesale, bottle, category, description, photo_path,
                    if is_returnable { 1 } else { 0 }],
            )?;
            id
        }
        None => {
            let id = new_id("var");
            tx.execute(
                "INSERT INTO product_variants (id, tenant_id, name, brand, sku, barcode, volume_label, container_type, material, bottle_profile_id, crate_type_id, pack_size, retail_case_fcfa, wholesale_case_fcfa, retail_bottle_fcfa, price_revision, is_active, category, description, photo_path, is_returnable)
                 VALUES (?1,?2,?3,?4,?5,NULL,?6,?7,?8,?9,?10,?11,?12,?13,?14,1,1,?15,?16,?17,?18)",
                params![id, a.tenant_id, name, field_str(payload, "brand")?, sku,
                    field_str(payload, "volume_label")?, field_str(payload, "container_type")?,
                    field_str(payload, "material")?, profile, crate_id, pack_size, retail, wholesale, bottle,
                    category, description, photo_path, if is_returnable { 1 } else { 0 }],
            )?;
            tx.execute(
                "INSERT OR IGNORE INTO stock_balances (branch_id, variant_id, full_units_qty, min_alert_units) VALUES (?1,?2,0,12)",
                params![a.branch_id, id],
            )?;
            id
        }
    };
    tx.execute(
        "INSERT INTO price_history (id, variant_id, retail_case_fcfa, wholesale_case_fcfa, retail_bottle_fcfa, changed_at, changed_by) VALUES (?1,?2,?3,?4,?5,?6,?7)",
        params![new_id("price"), variant_id, retail, wholesale, bottle, now_ms(), a.user_id],
    )?;
    db::enqueue(
        &tx, &a.tenant_id, &a.branch_id, &a.device_id, &a.user_id, "CATALOG_VARIANT_UPSERT",
        &format!("Variante {variant_id} · {name}"), &payload.to_string(), 1, &new_id("idem"),
    )?;
    db::audit(&tx, &a.branch_id, &a.user_id, "CATALOG_VARIANT_UPSERT", &format!("{variant_id} · {name}"))?;
    tx.commit()?;
    Ok(json!({ "variant_id": variant_id, "synced": false }))
}

/// Mise à jour des prix avec historique immuable.
pub fn update_variant_prices(conn: &mut Connection, payload: &Value) -> CoreResult<Value> {
    let a = actor(conn, payload)?;
    assert_catalog_role(&a.role)?;
    let variant_id = field_str(payload, "variant_id")?;
    let retail = field_i64(payload, "retail_case_fcfa")?;
    let wholesale = field_i64(payload, "wholesale_case_fcfa")?;
    let bottle = field_i64(payload, "retail_bottle_fcfa")?;
    assert_non_negative_prices(retail, wholesale, bottle)?;
    let old: (i64, i64, i64, i64) = conn.query_row(
        "SELECT retail_case_fcfa, wholesale_case_fcfa, retail_bottle_fcfa, price_revision FROM product_variants WHERE id = ?1",
        params![variant_id], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
    )?;
    let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
    tx.execute(
        "INSERT INTO price_history (id, variant_id, retail_case_fcfa, wholesale_case_fcfa, retail_bottle_fcfa, changed_at, changed_by) VALUES (?1,?2,?3,?4,?5,?6,?7)",
        params![new_id("price"), variant_id, old.0, old.1, old.2, now_ms(), a.user_id],
    )?;
    tx.execute(
        "UPDATE product_variants SET retail_case_fcfa=?2, wholesale_case_fcfa=?3, retail_bottle_fcfa=?4, price_revision=price_revision+1 WHERE id=?1",
        params![variant_id, retail, wholesale, bottle],
    )?;
    db::enqueue(
        &tx, &a.tenant_id, &a.branch_id, &a.device_id, &a.user_id, "PRICE_UPDATE",
        &format!("{variant_id} : {retail}/{wholesale}/{bottle} FCFA"), &payload.to_string(), 1, &new_id("idem"),
    )?;
    db::audit(&tx, &a.branch_id, &a.user_id, "PRICE_UPDATE", variant_id)?;
    tx.commit()?;
    Ok(json!({ "variant_id": variant_id, "price_revision": old.3 + 1 }))
}

/// Archivage logique : l'historique commercial reste intact.
pub fn archive_product_variant(conn: &mut Connection, payload: &Value) -> CoreResult<Value> {
    let a = actor(conn, payload)?;
    assert_catalog_role(&a.role)?;
    let variant_id = field_str(payload, "variant_id")?;
    let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
    tx.execute("UPDATE product_variants SET is_active = 0 WHERE id = ?1", params![variant_id])?;
    db::enqueue(
        &tx, &a.tenant_id, &a.branch_id, &a.device_id, &a.user_id, "CATALOG_VARIANT_ARCHIVE",
        variant_id, &payload.to_string(), 1, &new_id("idem"),
    )?;
    db::audit(&tx, &a.branch_id, &a.user_id, "CATALOG_VARIANT_ARCHIVE", variant_id)?;
    tx.commit()?;
    Ok(json!({ "variant_id": variant_id, "status": "ARCHIVED" }))
}

/// Nouveau format de casier physiquement configuré (jamais déduit du volume).
pub fn create_crate_type(conn: &mut Connection, payload: &Value) -> CoreResult<Value> {
    let a = actor(conn, payload)?;
    assert_catalog_role(&a.role)?;
    let code = field_str(payload, "code")?.trim().to_string();
    let capacity = field_i64(payload, "capacity")?;
    if code.is_empty() || capacity <= 0 {
        return Err(CoreError::new("INVALID_CRATE", "Code et capacité requis."));
    }
    let group = field_str(payload, "compatibility_group_id")?.to_string();
    let replacement = field_i64(payload, "replacement_value_fcfa")?;
    if replacement < 0 {
        return Err(CoreError::new("INVALID_REPLACEMENT_VALUE", "Valeur de remplacement invalide."));
    }
    let allow_half = if field_bool(payload, "allow_half_case", true) { 1 } else { 0 };
    let compartments = field_i64_or(payload, "compartments", capacity);
    if compartments <= 0 {
        return Err(CoreError::new("INVALID_CRATE", "Le nombre de compartiments doit être positif."));
    }
    let deposit = field_i64_or(payload, "deposit_value_fcfa", 0);
    if deposit < 0 {
        return Err(CoreError::new("INVALID_DEPOSIT_VALUE", "Valeur de consigne invalide."));
    }
    let is_reusable = if field_bool(payload, "is_reusable", true) { 1 } else { 0 };
    let physical_profile = field_str_opt(payload, "physical_profile_id").map(str::to_string);
    let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
    let exists: i64 = tx.query_row("SELECT COUNT(*) FROM crate_types WHERE code = ?1", params![code], |r| r.get(0))?;
    if exists > 0 {
        return Err(CoreError::new("CRATE_CODE_EXISTS", "Ce format de casier existe déjà."));
    }
    let id = new_id("crate");
    // Trois valeurs distinctes ; aucune n'est déduite du prix d'une boisson.
    let reference_value = field_i64_or(payload, "reference_value_fcfa", replacement);
    let acquisition_value = field_i64_or(payload, "acquisition_value_fcfa", reference_value);
    let empty_sale_price = field_i64_or(payload, "empty_sale_price_fcfa", 0);
    if reference_value < 0 || acquisition_value < 0 || empty_sale_price < 0 {
        return Err(CoreError::new("INVALID_CRATE_VALUE", "Les valeurs de casier sont des entiers FCFA positifs ou nuls."));
    }
    tx.execute(
        "INSERT INTO crate_types (id, code, capacity, compatibility_group_id, allow_half_case, replacement_value_fcfa, is_reusable, name, compartments, dimensions, deposit_value_fcfa, physical_profile_id, acquisition_value_fcfa, reference_value_fcfa, empty_sale_price_fcfa)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15)",
        params![id, code, capacity, group, allow_half, replacement, is_reusable,
            field_str(payload, "name").unwrap_or(&code), compartments,
            field_str_opt(payload, "dimensions"), deposit, physical_profile,
            acquisition_value, reference_value, empty_sale_price],
    )?;
    tx.execute(
        "INSERT INTO crate_value_history
           (id, crate_id, acquisition_fcfa, reference_fcfa, empty_sale_fcfa, effective_from,
            effective_to, changed_by, reason, created_at)
         VALUES (?1,?2,?3,?4,?5,?6,NULL,?7,'Création du format de casier',?6)",
        params![new_id("cvh"), id, acquisition_value, reference_value, empty_sale_price, now_ms(), a.user_id],
    )?;
    tx.execute(
        "INSERT OR IGNORE INTO crate_stock (branch_id, crate_id, full_qty, empty_qty) VALUES (?1,?2,0,0)",
        params![a.branch_id, id],
    )?;
    db::enqueue(
        &tx, &a.tenant_id, &a.branch_id, &a.device_id, &a.user_id, "CRATE_TYPE_UPSERT",
        &format!("{code} ({capacity})"), &payload.to_string(), 1, &new_id("idem"),
    )?;
    db::audit(&tx, &a.branch_id, &a.user_id, "CRATE_TYPE_UPSERT", &code)?;
    tx.commit()?;
    Ok(json!({ "crate_type_id": id, "code": code }))
}

/// Modification d'un format de casier par un administrateur : jamais de changement
/// de compatibilité physique implicite, la validation est explicite.
pub fn update_crate_type(conn: &mut Connection, payload: &Value) -> CoreResult<Value> {
    let a = actor(conn, payload)?;
    assert_catalog_role(&a.role)?;
    let crate_id = field_str(payload, "crate_type_id")?;
    let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
    let current: (i64, String) = tx.query_row(
        "SELECT capacity, compatibility_group_id FROM crate_types WHERE id = ?1",
        params![crate_id], |r| Ok((r.get(0)?, r.get(1)?)),
    ).optional()?.ok_or_else(|| CoreError::new("CRATE_NOT_FOUND", "Format de casier introuvable."))?;
    let capacity = field_i64_or(payload, "capacity", current.0);
    let group = field_str_opt(payload, "compatibility_group_id").unwrap_or(&current.1);
    if capacity <= 0 {
        return Err(CoreError::new("INVALID_CRATE", "Capacité invalide."));
    }
    let replacement = field_i64_or(payload, "replacement_value_fcfa", -1);
    if replacement < -1 {
        return Err(CoreError::new("INVALID_REPLACEMENT_VALUE", "Valeur de remplacement invalide."));
    }
    let deposit = field_i64_or(payload, "deposit_value_fcfa", -1);
    if deposit < -1 {
        return Err(CoreError::new("INVALID_DEPOSIT_VALUE", "Valeur de consigne invalide."));
    }
    tx.execute(
        "UPDATE crate_types SET capacity=?2, compatibility_group_id=?3,
            name=COALESCE(?4, name), compartments=COALESCE(?5, compartments),
            dimensions=COALESCE(?6, dimensions),
            deposit_value_fcfa=CASE WHEN ?7 < 0 THEN deposit_value_fcfa ELSE ?7 END,
            physical_profile_id=COALESCE(?8, physical_profile_id),
            replacement_value_fcfa=CASE WHEN ?9 < 0 THEN replacement_value_fcfa ELSE ?9 END,
            allow_half_case=COALESCE(?10, allow_half_case),
            is_reusable=COALESCE(?11, is_reusable)
         WHERE id=?1",
        params![
            crate_id, capacity, group, field_str_opt(payload, "name"),
            payload.get("compartments").and_then(Value::as_i64),
            field_str_opt(payload, "dimensions"),
            payload.get("deposit_value_fcfa").and_then(Value::as_i64),
            field_str_opt(payload, "physical_profile_id"),
            payload.get("replacement_value_fcfa").and_then(Value::as_i64),
            payload.get("allow_half_case").and_then(Value::as_bool).map(|v| if v { 1 } else { 0 }),
            payload.get("is_reusable").and_then(Value::as_bool).map(|v| if v { 1 } else { 0 }),
        ],
    )?;
    // Valeurs financières : toute modification crée une période d'historique et
    // ne réécrit jamais la valeur figée des mouvements passés.
    let current_values: (i64, i64, i64) = tx.query_row(
        "SELECT acquisition_value_fcfa, reference_value_fcfa, empty_sale_price_fcfa
         FROM crate_types WHERE id = ?1",
        params![crate_id], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
    )?;
    let acquisition = field_i64_or(payload, "acquisition_value_fcfa", current_values.0);
    let reference = field_i64_or(payload, "reference_value_fcfa", current_values.1);
    let empty_sale = field_i64_or(payload, "empty_sale_price_fcfa", current_values.2);
    if acquisition < 0 || reference < 0 || empty_sale < 0 {
        return Err(CoreError::new("INVALID_CRATE_VALUE", "Les valeurs de casier sont des entiers FCFA positifs ou nuls."));
    }
    if (acquisition, reference, empty_sale) != current_values {
        let reason = field_str_opt(payload, "reason").unwrap_or("Modification des valeurs du casier");
        tx.execute(
            "UPDATE crate_value_history SET effective_to = ?1 WHERE crate_id = ?2 AND effective_to IS NULL",
            params![now_ms(), crate_id],
        )?;
        tx.execute(
            "INSERT INTO crate_value_history
               (id, crate_id, acquisition_fcfa, reference_fcfa, empty_sale_fcfa, effective_from,
                effective_to, changed_by, reason, created_at)
             VALUES (?1,?2,?3,?4,?5,?6,NULL,?7,?8,?6)",
            params![new_id("cvh"), crate_id, acquisition, reference, empty_sale, now_ms(), a.user_id, reason],
        )?;
        tx.execute(
            "UPDATE crate_types SET acquisition_value_fcfa=?2, reference_value_fcfa=?3,
                    empty_sale_price_fcfa=?4 WHERE id=?1",
            params![crate_id, acquisition, reference, empty_sale],
        )?;
    }
    db::enqueue(
        &tx, &a.tenant_id, &a.branch_id, &a.device_id, &a.user_id, "CRATE_TYPE_UPSERT",
        &format!("Format {crate_id}"), &payload.to_string(), 1, &new_id("idem"),
    )?;
    db::audit(&tx, &a.branch_id, &a.user_id, "CRATE_TYPE_UPSERT", crate_id)?;
    tx.commit()?;
    Ok(json!({
        "crate_type_id": crate_id,
        "capacity": capacity,
        "acquisition_value_fcfa": acquisition,
        "reference_value_fcfa": reference,
        "empty_sale_price_fcfa": empty_sale
    }))
}

/// Stocks du dépôt : les casiers proviennent du journal physique (pleins,
/// vides, chez les clients, chez les fournisseurs). Les bouteilles ne sont plus
/// suivies : elles restent consultables comme information historique.
pub fn get_packaging_stock(conn: &Connection, payload: &Value) -> CoreResult<Value> {
    let a = actor(conn, payload)?;
    let stock = crate::crates::get_crate_stock(conn, payload)?;
    let mut stmt = conn.prepare(
        "SELECT bp.id, bp.name, bp.volume_label, bp.compatibility_group_id, COALESCE(ebs.qty,0)
         FROM bottle_profiles bp LEFT JOIN empty_bottle_stock ebs
         ON ebs.profile_id = bp.id AND ebs.branch_id = ?1
         WHERE bp.tenant_id = ?2 ORDER BY bp.name",
    )?;
    let bottles: Vec<Value> = stmt.query_map(params![a.branch_id, a.tenant_id], |r| Ok(json!({
        "profile_id": r.get::<_, String>(0)?,
        "name": r.get::<_, String>(1)?,
        "volume_label": r.get::<_, String>(2)?,
        "compatibility_group_id": r.get::<_, String>(3)?,
        "qty": r.get::<_, i64>(4)?,
        "tracked": false
    })))?.collect::<Result<_, _>>()?;
    Ok(json!({
        "crates": stock["crates"].clone(),
        "totals": stock["totals"].clone(),
        "valuation": stock["valuation"].clone(),
        "engine": stock["engine"].clone(),
        "legacy_engine": stock["legacy_engine"].clone(),
        "empty_bottles": bottles,
        "bottles_tracking": {
            "state": "INFORMATIONAL_ONLY",
            "creates_stock": false,
            "creates_debt": false,
            "note": "Une bouteille, un PET ou une canette n'est pas un emballage suivi : aucun stock, aucune dette, aucun retour."
        }
    }))
}

/// Bordereau local : opérations financières et emballages d'un partenaire.
pub fn get_partner_statement(conn: &Connection, payload: &Value) -> CoreResult<Value> {
    let a = actor(conn, payload)?;
    let partner_type = field_str(payload, "partner_type")?;
    let partner_id = field_str(payload, "partner_id")?;
    if !matches!(partner_type, "CUSTOMER" | "SUPPLIER") {
        return Err(CoreError::new("INVALID_PARTNER", "Type de partenaire inconnu."));
    }
    let (purchases, paid, balance) = if partner_type == "CUSTOMER" {
        let purchases: i64 = conn.query_row(
            "SELECT COALESCE(SUM(total_fcfa),0) FROM sales WHERE branch_id = ?1 AND customer_id = ?2",
            params![a.branch_id, partner_id], |r| r.get(0),
        )?;
        let paid: i64 = conn.query_row(
            "SELECT COALESCE(SUM(amount_fcfa),0) FROM payments WHERE branch_id = ?1 AND partner_type='CUSTOMER' AND partner_id=?2",
            params![a.branch_id, partner_id], |r| r.get(0),
        )?;
        let balance: i64 = conn.query_row(
            "SELECT COALESCE(financial_balance_fcfa,0) FROM customers WHERE id = ?1",
            params![partner_id], |r| r.get(0),
        )?;
        (purchases, paid, balance)
    } else {
        let purchases: i64 = conn.query_row(
            "SELECT COALESCE(SUM(pr.total_fcfa),0) FROM purchase_receipts pr JOIN purchase_orders po ON po.id=pr.order_id WHERE po.branch_id=?1 AND po.supplier_id=?2",
            params![a.branch_id, partner_id], |r| r.get(0),
        )?;
        let paid: i64 = conn.query_row(
            "SELECT COALESCE(SUM(amount_fcfa),0) FROM payments WHERE branch_id = ?1 AND partner_type='SUPPLIER' AND partner_id=?2",
            params![a.branch_id, partner_id], |r| r.get(0),
        )?;
        let balance: i64 = conn.query_row(
            "SELECT COALESCE(financial_debt_fcfa,0) FROM suppliers WHERE id = ?1",
            params![partner_id], |r| r.get(0),
        )?;
        (purchases, paid, balance)
    };
    let mut stmt = conn.prepare(
        "SELECT method, amount_fcfa, occurred_at, reference FROM payments
         WHERE branch_id=?1 AND partner_type=?2 AND partner_id=?3 ORDER BY occurred_at DESC LIMIT 100",
    )?;
    let payments: Vec<Value> = stmt.query_map(params![a.branch_id, partner_type, partner_id], |r| Ok(json!({
        "method": r.get::<_, String>(0)?,
        "amount_fcfa": r.get::<_, i64>(1)?,
        "occurred_at": r.get::<_, i64>(2)?,
        "reference": r.get::<_, Option<String>>(3)?
    })))?.collect::<Result<_, _>>()?;
    let mut stmt = conn.prepare(
        "SELECT item_kind, ref_id, direction, qty, reason, occurred_at FROM partner_packaging_ledger
         WHERE branch_id=?1 AND partner_type=?2 AND partner_id=?3 ORDER BY occurred_at DESC LIMIT 100",
    )?;
    let packaging: Vec<Value> = stmt.query_map(params![a.branch_id, partner_type, partner_id], |r| Ok(json!({
        "item_kind": r.get::<_, String>(0)?,
        "ref_id": r.get::<_, String>(1)?,
        "direction": r.get::<_, String>(2)?,
        "qty": r.get::<_, i64>(3)?,
        "reason": r.get::<_, String>(4)?,
        "occurred_at": r.get::<_, i64>(5)?
    })))?.collect::<Result<_, _>>()?;
    Ok(json!({
        "partner_type": partner_type,
        "partner_id": partner_id,
        "purchases_fcfa": purchases,
        "paid_fcfa": paid,
        "balance_fcfa": balance,
        "payments": payments,
        "packaging_ledger": packaging,
        "is_bordereau_local": true
    }))
}

fn session_record(conn: &Connection, token: &str) -> CoreResult<(String, String, String, String, i64, i64)> {
    conn.query_row(
        "SELECT s.user_id, s.branch_id, s.device_id, u.role, s.expires_at, s.locked
         FROM sessions s JOIN users u ON u.id=s.user_id WHERE s.token=?1",
        params![token],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?, row.get::<_, i64>(5)?)),
    )
    .optional()?
    .ok_or_else(|| CoreError::new("SESSION_INVALID", "Session introuvable."))
}

fn session_dto(token: &str, user: &str, role: &str, branch: &str, device: &str, expires: i64) -> Value {
    json!({
        "token": token, "user_id": user, "role": role, "branch_id": branch,
        "device_id": device, "expires_at": expires
    })
}

/// Renouvellement par rotation : l'ancien jeton est révoqué, jamais réutilisé.
pub fn renew_session(conn: &mut Connection, payload: &Value) -> CoreResult<Value> {
    let token = field_str(payload, "token")?;
    let (user, branch, device, role, expires, locked) = session_record(conn, token)?;
    if locked == 1 || expires <= now_ms() {
        return Err(CoreError::new("SESSION_INVALID", "Session expirée ou verrouillée."));
    }
    let revoked: i64 = conn.query_row("SELECT revoked FROM sessions WHERE token=?1", params![token], |r| r.get(0))?;
    if revoked == 1 {
        return Err(CoreError::new("SESSION_INVALID", "Session révoquée."));
    }
    let settings = security_settings(conn, db::TENANT_ID, &branch)?;
    let next = new_id("tok");
    let next_expiry = now_ms() + settings.session_duration_ms;
    let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
    tx.execute(
        "UPDATE sessions SET revoked=1, revoked_at=?2, revocation_reason='ROTATED' WHERE token=?1",
        params![token, now_ms()],
    )?;
    tx.execute(
        "INSERT INTO sessions (token, user_id, branch_id, device_id, expires_at, created_at, locked, last_activity_at)
         VALUES (?1,?2,?3,?4,?5,?6,0,?6)",
        params![next, user, branch, device, next_expiry, now_ms()],
    )?;
    db::audit_security(&tx, &branch, &user, Some(device), "SESSION_ROTATED", "Jeton renouvelé", "INFO", "AUTHENTICATION")?;
    tx.commit()?;
    Ok(session_dto(&next, &user, &role, &branch, &device, next_expiry))
}

pub fn revoke_session(conn: &mut Connection, payload: &Value) -> CoreResult<Value> {
    let token = field_str(payload, "token")?;
    let reason = field_str_opt(payload, "reason").unwrap_or("LOST_DEVICE");
    let (user, branch, device, _role, _expires, _locked) = session_record(conn, token)?;
    conn.execute(
        "UPDATE sessions SET revoked=1, revoked_at=?2, revocation_reason=?3 WHERE token=?1",
        params![token, now_ms(), reason],
    )?;
    db::audit_security(conn, &branch, &user, Some(device), "SESSION_REVOKED", reason, "WARN", "AUTHENTICATION")?;
    Ok(json!({ "status": "REVOKED", "reason": reason }))
}

pub fn lock_session(conn: &mut Connection, payload: &Value) -> CoreResult<Value> {
    let token = field_str(payload, "token")?;
    let (user, branch, device, _role, _expires, _locked) = session_record(conn, token)?;
    conn.execute("UPDATE sessions SET locked=1 WHERE token=?1", params![token])?;
    db::audit_security(conn, &branch, &user, Some(device), "SESSION_LOCKED", "Verrouillage automatique ou manuel", "INFO", "AUTHENTICATION")?;
    Ok(json!({ "status": "LOCKED" }))
}

pub fn unlock_session(conn: &mut Connection, payload: &Value) -> CoreResult<Value> {
    let token = field_str(payload, "token")?;
    let pin = field_str(payload, "pin")?;
    let (user, branch, device, _role, _expires, _locked) = session_record(conn, token)?;
    if !db::verify_user_pin(conn, &user, pin)? {
        db::audit_security(conn, &branch, &user, Some(device), "UNLOCK_FAILED", "PIN invalide", "WARN", "AUTHENTICATION")?;
        return Err(CoreError::new("INVALID_PIN", "Code PIN incorrect."));
    }
    let kdf: String = conn.query_row("SELECT pin_kdf FROM users WHERE id=?1", params![user], |r| r.get(0))?;
    if kdf == "SHA256_LEGACY" {
        db::upgrade_user_pin(conn, &user, pin, None)?;
    }
    conn.execute("UPDATE sessions SET locked=0, last_activity_at=?2 WHERE token=?1", params![token, now_ms()])?;
    db::audit_security(conn, &branch, &user, Some(device), "UNLOCKED", "Session déverrouillée", "INFO", "AUTHENTICATION")?;
    Ok(json!({ "status": "UNLOCKED" }))
}

/// Changement de PIN sous verrou : ancien PIN vérifié, nouveau sel généré, audit.
pub fn change_pin(conn: &mut Connection, payload: &Value) -> CoreResult<Value> {
    let a = actor(conn, payload)?;
    let old_pin = field_str(payload, "old_pin")?;
    let new_pin = field_str(payload, "new_pin")?;
    if new_pin.len() < 4 {
        return Err(CoreError::new("WEAK_PIN", "Le code PIN doit contenir au moins 4 chiffres."));
    }
    db::upgrade_user_pin(conn, &a.user_id, new_pin, Some(old_pin))?;
    db::audit_security(conn, &a.branch_id, &a.user_id, Some(&a.device_id), "PIN_CHANGED", "Code PIN mis à jour avec un sel aléatoire", "WARN", "AUTHENTICATION")?;
    Ok(json!({ "status": "CHANGED", "kdf": "PBKDF2_SHA256" }))
}

/// Utilisé par la couche FFI avant une sauvegarde : la politique est appliquée.
pub fn backup_encryption_required(conn: &Connection, payload: &Value) -> CoreResult<bool> {
    let a = actor(conn, payload)?;
    Ok(security_settings(conn, &a.tenant_id, &a.branch_id)?.backup_encryption_required)
}

pub fn check_permission_command(conn: &Connection, payload: &Value) -> CoreResult<Value> {
    let permission = field_str(payload, "permission")?;
    let allowed = check_permission(conn, payload, permission)?;
    Ok(json!({ "permission": permission, "allowed": allowed }))
}

pub fn get_security_policy(conn: &Connection, payload: &Value) -> CoreResult<Value> {
    let a = actor(conn, payload)?;
    let settings = security_settings(conn, &a.tenant_id, &a.branch_id)?;
    Ok(json!({
        "tenant_id": a.tenant_id, "branch_id": a.branch_id, "role": a.role,
        "session_duration_ms": settings.session_duration_ms,
        "auto_lock_ms": settings.auto_lock_ms,
        "pin_required": settings.pin_required,
        "max_failed_attempts": settings.max_failed_attempts,
        "lockout_ms": settings.lockout_ms,
        "backup_encryption_required": settings.backup_encryption_required
    }))
}

pub fn set_security_policy(conn: &mut Connection, payload: &Value) -> CoreResult<Value> {
    let a = actor(conn, payload)?;
    assert_permission(conn, &a, "security.admin")?;
    let branch = field_str_opt(payload, "branch_id").unwrap_or(&a.branch_id);
    let current = security_settings(conn, &a.tenant_id, branch)?;
    let duration = field_i64_or(payload, "session_duration_ms", current.session_duration_ms);
    let auto_lock = field_i64_or(payload, "auto_lock_ms", current.auto_lock_ms);
    let max_failed = field_i64_or(payload, "max_failed_attempts", current.max_failed_attempts);
    let lockout = field_i64_or(payload, "lockout_ms", current.lockout_ms);
    if duration < 60_000 || auto_lock < 0 || max_failed < 1 || lockout < 0 {
        return Err(CoreError::new("INVALID_SECURITY_POLICY", "Politique de sécurité invalide."));
    }
    let pin_required = field_bool(payload, "pin_required", current.pin_required);
    let encrypted = field_bool(payload, "backup_encryption_required", current.backup_encryption_required);
    conn.execute(
        "INSERT INTO security_settings (tenant_id, branch_id, session_duration_ms, auto_lock_ms,
            pin_required, max_failed_attempts, lockout_ms, backup_encryption_required, updated_at)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9)
         ON CONFLICT(tenant_id, branch_id) DO UPDATE SET
           session_duration_ms=excluded.session_duration_ms, auto_lock_ms=excluded.auto_lock_ms,
           pin_required=excluded.pin_required, max_failed_attempts=excluded.max_failed_attempts,
           lockout_ms=excluded.lockout_ms, backup_encryption_required=excluded.backup_encryption_required,
           updated_at=excluded.updated_at",
        params![a.tenant_id, branch, duration, auto_lock, i64::from(pin_required), max_failed,
                lockout, i64::from(encrypted), now_ms()],
    )?;
    db::audit_security(conn, &a.branch_id, &a.user_id, Some(&a.device_id), "SECURITY_POLICY_UPDATED", branch, "WARN", "ADMIN")?;
    get_security_policy(conn, payload)
}

pub fn set_role_permission(conn: &mut Connection, payload: &Value) -> CoreResult<Value> {
    let a = actor(conn, payload)?;
    assert_permission(conn, &a, "security.admin")?;
    let branch = field_str(payload, "branch_id")?;
    let role = field_str(payload, "role")?;
    let permission = field_str(payload, "permission")?;
    let allowed = payload
        .get("allowed")
        .and_then(Value::as_bool)
        .ok_or_else(|| CoreError::new("MISSING_FIELD", "Champ booléen requis : allowed"))?;
    if branch != "*" && !matches!(role, "OWNER" | "ADMIN" | "MANAGER" | "CASHIER" | "WAREHOUSE" | "ACCOUNTANT" | "READ_ONLY") {
        return Err(CoreError::new("INVALID_ROLE", "Rôle inconnu."));
    }
    conn.execute(
        "INSERT INTO role_permissions (tenant_id, branch_id, role, permission, allowed, updated_at)
         VALUES (?1,?2,?3,?4,?5,?6)
         ON CONFLICT(tenant_id, branch_id, role, permission) DO UPDATE SET
           allowed=excluded.allowed, updated_at=excluded.updated_at",
        params![a.tenant_id, branch, role, permission, i64::from(allowed), now_ms()],
    )?;
    db::audit_security(conn, &a.branch_id, &a.user_id, Some(&a.device_id), "ROLE_PERMISSION_UPDATED", &format!("{role}:{permission}={allowed}@{branch}"), "WARN", "ADMIN")?;
    Ok(json!({ "role": role, "permission": permission, "allowed": allowed, "branch_id": branch }))
}

/// Révocation d'un téléphone perdu : appareil et toutes ses sessions sont invalidés.
pub fn revoke_device(conn: &mut Connection, payload: &Value) -> CoreResult<Value> {
    let a = actor(conn, payload)?;
    assert_permission(conn, &a, "security.admin")?;
    let device_id = field_str(payload, "device_id")?;
    let reason = field_str_opt(payload, "reason").unwrap_or("LOST_DEVICE");
    let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
    tx.execute(
        "UPDATE devices SET is_authorized=0, revoked_at=?2 WHERE id=?1",
        params![device_id, now_ms()],
    )?;
    tx.execute(
        "UPDATE sessions SET revoked=1, revoked_at=?2, revocation_reason=?3 WHERE device_id=?1 AND revoked=0",
        params![device_id, now_ms(), reason],
    )?;
    db::audit_security(&tx, &a.branch_id, &a.user_id, Some(&a.device_id), "DEVICE_REVOKED", &format!("{device_id}:{reason}"), "WARN", "ADMIN")?;
    tx.commit()?;
    Ok(json!({ "device_id": device_id, "status": "REVOKED", "sessions_revoked": true }))
}

pub fn get_audit_log(conn: &Connection, payload: &Value) -> CoreResult<Value> {
    let a = actor(conn, payload)?;
    assert_permission(conn, &a, "read")?;
    let since = field_i64_or(payload, "since_ms", 0);
    let category = field_str_opt(payload, "category").unwrap_or("SECURITY");
    let mut stmt = conn.prepare(
        "SELECT id, branch_id, user_id, device_id, action, severity, category, occurred_at
         FROM audit_logs WHERE branch_id=?1 AND occurred_at>=?2 AND category=?3
         ORDER BY occurred_at DESC LIMIT 100",
    )?;
    let entries = stmt.query_map(params![a.branch_id, since, category], |row| Ok(json!({
        "id": row.get::<_, String>(0)?, "branch_id": row.get::<_, String>(1)?,
        "user_id": row.get::<_, String>(2)?, "device_id": row.get::<_, Option<String>>(3)?,
        "action": row.get::<_, String>(4)?, "severity": row.get::<_, String>(5)?,
        "category": row.get::<_, String>(6)?, "occurred_at": row.get::<_, i64>(7)?
    })))?.collect::<Result<Vec<_>, _>>()?;
    Ok(json!({ "entries": entries }))
}

/// Points de vente et entrepôts du même établissement, filtrés par droits.
pub fn list_branches(conn: &Connection, payload: &Value) -> CoreResult<Value> {
    let a = actor(conn, payload)?;
    let mut stmt = conn.prepare(
        "SELECT b.id, b.name, b.city, b.kind, b.parent_branch_id
         FROM branches b WHERE b.tenant_id=?1 AND b.is_active=1
           AND (NOT EXISTS (SELECT 1 FROM user_branch_access u WHERE u.user_id=?2)
                OR EXISTS (SELECT 1 FROM user_branch_access u
                           WHERE u.user_id=?2 AND u.branch_id=b.id))
         ORDER BY b.kind, b.id",
    )?;
    let branches = stmt.query_map(params![a.tenant_id, a.user_id], |row| Ok(json!({
        "branch_id": row.get::<_, String>(0)?, "name": row.get::<_, String>(1)?,
        "city": row.get::<_, String>(2)?, "kind": row.get::<_, String>(3)?,
        "parent_branch_id": row.get::<_, Option<String>>(4)?
    })))?.collect::<Result<Vec<_>, _>>()?;
    Ok(json!({ "tenant_id": a.tenant_id, "branches": branches }))
}

/// Transfert traçable entre deux points de vente/entrepôts du même établissement.
pub fn create_stock_transfer(conn: &mut Connection, payload: &Value) -> CoreResult<Value> {
    let a = actor(conn, payload)?;
    assert_permission(conn, &a, "inventory.write")?;
    let from = field_str(payload, "from_branch_id")?;
    let to = field_str(payload, "to_branch_id")?;
    let reason = field_str(payload, "reason")?;
    if from == to {
        return Err(CoreError::new("SAME_BRANCH", "Source et destination doivent être distincts."));
    }
    let branches = list_branches(conn, payload)?;
    let allowed: Vec<&str> = branches["branches"]
        .as_array()
        .map(|items| items.iter().filter_map(|item| item["branch_id"].as_str()).collect())
        .unwrap_or_default();
    if !allowed.contains(&from) || !allowed.contains(&to) {
        return Err(CoreError::new("BRANCH_FORBIDDEN", "Transfert hors périmètre autorisé."));
    }
    let lines = field_array(payload, "lines")?.clone();
    if lines.is_empty() {
        return Err(CoreError::new("EMPTY_TRANSFER", "Au moins une ligne est requise."));
    }
    let id = new_id("xfer");
    let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
    for line in &lines {
        let variant = field_str(line, "variant_id")?;
        let qty = field_i64(line, "qty")?;
        if qty <= 0 {
            return Err(CoreError::new("INVALID_QUANTITY", "Quantité de transfert entière et positive requise."));
        }
        let available: i64 = tx.query_row(
            "SELECT full_units_qty FROM stock_balances WHERE branch_id=?1 AND variant_id=?2",
            params![from, variant], |row| row.get(0),
        ).optional()?.unwrap_or(0);
        if available < qty {
            return Err(CoreError::new("STOCK_INSUFFICIENT", "Stock source insuffisant."));
        }
        tx.execute(
            "UPDATE stock_balances SET full_units_qty=full_units_qty-?3 WHERE branch_id=?1 AND variant_id=?2",
            params![from, variant, qty],
        )?;
        tx.execute(
            "INSERT INTO stock_balances (branch_id, variant_id, full_units_qty, min_alert_units)
             VALUES (?1,?2,?3,12)
             ON CONFLICT(branch_id, variant_id) DO UPDATE SET
               full_units_qty=full_units_qty+excluded.full_units_qty,
               server_revision=server_revision+1",
            params![to, variant, qty],
        )?;
        tx.execute(
            "INSERT INTO stock_movements (id, branch_id, variant_id, movement_type, qty, reason, document_id, user_id, device_id, occurred_at)
             VALUES (?1,?2,?3,'TRANSFER_OUT',?4,?5,?6,?7,?8,?9)",
            params![new_id("smov"), from, variant, -qty, reason, id, a.user_id, a.device_id, now_ms()],
        )?;
        tx.execute(
            "INSERT INTO stock_movements (id, branch_id, variant_id, movement_type, qty, reason, document_id, user_id, device_id, occurred_at)
             VALUES (?1,?2,?3,'TRANSFER_IN',?4,?5,?6,?7,?8,?9)",
            params![new_id("smov"), to, variant, qty, reason, id, a.user_id, a.device_id, now_ms()],
        )?;
        tx.execute(
            "INSERT INTO stock_transfer_lines (id, transfer_id, variant_id, qty) VALUES (?1,?2,?3,?4)",
            params![new_id("stl"), id, variant, qty],
        )?;
    }
    tx.execute(
        "INSERT INTO stock_transfers (id, tenant_id, from_branch_id, to_branch_id, status, reason, created_by, created_at)
         VALUES (?1,?2,?3,?4,'COMPLETED',?5,?6,?7)",
        params![id, a.tenant_id, from, to, reason, a.user_id, now_ms()],
    )?;
    db::enqueue(
        &tx, &a.tenant_id, &from, &a.device_id, &a.user_id, "STOCK_TRANSFER",
        &format!("Transfert {from} → {to}"), &json!({"transfer_id": id}).to_string(), 1, &new_id("idem"),
    )?;
    db::audit_security(&tx, &from, &a.user_id, Some(&a.device_id), "STOCK_TRANSFER", &format!("{from}->{to}"), "INFO", "STOCK")?;
    tx.commit()?;
    Ok(json!({ "transfer_id": id, "status": "COMPLETED", "from_branch_id": from, "to_branch_id": to }))
}

pub fn list_stock_transfers(conn: &Connection, payload: &Value) -> CoreResult<Value> {
    let a = actor(conn, payload)?;
    let mut stmt = conn.prepare(
        "SELECT t.id, t.from_branch_id, t.to_branch_id, t.status, t.reason, t.created_at,
                COALESCE(SUM(l.qty),0)
         FROM stock_transfers t LEFT JOIN stock_transfer_lines l ON l.transfer_id=t.id
         WHERE t.tenant_id=?1 GROUP BY t.id ORDER BY t.created_at DESC LIMIT 50",
    )?;
    let transfers = stmt.query_map(params![a.tenant_id], |row| Ok(json!({
        "transfer_id": row.get::<_, String>(0)?, "from_branch_id": row.get::<_, String>(1)?,
        "to_branch_id": row.get::<_, String>(2)?, "status": row.get::<_, String>(3)?,
        "reason": row.get::<_, String>(4)?, "created_at": row.get::<_, i64>(5)?,
        "total_qty": row.get::<_, i64>(6)?
    })))?.collect::<Result<Vec<_>, _>>()?;
    Ok(json!({ "transfers": transfers }))
}

fn stable_checksum(text: &str) -> String {
    let mut hash: u64 = 0xcbf29ce484222325;
    for byte in text.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{hash:016x}")
}

/// Export neutre pour DEPOTPRO Desktop : données et curseur, jamais de calcul recalculé côté TS.
pub fn export_desktop_snapshot(conn: &mut Connection, payload: &Value) -> CoreResult<Value> {
    let a = actor(conn, payload)?;
    let branch = field_str_opt(payload, "branch_id").unwrap_or(&a.branch_id);
    if branch != a.branch_id {
        return Err(CoreError::new("BRANCH_FORBIDDEN", "Export limité au point de vente courant."));
    }
    let catalog = get_inventory(conn, &json!({ "auth": { "token": payload["auth"]["token"].as_str().unwrap_or("") } }))?;
    let customers = get_customers(conn, payload)?;
    let suppliers = get_suppliers(conn, payload)?;
    let cursor: i64 = conn.query_row(
        "SELECT server_sequence FROM sync_cursors WHERE branch_id=?1",
        params![branch], |row| row.get(0),
    ).optional()?.unwrap_or(0);
    let mut stmt = conn.prepare(
        "SELECT variant_id, full_units_qty, server_revision FROM stock_balances WHERE branch_id=?1 ORDER BY variant_id",
    )?;
    let stock = stmt.query_map(params![branch], |row| Ok(json!({
        "variant_id": row.get::<_, String>(0)?, "qty": row.get::<_, i64>(1)?,
        "revision": row.get::<_, i64>(2)?
    })))?.collect::<Result<Vec<_>, _>>()?;
    drop(stmt);
    let body = json!({
        "tenant_id": a.tenant_id, "branch_id": branch, "catalog": catalog,
        "customers": customers, "suppliers": suppliers, "stock": stock, "server_sequence": cursor
    });
    let checksum = stable_checksum(&body.to_string());
    conn.execute(
        "INSERT INTO desktop_snapshots (id, tenant_id, branch_id, schema_version, checksum, exported_at)
         VALUES (?1,?2,?3,1,?4,?5)",
        params![new_id("snap"), a.tenant_id, branch, checksum, now_ms()],
    )?;
    Ok(json!({
        "schema_version": 1, "tenant_id": a.tenant_id, "branch_id": branch,
        "exported_at": now_ms(), "checksum": checksum, "data": body,
        "rules_source": "depotpro-core", "rules_version": 1
    }))
}

pub fn dispatch(conn: &mut Connection, command: &str, payload: &Value) -> CoreResult<Value> {
    if let Some(permission) = permission_for_command(command) {
        let a = actor(conn, payload)?;
        assert_permission(conn, &a, permission)?;
    }
    match command {
        "authenticate" | "authenticateUser" => authenticate(conn, payload),
        "renewSession" | "renew_session" => renew_session(conn, payload),
        "revokeSession" | "revoke_session" => revoke_session(conn, payload),
        "lockSession" | "lock_session" => lock_session(conn, payload),
        "unlockSession" | "unlock_session" => unlock_session(conn, payload),
        "checkPermission" | "check_permission" => check_permission_command(conn, payload),
        "getSecurityPolicy" | "get_security_policy" => get_security_policy(conn, payload),
        "setSecurityPolicy" | "set_security_policy" => set_security_policy(conn, payload),
        "setRolePermission" | "set_role_permission" => set_role_permission(conn, payload),
        "revokeDevice" | "revoke_device" => revoke_device(conn, payload),
        "getAuditLog" | "get_audit_log" => get_audit_log(conn, payload),
        "listBranches" | "list_branches" => list_branches(conn, payload),
        "createStockTransfer" | "create_stock_transfer" => create_stock_transfer(conn, payload),
        "listStockTransfers" | "list_stock_transfers" => list_stock_transfers(conn, payload),
        "exportDesktopSnapshot" | "export_desktop_snapshot" => export_desktop_snapshot(conn, payload),
        "importDesktopUpdates" => apply_sync_updates(conn, payload),
        "getDashboard" | "get_dashboard" => get_dashboard(conn, payload),
        "validateSale" | "validate_sale" => validate_sale(conn, payload),
        "createSale" | "create_sale" => create_sale(conn, payload),
        "createMixedOrder" | "create_mixed_order" => create_mixed_order(conn, payload),
        "registerPackagingReturn" | "register_packaging_return" => register_packaging_return(conn, payload),
        "createPurchaseOrder" | "create_purchase_order" => create_purchase_order(conn, payload),
        "receivePurchase" | "receive_purchase" => receive_purchase(conn, payload),
        "recordSupplierReturn" | "record_supplier_return" => record_supplier_return(conn, payload),
        "getPurchaseVariance" | "get_purchase_variance" => get_purchase_variance(conn, payload),
        "getInventory" | "get_inventory" => get_inventory(conn, payload),
        "recordInventoryCount" | "record_inventory_count" => record_inventory_count(conn, payload),
        "getStockMovements" | "get_stock_movements" => get_stock_movements(conn, payload),
        "getCustomers" | "get_customers" => get_customers(conn, payload),
        "getSuppliers" | "get_suppliers" => get_suppliers(conn, payload),
        "getSales" | "get_sales" => get_sales(conn, payload),
        "getCustomerAccount" | "get_customer_account" => get_customer_account(conn, payload),
        "getSupplierAccount" | "get_supplier_account" => get_supplier_account(conn, payload),
        "recordPayment" | "record_payment" => record_payment(conn, payload),
        "getPendingOperations" | "get_pending_operations" => get_pending_operations(conn, payload),
        "startSynchronization" | "start_synchronization" => start_synchronization(conn, payload),
        "getSynchronizationStatus" | "get_synchronization_status" => get_synchronization_status(conn, payload),
        "getSyncHistory" | "get_sync_history" => get_sync_history(conn, payload),
        "configureSyncServer" | "configure_sync_server" => configure_sync_server(conn, payload),
        "applySyncAcknowledgements" | "apply_sync_acknowledgements" => apply_sync_ack(conn, payload),
        "changePin" | "change_pin" => change_pin(conn, payload),
        "applySyncUpdates" | "apply_sync_updates" => apply_sync_updates(conn, payload),
        "resolveConflict" | "resolve_conflict" => resolve_conflict(conn, payload),
        "configureDeviceQuota" | "configure_device_quota" => configure_device_quota(conn, payload),
        "getCrateTypes" | "get_crate_types" => get_crate_types(conn, payload),
        "getPurchaseOrders" | "get_purchase_orders" => get_purchase_orders(conn, payload),
        "getSaleReceipt" | "get_sale_receipt" => get_sale_receipt(conn, payload),
        "getCashSession" | "get_cash_session" => get_cash_session(conn, payload),
        "openCashSession" | "open_cash_session" => open_cash_session(conn, payload),
        "recordCashMovement" | "record_cash_movement" => record_cash_movement(conn, payload),
        "closeCashSession" | "close_cash_session" => close_cash_session(conn, payload),
        "getReports" | "get_reports" => get_reports(conn, payload),
        "getConflicts" | "get_conflicts" => get_conflicts(conn, payload),
        "resetStaleSyncing" | "reset_stale_syncing" => reset_stale_syncing(conn, payload),
        "saveProductVariant" | "save_product_variant" => save_product_variant(conn, payload),
        "updateVariantPrices" | "update_variant_prices" => update_variant_prices(conn, payload),
        "archiveProductVariant" | "archive_product_variant" => archive_product_variant(conn, payload),
        "createCrateType" | "create_crate_type" => create_crate_type(conn, payload),
        "updateCrateType" | "update_crate_type" => update_crate_type(conn, payload),
        "createCustomer" | "create_customer" => create_customer(conn, payload),
        "getCrateStock" | "get_crate_stock" => crate::crates::get_crate_stock(conn, payload),
        "getCrateStockByType" | "get_crate_stock_by_type" => crate::crates::get_crate_stock_by_type(conn, payload),
        "getCustomerCrateBalance" | "get_customer_crate_balance" => crate::crates::get_customer_crate_balance(conn, payload),
        "getSupplierCrateBalance" | "get_supplier_crate_balance" => crate::crates::get_supplier_crate_balance(conn, payload),
        "getCrateMovementHistory" | "get_crate_movement_history" => crate::crates::get_crate_movement_history(conn, payload),
        "getCrateValuationHistory" | "get_crate_valuation_history" => crate::crates::get_crate_valuation_history(conn, payload),
        "configureCrateValuation" | "configure_crate_valuation" => crate::crates::configure_crate_valuation(conn, payload),
        "recordCustomerCrateReturn" | "record_customer_crate_return" => crate::crates::record_customer_crate_return(conn, payload),
        "recordSupplierCrateExchange" | "record_supplier_crate_exchange" => crate::crates::record_supplier_crate_exchange(conn, payload),
        "recordCrateAdjustment" | "record_crate_adjustment" => crate::crates::record_crate_adjustment(conn, payload),
        "recordCrateLoss" | "record_crate_loss" => crate::crates::record_crate_loss(conn, payload),
        "recordEmptyCrateSale" | "record_empty_crate_sale" => crate::crates::record_empty_crate_sale(conn, payload),
        "getCatalogDashboard" | "get_catalog_dashboard" => crate::catalog::get_catalog_dashboard(conn, payload),
        "getCatalogProducers" | "get_catalog_producers" => crate::catalog::get_catalog_producers(conn, payload),
        "getCatalogCategories" | "get_catalog_categories" => crate::catalog::get_catalog_categories(conn, payload),
        "getCatalogBrands" | "get_catalog_brands" => crate::catalog::get_catalog_brands(conn, payload),
        "getCatalogProducts" | "get_catalog_products" => crate::catalog::get_catalog_products(conn, payload),
        "getCatalogImportHistory" | "get_catalog_import_history" => crate::catalog::get_catalog_import_history(conn, payload),
        "getBreweryCatalogs" | "get_brewery_catalogs" => crate::catalog::get_brewery_catalogs(conn, payload),
        "previewCatalogImport" | "preview_catalog_import" => crate::catalog::preview_catalog_import(conn, payload),
        "importCatalogJson" | "import_catalog_json" => crate::catalog::import_catalog_json(conn, payload),
        "createCatalogProducer" | "create_catalog_producer" => crate::catalog::create_catalog_producer(conn, payload),
        "updateCatalogProducer" | "update_catalog_producer" => crate::catalog::update_catalog_producer(conn, payload),
        "createCatalogCategory" | "create_catalog_category" => crate::catalog::create_catalog_category(conn, payload),
        "createCatalogBrand" | "create_catalog_brand" => crate::catalog::create_catalog_brand(conn, payload),
        "createCatalogProduct" | "create_catalog_product" => crate::catalog::create_catalog_product(conn, payload),
        "activateCatalogProduct" | "activate_catalog_product" => crate::catalog::activate_catalog_product(conn, payload),
        "deactivateCatalogProduct" | "deactivate_catalog_product" => crate::catalog::deactivate_catalog_product(conn, payload),
        "updateCatalogVariant" | "update_catalog_variant" => crate::catalog::update_catalog_variant(conn, payload),
        "createBreweryCatalog" | "create_brewery_catalog" => crate::catalog::create_brewery_catalog(conn, payload),
        "updateBreweryCatalog" | "update_brewery_catalog" => crate::catalog::update_brewery_catalog(conn, payload),
        "getCrateMigrationReview" | "get_crate_migration_review" => crate::crates::get_crate_migration_review(conn, payload),
        "resolveCrateMigrationReview" | "resolve_crate_migration_review" => crate::crates::resolve_crate_migration_review(conn, payload),
        "getPackagingStock" | "get_packaging_stock" => get_packaging_stock(conn, payload),
        "getPartnerStatement" | "get_partner_statement" => get_partner_statement(conn, payload),
        "getPackagingAccount" | "get_packaging_account" => get_packaging_account(conn, payload),
        "correctPackagingMovement" | "correct_packaging_movement" => correct_packaging_movement(conn, payload),
        other => Err(CoreError::new("UNKNOWN_COMMAND", format!("Commande inconnue : {other}"))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_edition_requires_management_role() {
        assert!(assert_catalog_role("OWNER").is_ok());
        assert!(assert_catalog_role("ADMIN").is_ok());
        assert!(assert_catalog_role("MANAGER").is_ok());
        assert!(assert_catalog_role("CASHIER").is_err());
        assert!(assert_catalog_role("READ_ONLY").is_err());
    }

    #[test]
    fn negative_price_is_refused_before_any_write() {
        assert!(assert_non_negative_prices(0, 0, 0).is_ok());
        assert_eq!(assert_non_negative_prices(-1, 0, 0).unwrap_err().code, "INVALID_PRICE");
        assert_eq!(assert_non_negative_prices(100, -1, 0).unwrap_err().code, "INVALID_PRICE");
        assert_eq!(assert_non_negative_prices(100, 100, -1).unwrap_err().code, "INVALID_PRICE");
    }
}
