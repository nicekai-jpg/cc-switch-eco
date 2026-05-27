//! Ecosystems 数据访问对象
//!
//! 管理生态（Ecosystem）的 CRUD 操作。

use super::super::{lock_conn, Database};
use crate::error::AppError;
use rusqlite::params;
use serde::{Deserialize, Serialize};

/// 生态对象
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Ecosystem {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
    pub is_current: bool,
    #[serde(default)]
    pub created_at: i64,
}

impl Database {
    /// 获取所有生态
    pub fn get_all_ecosystems(&self) -> Result<Vec<Ecosystem>, AppError> {
        let conn = lock_conn!(self.conn);
        let mut stmt = conn
            .prepare("SELECT id, name, description, is_current, created_at FROM ecosystems ORDER BY name")
            .map_err(|e| AppError::Database(e.to_string()))?;

        let rows = stmt
            .query_map([], |row| {
                Ok(Ecosystem {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    description: row.get(2)?,
                    is_current: row.get::<_, i32>(3)? != 0,
                    created_at: row.get(4)?,
                })
            })
            .map_err(|e| AppError::Database(e.to_string()))?;

        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// 获取当前激活的生态
    pub fn get_current_ecosystem(&self) -> Result<Option<Ecosystem>, AppError> {
        let conn = lock_conn!(self.conn);
        let result = conn.query_row(
            "SELECT id, name, description, is_current, created_at FROM ecosystems WHERE is_current = 1",
            [],
            |row| {
                Ok(Ecosystem {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    description: row.get(2)?,
                    is_current: true,
                    created_at: row.get(4)?,
                })
            },
        );

        match result {
            Ok(eco) => Ok(Some(eco)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(AppError::Database(e.to_string())),
        }
    }

    /// 保存生态（INSERT OR REPLACE）
    pub fn save_ecosystem(&self, eco: &Ecosystem) -> Result<(), AppError> {
        let conn = lock_conn!(self.conn);
        conn.execute(
            "INSERT OR REPLACE INTO ecosystems (id, name, description, is_current, created_at) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![eco.id, eco.name, eco.description, eco.is_current as i32, eco.created_at],
        )
        .map_err(|e| AppError::Database(format!("保存生态失败: {e}")))?;
        Ok(())
    }

    /// 删除生态
    pub fn delete_ecosystem(&self, id: &str) -> Result<bool, AppError> {
        let conn = lock_conn!(self.conn);
        let rows = conn
            .execute("DELETE FROM ecosystems WHERE id = ?1", [id])
            .map_err(|e| AppError::Database(format!("删除生态失败: {e}")))?;
        Ok(rows > 0)
    }

    /// 设置当前生态（事务：清除所有 is_current，再设置指定生态）
    pub fn set_current_ecosystem(&self, id: &str) -> Result<(), AppError> {
        let mut conn = lock_conn!(self.conn);
        let tx = conn
            .transaction()
            .map_err(|e| AppError::Database(e.to_string()))?;

        tx.execute("UPDATE ecosystems SET is_current = 0", [])
            .map_err(|e| AppError::Database(e.to_string()))?;
        tx.execute(
            "UPDATE ecosystems SET is_current = 1 WHERE id = ?1",
            [id],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;

        tx.commit()
            .map_err(|e| AppError::Database(e.to_string()))?;
        Ok(())
    }

    /// 清除所有生态的 is_current 标记
    pub fn clear_current_ecosystem(&self) -> Result<(), AppError> {
        let conn = lock_conn!(self.conn);
        conn.execute("UPDATE ecosystems SET is_current = 0", [])
            .map_err(|e| AppError::Database(e.to_string()))?;
        Ok(())
    }

    /// 检查生态是否存在
    pub fn ecosystem_exists(&self, id: &str) -> Result<bool, AppError> {
        let conn = lock_conn!(self.conn);
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM ecosystems WHERE id = ?1",
                [id],
                |row| row.get(0),
            )
            .map_err(|e| AppError::Database(e.to_string()))?;
        Ok(count > 0)
    }
}
