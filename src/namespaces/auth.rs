//! Auth namespace - PIN lock/unlock status.

use nine_s_core::prelude::*;
use serde_json::{json, Value};
use std::sync::Arc;

const STATUS: &str = "/status";
const UNLOCK: &str = "/unlock";
const LOCK: &str = "/lock";

const STATUS_TYPE: &str = "system/auth/status@v1";
const UNLOCK_TYPE: &str = "system/auth/unlock@v1";
const LOCK_TYPE: &str = "system/auth/lock@v1";

#[derive(Clone, Debug, Default)]
pub struct AuthStatus {
    pub locked: bool,
    pub initialized: bool,
}

type StatusFn = dyn Fn() -> NineSResult<AuthStatus> + Send + Sync;
type UnlockFn = dyn Fn(&str) -> NineSResult<bool> + Send + Sync;
type LockFn = dyn Fn() -> NineSResult<bool> + Send + Sync;

#[derive(Clone)]
pub struct AuthController {
    status: Arc<StatusFn>,
    unlock: Arc<UnlockFn>,
    lock: Arc<LockFn>,
}

impl AuthController {
    pub fn new(
        status: Arc<StatusFn>,
        unlock: Arc<UnlockFn>,
        lock: Arc<LockFn>,
    ) -> Self {
        Self { status, unlock, lock }
    }

    pub fn status(&self) -> NineSResult<AuthStatus> { (self.status)() }
    pub fn unlock(&self, pin: &str) -> NineSResult<bool> { (self.unlock)(pin) }
    pub fn lock(&self) -> NineSResult<bool> { (self.lock)() }
}

pub struct AuthNamespace {
    controller: AuthController,
}

impl AuthNamespace {
    pub fn new(controller: AuthController) -> Self { Self { controller } }

    fn read_status(&self) -> NineSResult<Scroll> {
        let status = self.controller.status()?;
        Ok(Scroll::new("/system/auth/status", json!({
            "locked": status.locked,
            "initialized": status.initialized,
        })).set_type(STATUS_TYPE))
    }

    fn write_unlock(&self, data: Value) -> NineSResult<Scroll> {
        let pin = data["pin"]
            .as_str()
            .ok_or_else(|| NineSError::Other("no 'pin'".into()))?;
        let success = self.controller.unlock(pin)?;
        Ok(Scroll::new("/system/auth/unlock", json!({"success": success}))
            .set_type(UNLOCK_TYPE))
    }

    fn write_lock(&self) -> NineSResult<Scroll> {
        let success = self.controller.lock()?;
        Ok(Scroll::new("/system/auth/lock", json!({"success": success}))
            .set_type(LOCK_TYPE))
    }
}

impl Namespace for AuthNamespace {
    fn read(&self, path: &str) -> NineSResult<Option<Scroll>> {
        Ok(Some(match path {
            STATUS | "" | "/" => self.read_status()?,
            _ => return Ok(None),
        }))
    }

    fn write(&self, path: &str, data: Value) -> NineSResult<Scroll> {
        match path {
            UNLOCK => self.write_unlock(data),
            LOCK => self.write_lock(),
            _ => Err(NineSError::Other(format!("unknown: {}", path))),
        }
    }

    fn list(&self, _: &str) -> NineSResult<Vec<String>> {
        Ok(vec![STATUS.into(), UNLOCK.into(), LOCK.into()])
    }
}
