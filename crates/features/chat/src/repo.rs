//! 会话持久化（`D6`：业务 SQL 是**语义**，住在这里；连接与迁移机制由 `agent-store` 提供）。
//!
//! 表结构（`Migrator` v1）：`sessions`
//!
//! | 列 | 用途 |
//! |----|------|
//! | `id` / `title` / `model` / `created_at` / `updated_at` | **元数据**：会话列表只读这几列，不必反序列化消息体 |
//! | `chat` | [`Chat`] 的 JSON，**原样落盘**（`Chat` 全字段可序列化，见 `crate::chat`） |
//!
//! 两条约定：
//! 1. **时钟不进本模块**：`now_ms` 由调用方传入，测试可确定性地断言时间列。
//! 2. **写操作走 upsert**：`save` 按 `id` 覆盖，`created_at` 首次写入后不再变（列表排序靠它）。

use agent_store::rusqlite::{OptionalExtension, params};
use agent_store::{Migrator, Store, StoreError};
use serde::{Deserialize, Serialize};

use crate::chat::Chat;

/// 会话元数据（列表用，不含消息体）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionMeta {
    pub id: String,
    pub title: String,
    pub model: String,
    /// Unix 毫秒。
    pub created_at: i64,
    /// Unix 毫秒；列表按它倒序。
    pub updated_at: i64,
}

/// 持久化错误：存储层错误 + 会话 JSON 的（反）序列化错误。
///
/// JSON 是**业务数据**的编码，所以它在这一层，而不是 `kernel/store`（`D6` §1）。
#[derive(Debug, thiserror::Error)]
pub enum RepoError {
    #[error(transparent)]
    Store(#[from] StoreError),
    #[error("会话 JSON 解析失败: {0}")]
    Json(#[from] serde_json::Error),
}

/// 表结构 v1（幂等：`Migrator` 用 `PRAGMA user_version` 记账，只跑一次）。
const SCHEMA_V1: &str = "
CREATE TABLE sessions (
    id         TEXT    PRIMARY KEY,
    title      TEXT    NOT NULL DEFAULT '',
    model      TEXT    NOT NULL DEFAULT '',
    chat       TEXT    NOT NULL,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);
CREATE INDEX idx_sessions_updated ON sessions (updated_at DESC, id ASC);
";

/// 本功能的全部迁移，交给 `Store::migrate` 执行。
pub fn migrator() -> Migrator {
    let mut m = Migrator::new();
    m.add(1, &[SCHEMA_V1]);
    m
}

/// 会话仓储：连接来自 `agent-store`，表结构与 SQL 归本模块。
#[derive(Clone)]
pub struct SessionRepo {
    store: Store,
}

impl SessionRepo {
    /// 迁移到最新表结构并返回仓储（重复调用幂等）。
    pub fn open(store: Store) -> Result<Self, RepoError> {
        store.migrate(&migrator())?;
        Ok(Self { store })
    }

    pub fn store(&self) -> &Store {
        &self.store
    }

    /// 新建或覆盖一个会话（按 `id` upsert）；`created_at` 首写后不变。
    pub fn save(&self, id: &str, title: &str, chat: &Chat, now_ms: i64) -> Result<(), RepoError> {
        let chat_json = serde_json::to_string(chat)?;
        self.store.with_conn(|conn| {
            conn.execute(
                "INSERT INTO sessions (id, title, model, chat, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?5)
                 ON CONFLICT(id) DO UPDATE SET
                     title      = excluded.title,
                     model      = excluded.model,
                     chat       = excluded.chat,
                     updated_at = excluded.updated_at",
                params![id, title, chat.model, chat_json, now_ms],
            )?;
            Ok(())
        })?;
        Ok(())
    }

    /// 取回一个会话的完整状态；不存在返回 `None`。
    pub fn load(&self, id: &str) -> Result<Option<Chat>, RepoError> {
        let json = self.store.with_conn(|conn| {
            Ok(conn
                .query_row(
                    "SELECT chat FROM sessions WHERE id = ?1",
                    params![id],
                    |row| row.get::<_, String>(0),
                )
                .optional()?)
        })?;
        match json {
            Some(json) => Ok(Some(serde_json::from_str(&json)?)),
            None => Ok(None),
        }
    }

    /// 会话列表，按最近更新倒序。
    pub fn list(&self) -> Result<Vec<SessionMeta>, RepoError> {
        Ok(self.store.with_conn(|conn| {
            let mut stmt = conn.prepare(
                "SELECT id, title, model, created_at, updated_at
                 FROM sessions ORDER BY updated_at DESC, id ASC",
            )?;
            let rows = stmt.query_map([], |row| {
                Ok(SessionMeta {
                    id: row.get(0)?,
                    title: row.get(1)?,
                    model: row.get(2)?,
                    created_at: row.get(3)?,
                    updated_at: row.get(4)?,
                })
            })?;
            let mut metas = Vec::new();
            for row in rows {
                metas.push(row?);
            }
            Ok(metas)
        })?)
    }

    /// 改名；返回是否命中了一个已存在的会话。
    pub fn rename(&self, id: &str, title: &str, now_ms: i64) -> Result<bool, RepoError> {
        Ok(self.store.with_conn(|conn| {
            let n = conn.execute(
                "UPDATE sessions SET title = ?2, updated_at = ?3 WHERE id = ?1",
                params![id, title, now_ms],
            )?;
            Ok(n > 0)
        })?)
    }

    /// 删除；返回是否命中了一个已存在的会话。
    pub fn delete(&self, id: &str) -> Result<bool, RepoError> {
        Ok(self.store.with_conn(|conn| {
            let n = conn.execute("DELETE FROM sessions WHERE id = ?1", params![id])?;
            Ok(n > 0)
        })?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn repo() -> SessionRepo {
        SessionRepo::open(Store::open_in_memory().unwrap()).unwrap()
    }

    fn chat_with(model: &str, texts: &[&str]) -> Chat {
        let mut chat = Chat::new(model);
        for text in texts {
            chat.push_user(*text);
        }
        chat
    }

    /// `Chat` 未派生 `PartialEq`，按 JSON 逐字段比。
    fn same_chat(a: &Chat, b: &Chat) -> bool {
        serde_json::to_value(a).unwrap() == serde_json::to_value(b).unwrap()
    }

    #[test]
    fn save_then_load_roundtrips_chat() {
        let repo = repo();
        let chat = chat_with("m1", &["你好", "第二条"]);
        repo.save("s1", "会话一", &chat, 1_000).unwrap();

        let loaded = repo.load("s1").unwrap().expect("已存的会话必须能取回");
        assert!(same_chat(&loaded, &chat));
        assert_eq!(loaded.model, "m1");
        assert_eq!(loaded.messages.len(), 2);
    }

    #[test]
    fn load_missing_returns_none() {
        assert!(repo().load("nope").unwrap().is_none());
    }

    #[test]
    fn save_is_upsert_and_keeps_created_at() {
        let repo = repo();
        repo.save("s1", "旧标题", &chat_with("m", &["a"]), 100)
            .unwrap();
        repo.save("s1", "新标题", &chat_with("m", &["a", "b"]), 200)
            .unwrap();

        let metas = repo.list().unwrap();
        assert_eq!(metas.len(), 1, "upsert 不应产生第二行");
        assert_eq!(metas[0].title, "新标题");
        assert_eq!(metas[0].created_at, 100, "created_at 首写后不变");
        assert_eq!(metas[0].updated_at, 200);
        assert_eq!(repo.load("s1").unwrap().unwrap().messages.len(), 2);
    }

    #[test]
    fn list_orders_by_updated_at_desc() {
        let repo = repo();
        repo.save("a", "A", &chat_with("m", &["a"]), 100).unwrap();
        repo.save("b", "B", &chat_with("m", &["b"]), 300).unwrap();
        repo.save("c", "C", &chat_with("m", &["c"]), 200).unwrap();

        let ids: Vec<String> = repo.list().unwrap().into_iter().map(|m| m.id).collect();
        assert_eq!(ids, vec!["b", "c", "a"]);
    }

    #[test]
    fn rename_and_delete_report_hit() {
        let repo = repo();
        repo.save("s1", "标题", &chat_with("m", &["a"]), 10)
            .unwrap();

        assert!(repo.rename("s1", "改名", 20).unwrap());
        assert_eq!(repo.list().unwrap()[0].title, "改名");
        assert_eq!(repo.list().unwrap()[0].updated_at, 20);
        assert!(
            !repo.rename("missing", "x", 20).unwrap(),
            "未命中应返回 false"
        );

        assert!(repo.delete("s1").unwrap());
        assert!(!repo.delete("s1").unwrap(), "重复删除应返回 false");
        assert!(repo.load("s1").unwrap().is_none());
    }

    #[test]
    fn open_is_idempotent() {
        let store = Store::open_in_memory().unwrap();
        SessionRepo::open(store.clone()).unwrap();
        SessionRepo::open(store).unwrap();
    }
}
