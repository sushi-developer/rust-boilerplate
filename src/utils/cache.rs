#![allow(dead_code)]
use deadpool_redis::{
    Connection,
    redis::{AsyncCommands, cmd},
};
use serde::{Serialize, de::DeserializeOwned};
use tracing::error;

use crate::errors::app_error::AppError;

pub async fn get<T: DeserializeOwned>(
    conn: &mut Connection,
    key: &str,
) -> Result<Option<T>, AppError> {
    match cmd("GET")
        .arg(key)
        .query_async::<Option<String>>(conn)
        .await
    {
        Ok(Some(data)) => match serde_json::from_str::<T>(&data) {
            Ok(value) => Ok(Some(value)),
            Err(err) => {
                error!("{}", err);
                Err(err.into())
            }
        },
        Ok(None) => Ok(None),
        Err(err) => {
            error!("{}", err);
            Err(err.into())
        }
    }
}

pub async fn del(conn: &mut Connection, key: &str) -> Result<(), AppError> {
    conn.del::<_, ()>(key).await.map_err(|err| {
        error!("{}", err);
        AppError::from(err)
    })?;
    Ok(())
}

pub async fn set<T: Serialize>(
    conn: &mut Connection,
    key: &str,
    value: &T,
    ttl_secs: Option<u64>,
) -> Result<(), AppError> {
    let data = serde_json::to_string(value).map_err(|err| {
        error!("{}", err);
        err
    })?;

    let mut c = cmd("SET");
    c.arg(key).arg(data);
    if let Some(ttl) = ttl_secs {
        c.arg("EX").arg(ttl.to_string());
    }
    c.query_async::<()>(conn).await.map_err(|err| {
        error!("{}", err);
        err
    })?;

    Ok(())
}

pub async fn keys(conn: &mut Connection, pattern: &str) -> Result<Vec<String>, AppError> {
    let mut cursor: u64 = 0;
    let mut found = Vec::new();

    loop {
        let (next_cursor, batch): (u64, Vec<String>) = cmd("SCAN")
            .arg(cursor)
            .arg("MATCH")
            .arg(pattern)
            .arg("COUNT")
            .arg(100)
            .query_async(conn)
            .await
            .map_err(|err| {
                error!("{}", err);
                err
            })?;

        found.extend(batch);
        if next_cursor == 0 {
            break;
        }
        cursor = next_cursor;
    }

    Ok(found)
}

pub async fn values<T: DeserializeOwned>(
    conn: &mut Connection,
    keys: &[String],
) -> Result<Vec<Option<T>>, AppError> {
    if keys.is_empty() {
        return Ok(vec![]);
    }

    let raw: Vec<Option<String>> = conn.mget(keys).await.map_err(|err| {
        error!("{}", err);
        err
    })?;

    raw.into_iter()
        .map(|opt| match opt {
            Some(data) => serde_json::from_str::<T>(&data).map(Some).map_err(|err| {
                error!("{}", err);
                AppError::from(err)
            }),
            None => Ok(None),
        })
        .collect()
}
