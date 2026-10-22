use super::*;

#[derive(Clone, Debug, serde::Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct AccountBilling {
    pub group_id: Option<i64>,
    pub quota_mode: String,
    pub monthly_gb: Option<u64>,
}

impl Default for AccountBilling {
    fn default() -> Self {
        Self {
            group_id: None,
            quota_mode: "default".into(),
            monthly_gb: None,
        }
    }
}

impl AccountBilling {
    pub fn validate(&self) -> anyhow::Result<()> {
        anyhow::ensure!(
            matches!(self.quota_mode.as_str(), "default" | "custom" | "unlimited"),
            "invalid quota mode"
        );
        anyhow::ensure!(
            (self.quota_mode == "custom") == self.monthly_gb.is_some(),
            "custom quota requires monthly_gb; other modes must omit it"
        );
        if let Some(gb) = self.monthly_gb {
            anyhow::ensure!(
                gb <= (i64::MAX as u64) / (1024 * 1024 * 1024),
                "monthly quota is too large"
            );
        }
        Ok(())
    }
}

#[derive(Serialize)]
pub struct RegistrationInvitation {
    pub id: i64,
    pub status: String,
    pub expires_at: i64,
    pub created_at: i64,
    pub billing: AccountBilling,
}

#[derive(Debug, PartialEq, Eq)]
pub enum LocalCreateOutcome {
    Created(i64),
    UsernameExists,
    InvalidInvitation,
    InvalidGroup,
}

pub struct LocalUserCreation<'a> {
    pub actor: &'a str,
    pub username: &'a str,
    pub password: String,
    pub display_name: &'a str,
    pub billing: &'a AccountBilling,
    pub invitation_token: Option<&'a str>,
    pub routing_length: u8,
}

impl Database {
    pub(super) async fn migrate_local_accounts(&self) -> anyhow::Result<()> {
        let mut tx = self.pool.begin().await?;
        let version: i64 =
            sqlx::query_scalar("SELECT COALESCE(MAX(version), 0) FROM schema_migrations")
                .fetch_one(&mut *tx)
                .await?;
        if version < CURRENT_SCHEMA_VERSION {
            sqlx::query("ALTER TABLE users ADD COLUMN username TEXT COLLATE NOCASE")
                .execute(&mut *tx)
                .await?;
            sqlx::query(
                "CREATE UNIQUE INDEX users_username ON users(username) WHERE username IS NOT NULL",
            )
            .execute(&mut *tx)
            .await?;
            sqlx::query("CREATE TABLE local_user_credentials (user_id INTEGER PRIMARY KEY REFERENCES users(id), password_hash TEXT NOT NULL, failed_attempts INTEGER NOT NULL DEFAULT 0, locked_until INTEGER NOT NULL DEFAULT 0)").execute(&mut *tx).await?;
            sqlx::query("CREATE TABLE registration_invitations (id INTEGER PRIMARY KEY AUTOINCREMENT, token_hash TEXT NOT NULL UNIQUE, billing_json TEXT NOT NULL, status TEXT NOT NULL DEFAULT 'pending', expires_at INTEGER NOT NULL, created_at INTEGER NOT NULL, created_by TEXT NOT NULL, user_id INTEGER REFERENCES users(id))").execute(&mut *tx).await?;
            sqlx::query("INSERT INTO schema_migrations (version, name, applied_at) VALUES (?, 'local-users-and-registration-invitations', ?)").bind(CURRENT_SCHEMA_VERSION).bind(unix_timestamp()).execute(&mut *tx).await?;
        }
        tx.commit().await?;
        Ok(())
    }

    pub async fn create_registration_invitation(
        &self,
        actor: &str,
        billing: &AccountBilling,
        ttl_hours: u32,
    ) -> anyhow::Result<Option<(i64, String)>> {
        billing.validate()?;
        let token = random_secret(48);
        let mut tx = self.pool.begin().await?;
        if !valid_group(&mut tx, billing.group_id).await? {
            return Ok(None);
        }
        let now = unix_timestamp();
        let result = sqlx::query("INSERT INTO registration_invitations (token_hash, billing_json, expires_at, created_at, created_by) VALUES (?, ?, ?, ?, ?)")
            .bind(hash_token(&token)).bind(serde_json::to_string(billing)?).bind(now + i64::from(ttl_hours) * 3600).bind(now).bind(actor).execute(&mut *tx).await?;
        let id = result.last_insert_rowid();
        local_audit(
            &mut tx,
            actor,
            "registration_invitation_created",
            &format!("invitation_id={id}"),
        )
        .await?;
        tx.commit().await?;
        Ok(Some((id, token)))
    }

    pub async fn registration_invitations(&self) -> anyhow::Result<Vec<RegistrationInvitation>> {
        sqlx::query("SELECT id, status, expires_at, created_at, billing_json FROM registration_invitations ORDER BY id DESC LIMIT 100")
            .fetch_all(&self.pool).await?.into_iter().map(|row| {
                let expires_at: i64 = row.try_get("expires_at")?;
                let mut status: String = row.try_get("status")?;
                if status == "pending" && expires_at <= unix_timestamp() { status = "expired".into(); }
                Ok(RegistrationInvitation { id: row.try_get("id")?, status, expires_at, created_at: row.try_get("created_at")?, billing: serde_json::from_str(&row.try_get::<String, _>("billing_json")?)? })
            }).collect()
    }

    pub async fn revoke_registration_invitation(
        &self,
        actor: &str,
        id: i64,
    ) -> anyhow::Result<bool> {
        let mut tx = self.pool.begin().await?;
        let changed = sqlx::query("UPDATE registration_invitations SET status = 'revoked' WHERE id = ? AND status = 'pending'").bind(id).execute(&mut *tx).await?.rows_affected() > 0;
        if changed {
            local_audit(
                &mut tx,
                actor,
                "registration_invitation_revoked",
                &format!("invitation_id={id}"),
            )
            .await?;
        }
        tx.commit().await?;
        Ok(changed)
    }

    pub async fn create_local_user(
        &self,
        input: LocalUserCreation<'_>,
    ) -> anyhow::Result<LocalCreateOutcome> {
        let LocalUserCreation {
            actor,
            username,
            password,
            display_name,
            billing,
            invitation_token,
            routing_length,
        } = input;
        billing.validate()?;
        let password_hash = run_argon2_task(move || hash_password(&password)).await??;
        let mut tx = self.pool.begin().await?;
        // First operation is a write, serializing competing claims without a deferred read/write upgrade.
        let (invitation_id, billing) = if let Some(token) = invitation_token {
            let claimed = sqlx::query("UPDATE registration_invitations SET status = 'accepted' WHERE token_hash = ? AND status = 'pending' AND expires_at > ? RETURNING id, billing_json")
                .bind(hash_token(token)).bind(unix_timestamp()).fetch_optional(&mut *tx).await?;
            let Some(row) = claimed else {
                return Ok(LocalCreateOutcome::InvalidInvitation);
            };
            (
                Some(row.try_get::<i64, _>("id")?),
                serde_json::from_str::<AccountBilling>(&row.try_get::<String, _>("billing_json")?)?,
            )
        } else {
            sqlx::query("UPDATE users SET updated_at = updated_at WHERE id = -1")
                .execute(&mut *tx)
                .await?;
            (None, billing.clone())
        };
        if !valid_group(&mut tx, billing.group_id).await? {
            return Ok(LocalCreateOutcome::InvalidGroup);
        }
        let now = unix_timestamp();
        // A non-deliverable internal address preserves existing email-based database contracts.
        // It is never supplied by the registrant, so unverified addresses cannot claim an email identity.
        let internal_email = format!("{}@accounts.invalid", Uuid::new_v4());
        let insert = sqlx::query("INSERT INTO users (username, email, display_name, disabled, created_at, updated_at) VALUES (?, ?, ?, 0, ?, ?) ON CONFLICT DO NOTHING")
            .bind(username).bind(internal_email).bind(display_name).bind(now).bind(now).execute(&mut *tx).await?;
        if insert.rows_affected() == 0 {
            return Ok(LocalCreateOutcome::UsernameExists);
        }
        let id = insert.last_insert_rowid();
        insert_unique_routing_id(&mut tx, id, routing_length, now).await?;
        sqlx::query("INSERT INTO local_user_credentials (user_id, password_hash) VALUES (?, ?)")
            .bind(id)
            .bind(password_hash)
            .execute(&mut *tx)
            .await?;
        if let Some(group_id) = billing.group_id {
            sqlx::query("INSERT INTO group_members (group_id, user_id, is_billing, created_at) VALUES (?, ?, 1, ?)").bind(group_id).bind(id).bind(now).execute(&mut *tx).await?;
        }
        if !matches!(billing.quota_mode.as_str(), "default") {
            sqlx::query("INSERT INTO user_quota_overrides (user_id, mode, monthly_limit_bytes, updated_at) VALUES (?, ?, ?, ?)")
                .bind(id).bind(&billing.quota_mode).bind(billing.monthly_gb.map(|gb| (gb * 1024 * 1024 * 1024) as i64)).bind(now).execute(&mut *tx).await?;
        }
        if let Some(invitation_id) = invitation_id {
            sqlx::query("UPDATE registration_invitations SET user_id = ? WHERE id = ?")
                .bind(id)
                .bind(invitation_id)
                .execute(&mut *tx)
                .await?;
        }
        local_audit(
            &mut tx,
            actor,
            "user_created",
            &format!("user_id={id}; method=password"),
        )
        .await?;
        tx.commit().await?;
        Ok(LocalCreateOutcome::Created(id))
    }

    /// Verifies local credentials and returns a session with an independent random token.
    pub async fn authenticate_local_user(
        &self,
        username: &str,
        password: String,
    ) -> anyhow::Result<Option<UserSession>> {
        let row = sqlx::query("SELECT u.id, c.password_hash, c.locked_until FROM users u JOIN local_user_credentials c ON c.user_id = u.id WHERE u.username = ? COLLATE NOCASE AND u.disabled = 0 AND u.deleted_at IS NULL")
            .bind(username).fetch_optional(&self.pool).await?;
        let (id, stored, locked) = match row {
            Some(row) => (
                Some(row.try_get::<i64, _>("id")?),
                Some(row.try_get::<String, _>("password_hash")?),
                row.try_get::<i64, _>("locked_until")? > unix_timestamp(),
            ),
            None => (None, None, false),
        };
        let previous_hash = stored.clone();
        // Unknown accounts also perform Argon2 work to reduce username timing disclosure.
        let valid = run_argon2_task(move || match stored {
            Some(hash) => argon2_matches(&password, &hash),
            None => {
                let _ = hash_password(&password);
                false
            }
        })
        .await?;
        let Some(id) = id else {
            return Ok(None);
        };
        if locked {
            return Ok(None);
        }
        if !valid {
            sqlx::query("UPDATE local_user_credentials SET failed_attempts = CASE WHEN locked_until > 0 AND locked_until <= ? THEN 1 ELSE failed_attempts + 1 END, locked_until = CASE WHEN locked_until > 0 AND locked_until <= ? THEN 0 WHEN failed_attempts + 1 >= 5 THEN ? ELSE locked_until END WHERE user_id = ? AND password_hash = ? AND locked_until <= ?")
                .bind(unix_timestamp()).bind(unix_timestamp()).bind(unix_timestamp() + 900).bind(id).bind(&previous_hash).bind(unix_timestamp()).execute(&self.pool).await?;
            return Ok(None);
        }
        let mut tx = self.pool.begin().await?;
        let updated = sqlx::query("UPDATE local_user_credentials SET failed_attempts = 0, locked_until = 0 WHERE user_id = ? AND password_hash = ? AND locked_until <= ?")
            .bind(id).bind(previous_hash).bind(unix_timestamp()).execute(&mut *tx).await?.rows_affected();
        if updated == 0 {
            return Ok(None);
        }
        let Some(row) = sqlx::query("SELECT email, display_name FROM users WHERE id = ? AND disabled = 0 AND deleted_at IS NULL")
            .bind(id).fetch_optional(&mut *tx).await? else { return Ok(None); };
        let now = unix_timestamp();
        let token = random_secret(48);
        let expires_at = now + USER_SESSION_LIFETIME_SECS;
        sqlx::query("INSERT INTO user_sessions (token_hash, user_id, auth_method, created_at, expires_at, last_used_at) VALUES (?, ?, 'password', ?, ?, ?)")
            .bind(hash_token(&token)).bind(id).bind(now).bind(expires_at).bind(now).execute(&mut *tx).await?;
        local_audit(
            &mut tx,
            &format!("user:{id}"),
            "user_login_succeeded",
            "method=password",
        )
        .await?;
        tx.commit().await?;
        Ok(Some(UserSession {
            token,
            expires_at,
            identity: UserIdentity {
                user_id: id,
                email: row.try_get("email")?,
                display_name: row.try_get("display_name")?,
            },
        }))
    }

    pub async fn change_local_password(
        &self,
        user_id: i64,
        current: String,
        replacement: String,
    ) -> anyhow::Result<bool> {
        let Some(hash) = sqlx::query_scalar::<_, String>(
            "SELECT password_hash FROM local_user_credentials WHERE user_id = ?",
        )
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await?
        else {
            return Ok(false);
        };
        let previous = hash.clone();
        let next = run_argon2_task(move || {
            if argon2_matches(&current, &hash) {
                hash_password(&replacement).map(Some)
            } else {
                Ok(None)
            }
        })
        .await??;
        let Some(next) = next else {
            return Ok(false);
        };
        let mut tx = self.pool.begin().await?;
        let changed = sqlx::query("UPDATE local_user_credentials SET password_hash = ?, failed_attempts = 0, locked_until = 0 WHERE user_id = ? AND password_hash = ?").bind(next).bind(user_id).bind(previous).execute(&mut *tx).await?.rows_affected() > 0;
        if changed {
            sqlx::query("DELETE FROM user_sessions WHERE user_id = ?")
                .bind(user_id)
                .execute(&mut *tx)
                .await?;
            local_audit(
                &mut tx,
                &format!("user:{user_id}"),
                "user_password_changed",
                "all sessions revoked",
            )
            .await?;
        }
        tx.commit().await?;
        Ok(changed)
    }
}

async fn valid_group(tx: &mut Transaction<'_, Sqlite>, group: Option<i64>) -> anyhow::Result<bool> {
    match group {
        None => Ok(true),
        Some(id) => Ok(
            sqlx::query("SELECT 1 FROM groups WHERE id = ? AND kind = 'billing'")
                .bind(id)
                .fetch_optional(&mut **tx)
                .await?
                .is_some(),
        ),
    }
}

async fn local_audit(
    tx: &mut Transaction<'_, Sqlite>,
    actor: &str,
    action: &str,
    detail: &str,
) -> anyhow::Result<()> {
    sqlx::query(
        "INSERT INTO config_audit_log (created_at, username, action, detail) VALUES (?, ?, ?, ?)",
    )
    .bind(unix_timestamp())
    .bind(actor)
    .bind(action)
    .bind(detail)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn account<'a>(
        username: &'a str,
        billing: &'a AccountBilling,
        token: Option<&'a str>,
    ) -> LocalUserCreation<'a> {
        LocalUserCreation {
            actor: "admin",
            username,
            password: random_secret(32),
            display_name: "Friend",
            billing,
            invitation_token: token,
            routing_length: 12,
        }
    }

    #[tokio::test]
    async fn local_accounts_preserve_quota_hash_passwords_and_revoke_sessions() {
        let (db, _) = Database::open(":memory:").await.unwrap();
        let group = db
            .create_billing_group("admin", "Friends", Some(1000))
            .await
            .unwrap()
            .unwrap();
        let billing = AccountBilling {
            group_id: Some(group.id),
            quota_mode: "custom".into(),
            monthly_gb: Some(5),
        };
        let new_account = account("friend", &billing, None);
        let password = new_account.password.clone();
        let replacement = random_secret(32);
        let incorrect = random_secret(32);
        let LocalCreateOutcome::Created(id) = db.create_local_user(new_account).await.unwrap()
        else {
            panic!("account not created")
        };
        let user = db.user_account(id).await.unwrap().unwrap();
        assert_eq!(user.username.as_deref(), Some("friend"));
        assert!(serde_json::to_string(&user)
            .unwrap()
            .find("password")
            .is_none());
        let hash: String = sqlx::query_scalar(
            "SELECT password_hash FROM local_user_credentials WHERE user_id = ?",
        )
        .bind(id)
        .fetch_one(&db.pool)
        .await
        .unwrap();
        assert!(hash.starts_with("$argon2"));
        assert!(!hash.contains(&password));
        assert!(argon2_matches(&password, &hash));
        let quota = db.user_billing_profile(id).await.unwrap().unwrap();
        assert_eq!(quota.group_id, Some(group.id));
        assert_eq!(quota.user_monthly_limit_bytes, Some(5 * 1024 * 1024 * 1024));
        assert_eq!(
            db.create_local_user(account("FRIEND", &billing, None))
                .await
                .unwrap(),
            LocalCreateOutcome::UsernameExists
        );
        assert!(db
            .authenticate_local_user("friend", incorrect.clone())
            .await
            .unwrap()
            .is_none());
        let session = db
            .authenticate_local_user("FRIEND", password.clone())
            .await
            .unwrap()
            .unwrap();
        assert_ne!(session.token, password);
        let session_hash: String =
            sqlx::query_scalar("SELECT token_hash FROM user_sessions WHERE user_id = ?")
                .bind(id)
                .fetch_one(&db.pool)
                .await
                .unwrap();
        assert_eq!(session_hash, hash_token(&session.token));
        assert_ne!(session_hash, hash);
        assert!(!db
            .change_local_password(id, incorrect.clone(), replacement.clone())
            .await
            .unwrap());
        assert!(db
            .change_local_password(id, password.clone(), replacement.clone())
            .await
            .unwrap());
        assert!(db
            .authenticate_user_session(&session.token)
            .await
            .unwrap()
            .is_none());
        assert!(db
            .authenticate_local_user("friend", password.clone())
            .await
            .unwrap()
            .is_none());
        assert!(db
            .authenticate_local_user("friend", replacement.clone())
            .await
            .unwrap()
            .is_some());
        db.set_user_disabled("admin", id, true).await.unwrap();
        assert!(db
            .authenticate_local_user("friend", replacement.clone())
            .await
            .unwrap()
            .is_none());
    }

    #[tokio::test]
    async fn invitation_claim_rolls_back_on_duplicate_and_uses_only_admin_billing() {
        let (db, _) = Database::open(":memory:").await.unwrap();
        let billing = AccountBilling {
            group_id: None,
            quota_mode: "custom".into(),
            monthly_gb: Some(7),
        };
        db.create_local_user(account("existing", &billing, None))
            .await
            .unwrap();
        let (_, token) = db
            .create_registration_invitation("admin", &billing, 72)
            .await
            .unwrap()
            .unwrap();
        assert!(
            !serde_json::to_string(&db.registration_invitations().await.unwrap())
                .unwrap()
                .contains(&token)
        );
        assert_eq!(
            db.create_local_user(account(
                "existing",
                &AccountBilling::default(),
                Some(&token)
            ))
            .await
            .unwrap(),
            LocalCreateOutcome::UsernameExists
        );
        assert_eq!(
            db.registration_invitations().await.unwrap()[0].status,
            "pending"
        );
        let LocalCreateOutcome::Created(id) = db
            .create_local_user(account(
                "new-friend",
                &AccountBilling {
                    quota_mode: "unlimited".into(),
                    ..Default::default()
                },
                Some(&token),
            ))
            .await
            .unwrap()
        else {
            panic!("invitation failed")
        };
        assert_eq!(
            db.user_billing_profile(id)
                .await
                .unwrap()
                .unwrap()
                .user_monthly_limit_bytes,
            Some(7 * 1024 * 1024 * 1024)
        );
        assert_eq!(
            db.create_local_user(account("another", &billing, Some(&token)))
                .await
                .unwrap(),
            LocalCreateOutcome::InvalidInvitation
        );
        let stored: String = sqlx::query_scalar("SELECT token_hash FROM registration_invitations")
            .fetch_one(&db.pool)
            .await
            .unwrap();
        assert_ne!(stored, token);
        assert_eq!(stored, hash_token(&token));
    }

    #[tokio::test]
    async fn expired_revoked_and_concurrent_invitation_claims_are_safe() {
        let (db, _) = Database::open(":memory:").await.unwrap();
        let billing = AccountBilling::default();
        let (id, token) = db
            .create_registration_invitation("admin", &billing, 1)
            .await
            .unwrap()
            .unwrap();
        db.revoke_registration_invitation("admin", id)
            .await
            .unwrap();
        assert_eq!(
            db.create_local_user(account("revoked", &billing, Some(&token)))
                .await
                .unwrap(),
            LocalCreateOutcome::InvalidInvitation
        );
        let (id, token) = db
            .create_registration_invitation("admin", &billing, 1)
            .await
            .unwrap()
            .unwrap();
        sqlx::query("UPDATE registration_invitations SET expires_at = 0 WHERE id = ?")
            .bind(id)
            .execute(&db.pool)
            .await
            .unwrap();
        assert_eq!(
            db.create_local_user(account("expired", &billing, Some(&token)))
                .await
                .unwrap(),
            LocalCreateOutcome::InvalidInvitation
        );
        let (_, token) = db
            .create_registration_invitation("admin", &billing, 1)
            .await
            .unwrap()
            .unwrap();
        let (first, second) = tokio::join!(
            db.create_local_user(account("first", &billing, Some(&token))),
            db.create_local_user(account("second", &billing, Some(&token)))
        );
        let outcomes = [first.unwrap(), second.unwrap()];
        assert_eq!(
            outcomes
                .iter()
                .filter(|v| matches!(v, LocalCreateOutcome::Created(_)))
                .count(),
            1
        );
        assert_eq!(
            outcomes
                .iter()
                .filter(|v| **v == LocalCreateOutcome::InvalidInvitation)
                .count(),
            1
        );
    }

    #[tokio::test]
    async fn password_lockout_and_invalid_group_do_not_create_accounts() {
        let (db, _) = Database::open(":memory:").await.unwrap();
        let invalid = AccountBilling {
            group_id: Some(999),
            ..Default::default()
        };
        assert_eq!(
            db.create_local_user(account("invalid", &invalid, None))
                .await
                .unwrap(),
            LocalCreateOutcome::InvalidGroup
        );
        assert!(db.list_users().await.unwrap().is_empty());
        let billing = AccountBilling::default();
        let new_account = account("locked", &billing, None);
        let password = new_account.password.clone();
        let incorrect = random_secret(32);
        db.create_local_user(new_account).await.unwrap();
        for _ in 0..5 {
            assert!(db
                .authenticate_local_user("locked", incorrect.clone())
                .await
                .unwrap()
                .is_none());
        }
        assert!(db
            .authenticate_local_user("locked", password.clone())
            .await
            .unwrap()
            .is_none());
        sqlx::query("UPDATE local_user_credentials SET locked_until = 1")
            .execute(&db.pool)
            .await
            .unwrap();
        assert!(db
            .authenticate_local_user("locked", password.clone())
            .await
            .unwrap()
            .is_some());
    }

    #[tokio::test]
    async fn migration_upgrades_v3_preserving_existing_email_accounts() {
        let (db, _) = Database::open(":memory:").await.unwrap();
        let old = db
            .create_user("admin", "existing@example.com", "Existing", 12)
            .await
            .unwrap()
            .unwrap();
        let old_session = db
            .create_user_session(old.id, "email")
            .await
            .unwrap()
            .unwrap();
        sqlx::query("DROP TABLE registration_invitations")
            .execute(&db.pool)
            .await
            .unwrap();
        sqlx::query("DROP TABLE local_user_credentials")
            .execute(&db.pool)
            .await
            .unwrap();
        sqlx::query("DROP INDEX users_username")
            .execute(&db.pool)
            .await
            .unwrap();
        sqlx::query("ALTER TABLE users DROP COLUMN username")
            .execute(&db.pool)
            .await
            .unwrap();
        sqlx::query("DELETE FROM schema_migrations WHERE version = 4")
            .execute(&db.pool)
            .await
            .unwrap();
        db.migrate().await.unwrap();
        assert_eq!(db.user_account(old.id).await.unwrap().unwrap(), old);
        assert!(db
            .authenticate_user_session(&old_session.token)
            .await
            .unwrap()
            .is_some());
        assert_eq!(
            db.health(":memory:").await.unwrap().schema_version,
            CURRENT_SCHEMA_VERSION
        );
        db.migrate().await.unwrap();
    }
}
