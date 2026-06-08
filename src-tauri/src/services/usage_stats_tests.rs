use super::*;

fn local_ts(year: i32, month: u32, day: u32, hour: u32, minute: u32, second: u32) -> i64 {
    match Local.with_ymd_and_hms(year, month, day, hour, minute, second) {
        chrono::LocalResult::Single(dt) => dt.timestamp(),
        chrono::LocalResult::Ambiguous(earliest, _) => earliest.timestamp(),
        chrono::LocalResult::None => panic!("valid local datetime"),
    }
}

#[allow(clippy::too_many_arguments)]
fn insert_usage_log(
    conn: &Connection,
    request_id: &str,
    app_type: &str,
    provider_id: &str,
    model: &str,
    data_source: &str,
    created_at: i64,
    input_tokens: i64,
    output_tokens: i64,
    cache_read_tokens: i64,
    cache_creation_tokens: i64,
    status_code: i64,
    total_cost_usd: &str,
) -> Result<(), AppError> {
    conn.execute(
        "INSERT INTO proxy_request_logs (
            request_id, provider_id, app_type, model, request_model,
            input_tokens, output_tokens, cache_read_tokens, cache_creation_tokens,
            input_cost_usd, output_cost_usd, cache_read_cost_usd, cache_creation_cost_usd,
            total_cost_usd, latency_ms, status_code, created_at, data_source
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, '0', '0', '0', '0', ?, 100, ?, ?, ?)",
        params![
            request_id,
            provider_id,
            app_type,
            model,
            model,
            input_tokens,
            output_tokens,
            cache_read_tokens,
            cache_creation_tokens,
            total_cost_usd,
            status_code,
            created_at,
            data_source
        ],
    )?;
    Ok(())
}

fn create_legacy_nullable_logs_table(conn: &Connection) -> Result<(), AppError> {
    conn.execute(
        "CREATE TABLE proxy_request_logs (
            request_id TEXT PRIMARY KEY,
            app_type TEXT NOT NULL,
            model TEXT NOT NULL,
            input_tokens INTEGER NOT NULL,
            output_tokens INTEGER NOT NULL,
            cache_read_tokens INTEGER NOT NULL,
            cache_creation_tokens INTEGER NOT NULL,
            status_code INTEGER NOT NULL,
            created_at INTEGER NOT NULL,
            data_source TEXT
        )",
        [],
    )?;
    Ok(())
}

#[test]
fn test_effective_filter_keeps_legacy_null_data_source_proxy_rows() -> Result<(), AppError> {
    let conn = Connection::open_in_memory()?;
    create_legacy_nullable_logs_table(&conn)?;
    conn.execute(
        "INSERT INTO proxy_request_logs (
            request_id, app_type, model, input_tokens, output_tokens,
            cache_read_tokens, cache_creation_tokens, status_code, created_at, data_source
        ) VALUES ('legacy-proxy', 'codex', 'gpt-5.5', 10, 2, 1, 0, 200, 1000, NULL)",
        [],
    )?;

    let filter = effective_usage_log_filter("l");
    let sql = format!("SELECT COUNT(*) FROM proxy_request_logs l WHERE {filter}");
    let count: i64 = conn.query_row(&sql, [], |row| row.get(0))?;
    assert_eq!(count, 1);

    Ok(())
}

#[test]
fn test_matching_proxy_log_treats_legacy_null_data_source_as_proxy() -> Result<(), AppError> {
    let conn = Connection::open_in_memory()?;
    create_legacy_nullable_logs_table(&conn)?;
    conn.execute(
        "INSERT INTO proxy_request_logs (
            request_id, app_type, model, input_tokens, output_tokens,
            cache_read_tokens, cache_creation_tokens, status_code, created_at, data_source
        ) VALUES ('legacy-proxy', 'codex', 'gpt-5.5', 10, 2, 1, 0, 200, 1000, NULL)",
        [],
    )?;

    let key = DedupKey {
        app_type: "codex",
        model: "gpt-5.5",
        input_tokens: 10,
        output_tokens: 2,
        cache_read_tokens: 1,
        cache_creation_tokens: 0,
        created_at: 1000,
    };
    assert!(has_matching_proxy_usage_log(&conn, &key)?);

    Ok(())
}

#[test]
fn test_backfill_missing_usage_costs_uses_new_gpt_5_5_pricing() -> Result<(), AppError> {
    let db = Database::memory()?;

    {
        let conn = lock_conn!(db.conn);
        insert_usage_log(
            &conn,
            "codex-gpt-5-5-zero-cost",
            "codex",
            "_codex_session",
            "gpt-5.5",
            "codex_session",
            1000,
            1_000_000,
            1_000_000,
            0,
            0,
            200,
            "0",
        )?;
    }

    assert_eq!(db.backfill_missing_usage_costs()?, 1);

    let conn = lock_conn!(db.conn);
    let (input_cost, output_cost, total_cost): (String, String, String) = conn.query_row(
        "SELECT input_cost_usd, output_cost_usd, total_cost_usd
         FROM proxy_request_logs WHERE request_id = 'codex-gpt-5-5-zero-cost'",
        [],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
    )?;
    assert_eq!(input_cost, "5.000000");
    assert_eq!(output_cost, "30.000000");
    assert_eq!(total_cost, "35.000000");

    Ok(())
}

#[test]
fn test_backfill_missing_usage_costs_uses_stored_multiplier() -> Result<(), AppError> {
    let db = Database::memory()?;

    {
        let conn = lock_conn!(db.conn);
        insert_usage_log(
            &conn,
            "codex-gpt-5-5-multiplier",
            "codex",
            "_codex_session",
            "gpt-5.5",
            "codex_session",
            1000,
            1_000_000,
            0,
            0,
            0,
            200,
            "0",
        )?;
        conn.execute(
            "UPDATE proxy_request_logs
             SET cost_multiplier = '1.5'
             WHERE request_id = 'codex-gpt-5-5-multiplier'",
            [],
        )?;
    }

    assert_eq!(db.backfill_missing_usage_costs()?, 1);

    let conn = lock_conn!(db.conn);
    let (input_cost, total_cost): (String, String) = conn.query_row(
        "SELECT input_cost_usd, total_cost_usd
         FROM proxy_request_logs WHERE request_id = 'codex-gpt-5-5-multiplier'",
        [],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )?;
    assert_eq!(input_cost, "5.000000");
    assert_eq!(total_cost, "7.500000");

    Ok(())
}

#[test]
fn test_backfill_missing_usage_costs_falls_back_to_request_model() -> Result<(), AppError> {
    let db = Database::memory()?;

    {
        let conn = lock_conn!(db.conn);
        conn.execute(
            "INSERT INTO proxy_request_logs (
                request_id, provider_id, app_type, model, request_model,
                input_tokens, output_tokens, cache_read_tokens, cache_creation_tokens,
                input_cost_usd, output_cost_usd, cache_read_cost_usd, cache_creation_cost_usd,
                total_cost_usd, latency_ms, status_code, created_at, data_source
            ) VALUES (
                'codex-request-model-fallback', '_codex_session', 'codex', 'unknown', 'gpt-5.5',
                1000000, 0, 0, 0,
                '0', '0', '0', '0',
                '0', 100, 200, 1000, 'codex_session'
            )",
            [],
        )?;
    }

    assert_eq!(db.backfill_missing_usage_costs()?, 1);

    let conn = lock_conn!(db.conn);
    let total_cost: String = conn.query_row(
        "SELECT total_cost_usd
         FROM proxy_request_logs WHERE request_id = 'codex-request-model-fallback'",
        [],
        |row| row.get(0),
    )?;
    assert_eq!(total_cost, "5.000000");

    Ok(())
}

#[test]
fn test_backfill_missing_usage_costs_keeps_claude_fresh_input() -> Result<(), AppError> {
    let db = Database::memory()?;

    {
        let conn = lock_conn!(db.conn);
        insert_usage_log(
            &conn,
            "claude-cache-fresh-input",
            "claude",
            "_session",
            "claude-haiku-4-5",
            "session_log",
            1000,
            100,
            0,
            200,
            0,
            200,
            "0",
        )?;
    }

    assert_eq!(db.backfill_missing_usage_costs()?, 1);

    let conn = lock_conn!(db.conn);
    let (input_cost, cache_read_cost, total_cost): (String, String, String) = conn.query_row(
        "SELECT input_cost_usd, cache_read_cost_usd, total_cost_usd
         FROM proxy_request_logs WHERE request_id = 'claude-cache-fresh-input'",
        [],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
    )?;
    assert_eq!(input_cost, "0.000100");
    assert_eq!(cache_read_cost, "0.000020");
    assert_eq!(total_cost, "0.000120");

    Ok(())
}

#[test]
fn test_get_usage_summary() -> Result<(), AppError> {
    let db = Database::memory()?;

    // 插入测试数据
    {
        let conn = lock_conn!(db.conn);
        conn.execute(
            "INSERT INTO proxy_request_logs (
                request_id, provider_id, app_type, model,
                input_tokens, output_tokens, total_cost_usd,
                latency_ms, status_code, created_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            params!["req1", "p1", "claude", "claude-3", 100, 50, "0.01", 100, 200, 1000],
        )?;
        conn.execute(
            "INSERT INTO proxy_request_logs (
                request_id, provider_id, app_type, model,
                input_tokens, output_tokens, total_cost_usd,
                latency_ms, status_code, created_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            params!["req2", "p1", "claude", "claude-3", 200, 100, "0.02", 150, 200, 2000],
        )?;
    }

    let summary = db.get_usage_summary(None, None, None)?;
    assert_eq!(summary.total_requests, 2);
    assert_eq!(summary.success_rate, 100.0);

    Ok(())
}

#[test]
fn test_get_usage_summary_excludes_partial_rollup_boundary_days() -> Result<(), AppError> {
    let db = Database::memory()?;
    let start = local_ts(2024, 1, 1, 12, 0, 0);
    let end = local_ts(2024, 1, 3, 12, 0, 0);

    {
        let conn = lock_conn!(db.conn);
        conn.execute(
            "INSERT INTO usage_daily_rollups (
                date, app_type, provider_id, model,
                request_count, success_count, input_tokens, output_tokens,
                cache_read_tokens, cache_creation_tokens, total_cost_usd, avg_latency_ms
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            params![
                "2024-01-01",
                "claude",
                "p1",
                "claude-3",
                10,
                10,
                1000,
                500,
                0,
                0,
                "1.00",
                100
            ],
        )?;
        conn.execute(
            "INSERT INTO usage_daily_rollups (
                date, app_type, provider_id, model,
                request_count, success_count, input_tokens, output_tokens,
                cache_read_tokens, cache_creation_tokens, total_cost_usd, avg_latency_ms
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            params![
                "2024-01-02",
                "claude",
                "p1",
                "claude-3",
                20,
                19,
                2000,
                1000,
                0,
                0,
                "2.00",
                120
            ],
        )?;
        conn.execute(
            "INSERT INTO usage_daily_rollups (
                date, app_type, provider_id, model,
                request_count, success_count, input_tokens, output_tokens,
                cache_read_tokens, cache_creation_tokens, total_cost_usd, avg_latency_ms
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            params![
                "2024-01-03",
                "claude",
                "p1",
                "claude-3",
                30,
                29,
                3000,
                1500,
                0,
                0,
                "3.00",
                140
            ],
        )?;
    }

    let summary = db.get_usage_summary(Some(start), Some(end), Some("claude"))?;
    assert_eq!(summary.total_requests, 20);
    assert_eq!(summary.total_input_tokens, 2000);
    assert_eq!(summary.total_output_tokens, 1000);

    Ok(())
}

#[test]
fn test_get_usage_summary_includes_end_day_rollup_for_minute_precision_end_time(
) -> Result<(), AppError> {
    let db = Database::memory()?;
    let start = local_ts(2024, 1, 1, 0, 0, 0);
    let end = local_ts(2024, 1, 2, 23, 59, 0);

    {
        let conn = lock_conn!(db.conn);
        conn.execute(
            "INSERT INTO usage_daily_rollups (
                date, app_type, provider_id, model,
                request_count, success_count, input_tokens, output_tokens,
                cache_read_tokens, cache_creation_tokens, total_cost_usd, avg_latency_ms
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            params![
                "2024-01-01",
                "claude",
                "p1",
                "claude-3",
                10,
                10,
                1000,
                500,
                0,
                0,
                "1.00",
                100
            ],
        )?;
        conn.execute(
            "INSERT INTO usage_daily_rollups (
                date, app_type, provider_id, model,
                request_count, success_count, input_tokens, output_tokens,
                cache_read_tokens, cache_creation_tokens, total_cost_usd, avg_latency_ms
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            params![
                "2024-01-02",
                "claude",
                "p1",
                "claude-3",
                20,
                19,
                2000,
                1000,
                0,
                0,
                "2.00",
                120
            ],
        )?;
    }

    let summary = db.get_usage_summary(Some(start), Some(end), Some("claude"))?;
    assert_eq!(summary.total_requests, 30);
    assert_eq!(summary.total_input_tokens, 3000);
    assert_eq!(summary.total_output_tokens, 1500);

    Ok(())
}

#[test]
fn test_effective_usage_dedup_prefers_proxy_for_session_sources() -> Result<(), AppError> {
    let db = Database::memory()?;

    {
        let conn = lock_conn!(db.conn);
        insert_usage_log(
            &conn,
            "codex-proxy",
            "codex",
            "openai",
            "GPT-5.4",
            "proxy",
            10_000,
            100,
            20,
            10,
            7,
            200,
            "0.10",
        )?;
        insert_usage_log(
            &conn,
            "codex-session-dup",
            "codex",
            "_codex_session",
            "gpt-5.4",
            "codex_session",
            10_060,
            100,
            20,
            10,
            0,
            200,
            "0.10",
        )?;
        insert_usage_log(
            &conn,
            "claude-proxy",
            "claude",
            "openai-compatible",
            "claude-sonnet-4-5",
            "proxy",
            25_000,
            300,
            60,
            20,
            5,
            200,
            "0.30",
        )?;
        insert_usage_log(
            &conn,
            "claude-session-dup",
            "claude",
            "_session",
            "claude-sonnet-4-5",
            "session_log",
            25_060,
            300,
            60,
            20,
            5,
            200,
            "0.30",
        )?;
        insert_usage_log(
            &conn,
            "gemini-proxy",
            "gemini",
            "google",
            "gemini-2.5-pro",
            "proxy",
            20_000,
            200,
            40,
            30,
            0,
            200,
            "0.20",
        )?;
        insert_usage_log(
            &conn,
            "gemini-session-dup",
            "gemini",
            "_gemini_session",
            "gemini-2.5-pro",
            "gemini_session",
            20_060,
            200,
            40,
            30,
            0,
            200,
            "0.20",
        )?;
        insert_usage_log(
            &conn,
            "codex-session-only",
            "codex",
            "_codex_session",
            "gpt-5.4",
            "codex_session",
            30_000,
            50,
            5,
            0,
            0,
            200,
            "0.02",
        )?;
    }

    let summary = db.get_usage_summary(None, None, None)?;
    assert_eq!(summary.total_requests, 4);
    // codex-proxy contributes 100-10=90; gemini-proxy contributes 200-30=170
    // (both cache-inclusive providers). claude-proxy=300, codex-session-only=50.
    // 90 + 170 + 300 + 50 = 610.
    assert_eq!(summary.total_input_tokens, 610);
    assert_eq!(summary.total_output_tokens, 125);
    assert_eq!(summary.total_cache_read_tokens, 60);
    assert_eq!(summary.total_cache_creation_tokens, 12);
    // real_total = fresh_input(610) + output(125) + cache_create(12) + cache_read(60) = 807
    assert_eq!(summary.real_total_tokens, 807);
    // hit_rate = 60 / (610 + 12 + 60) = 60 / 682
    let expected_hit_rate = 60.0_f64 / 682.0_f64;
    assert!((summary.cache_hit_rate - expected_hit_rate).abs() < 1e-9);

    let trends = db.get_daily_trends(Some(0), Some(40_000), None)?;
    assert_eq!(trends.iter().map(|stat| stat.request_count).sum::<u64>(), 4);

    let provider_stats = db.get_provider_stats(None, None, None)?;
    assert_eq!(
        provider_stats
            .iter()
            .map(|stat| stat.request_count)
            .sum::<u64>(),
        4
    );
    assert!(provider_stats
        .iter()
        .any(|stat| stat.provider_id == "_codex_session" && stat.request_count == 1));
    assert!(!provider_stats
        .iter()
        .any(|stat| stat.provider_id == "_gemini_session"));
    assert!(!provider_stats
        .iter()
        .any(|stat| stat.provider_id == "_session"));

    let model_stats = db.get_model_stats(None, None, None)?;
    assert_eq!(
        model_stats
            .iter()
            .map(|stat| stat.request_count)
            .sum::<u64>(),
        4
    );

    let logs = db.get_request_logs(&LogFilters::default(), 0, 10)?;
    let request_ids: Vec<&str> = logs
        .data
        .iter()
        .map(|log| log.request_id.as_str())
        .collect();
    assert_eq!(logs.total, 4);
    assert!(request_ids.contains(&"codex-proxy"));
    assert!(request_ids.contains(&"claude-proxy"));
    assert!(request_ids.contains(&"gemini-proxy"));
    assert!(request_ids.contains(&"codex-session-only"));
    assert!(!request_ids.contains(&"codex-session-dup"));
    assert!(!request_ids.contains(&"claude-session-dup"));
    assert!(!request_ids.contains(&"gemini-session-dup"));

    let breakdown = crate::services::session_usage::get_data_source_breakdown(&db)?;
    let proxy_count = breakdown
        .iter()
        .find(|item| item.data_source == "proxy")
        .map(|item| item.request_count);
    let codex_session_count = breakdown
        .iter()
        .find(|item| item.data_source == "codex_session")
        .map(|item| item.request_count);
    let gemini_session_count = breakdown
        .iter()
        .find(|item| item.data_source == "gemini_session")
        .map(|item| item.request_count);
    let session_log_count = breakdown
        .iter()
        .find(|item| item.data_source == "session_log")
        .map(|item| item.request_count);
    assert_eq!(proxy_count, Some(3));
    assert_eq!(codex_session_count, Some(1));
    assert_eq!(gemini_session_count, None);
    assert_eq!(session_log_count, None);

    Ok(())
}

#[test]
fn test_effective_usage_dedup_keeps_non_matching_session_rows() -> Result<(), AppError> {
    let db = Database::memory()?;

    {
        let conn = lock_conn!(db.conn);
        insert_usage_log(
            &conn,
            "proxy-base",
            "codex",
            "openai",
            "gpt-5.4",
            "proxy",
            10_000,
            100,
            20,
            10,
            0,
            200,
            "0.10",
        )?;
        insert_usage_log(
            &conn,
            "session-outside-window",
            "codex",
            "_codex_session",
            "gpt-5.4",
            "codex_session",
            10_601,
            100,
            20,
            10,
            0,
            200,
            "0.10",
        )?;
        insert_usage_log(
            &conn,
            "session-token-mismatch",
            "codex",
            "_codex_session",
            "gpt-5.4",
            "codex_session",
            10_060,
            101,
            20,
            10,
            0,
            200,
            "0.10",
        )?;
        insert_usage_log(
            &conn,
            "session-app-mismatch",
            "gemini",
            "_gemini_session",
            "gpt-5.4",
            "gemini_session",
            10_060,
            100,
            20,
            10,
            0,
            200,
            "0.10",
        )?;
        insert_usage_log(
            &conn,
            "session-model-mismatch",
            "codex",
            "_codex_session",
            "different-model",
            "codex_session",
            10_060,
            100,
            20,
            10,
            0,
            200,
            "0.10",
        )?;
        insert_usage_log(
            &conn,
            "proxy-error",
            "codex",
            "openai",
            "gpt-5.4",
            "proxy",
            20_000,
            300,
            60,
            0,
            0,
            500,
            "0.00",
        )?;
        insert_usage_log(
            &conn,
            "session-matches-error-proxy",
            "codex",
            "_codex_session",
            "gpt-5.4",
            "codex_session",
            20_060,
            300,
            60,
            0,
            0,
            200,
            "0.30",
        )?;
        insert_usage_log(
            &conn,
            "claude-proxy-cache-creation",
            "claude",
            "anthropic",
            "claude-sonnet-4-5",
            "proxy",
            30_000,
            100,
            20,
            10,
            5,
            200,
            "0.10",
        )?;
        insert_usage_log(
            &conn,
            "claude-session-cache-creation-mismatch",
            "claude",
            "_session",
            "claude-sonnet-4-5",
            "session_log",
            30_060,
            100,
            20,
            10,
            0,
            200,
            "0.10",
        )?;
    }

    let summary = db.get_usage_summary(None, None, None)?;
    assert_eq!(summary.total_requests, 9);

    let logs = db.get_request_logs(&LogFilters::default(), 0, 10)?;
    let request_ids: Vec<&str> = logs
        .data
        .iter()
        .map(|log| log.request_id.as_str())
        .collect();
    assert_eq!(logs.total, 9);
    assert!(request_ids.contains(&"session-outside-window"));
    assert!(request_ids.contains(&"session-token-mismatch"));
    assert!(request_ids.contains(&"session-app-mismatch"));
    assert!(request_ids.contains(&"session-model-mismatch"));
    assert!(request_ids.contains(&"session-matches-error-proxy"));
    assert!(request_ids.contains(&"claude-session-cache-creation-mismatch"));

    Ok(())
}

#[test]
fn test_get_model_stats() -> Result<(), AppError> {
    let db = Database::memory()?;

    // 插入测试数据
    {
        let conn = lock_conn!(db.conn);
        conn.execute(
            "INSERT INTO proxy_request_logs (
                request_id, provider_id, app_type, model,
                input_tokens, output_tokens, total_cost_usd,
                latency_ms, status_code, created_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            params![
                "req1",
                "p1",
                "claude",
                "claude-3-sonnet",
                100,
                50,
                "0.01",
                100,
                200,
                1000
            ],
        )?;
    }

    let stats = db.get_model_stats(None, None, None)?;
    assert_eq!(stats.len(), 1);
    assert_eq!(stats[0].model, "claude-3-sonnet");
    assert_eq!(stats[0].request_count, 1);

    Ok(())
}

#[test]
fn test_get_provider_stats_with_time_filter() -> Result<(), AppError> {
    let db = Database::memory()?;

    {
        let conn = lock_conn!(db.conn);
        conn.execute(
            "INSERT INTO proxy_request_logs (
                request_id, provider_id, app_type, model,
                input_tokens, output_tokens, total_cost_usd,
                latency_ms, status_code, created_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            params!["old", "p1", "claude", "claude-3", 100, 50, "0.01", 100, 200, 1000],
        )?;
        conn.execute(
            "INSERT INTO proxy_request_logs (
                request_id, provider_id, app_type, model,
                input_tokens, output_tokens, total_cost_usd,
                latency_ms, status_code, created_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            params!["new", "p1", "claude", "claude-3", 200, 75, "0.02", 120, 200, 2000],
        )?;
    }

    let stats = db.get_provider_stats(Some(1500), Some(2500), Some("claude"))?;
    assert_eq!(stats.len(), 1);
    assert_eq!(stats[0].provider_id, "p1");
    assert_eq!(stats[0].request_count, 1);
    assert_eq!(stats[0].total_tokens, 275);

    Ok(())
}

#[test]
fn test_get_provider_stats_excludes_partial_rollup_boundary_days() -> Result<(), AppError> {
    let db = Database::memory()?;
    let start = local_ts(2024, 2, 1, 12, 0, 0);
    let end = local_ts(2024, 2, 3, 12, 0, 0);

    {
        let conn = lock_conn!(db.conn);
        conn.execute(
            "INSERT INTO usage_daily_rollups (
                date, app_type, provider_id, model,
                request_count, success_count, input_tokens, output_tokens,
                cache_read_tokens, cache_creation_tokens, total_cost_usd, avg_latency_ms
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            params![
                "2024-02-01",
                "claude",
                "p-rollup",
                "claude-3",
                5,
                5,
                500,
                250,
                0,
                0,
                "0.50",
                100
            ],
        )?;
        conn.execute(
            "INSERT INTO usage_daily_rollups (
                date, app_type, provider_id, model,
                request_count, success_count, input_tokens, output_tokens,
                cache_read_tokens, cache_creation_tokens, total_cost_usd, avg_latency_ms
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            params![
                "2024-02-02",
                "claude",
                "p-rollup",
                "claude-3",
                8,
                7,
                800,
                400,
                0,
                0,
                "0.80",
                120
            ],
        )?;
        conn.execute(
            "INSERT INTO usage_daily_rollups (
                date, app_type, provider_id, model,
                request_count, success_count, input_tokens, output_tokens,
                cache_read_tokens, cache_creation_tokens, total_cost_usd, avg_latency_ms
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            params![
                "2024-02-03",
                "claude",
                "p-rollup",
                "claude-3",
                12,
                11,
                1200,
                600,
                0,
                0,
                "1.20",
                140
            ],
        )?;
    }

    let stats = db.get_provider_stats(Some(start), Some(end), Some("claude"))?;
    assert_eq!(stats.len(), 1);
    assert_eq!(stats[0].provider_id, "p-rollup");
    assert_eq!(stats[0].request_count, 8);
    assert_eq!(stats[0].total_tokens, 1200);

    Ok(())
}

#[test]
fn test_get_daily_trends_respects_shorter_than_24_hours() -> Result<(), AppError> {
    let db = Database::memory()?;

    {
        let conn = lock_conn!(db.conn);
        conn.execute(
            "INSERT INTO proxy_request_logs (
                request_id, provider_id, app_type, model,
                input_tokens, output_tokens, total_cost_usd,
                latency_ms, status_code, created_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            params![
                "req-short",
                "p1",
                "claude",
                "claude-3",
                100,
                50,
                "0.01",
                100,
                200,
                10_800
            ],
        )?;
    }

    let stats = db.get_daily_trends(Some(0), Some(15 * 60 * 60), Some("claude"))?;
    assert_eq!(stats.len(), 15);
    assert_eq!(stats[3].request_count, 1);

    Ok(())
}

#[test]
fn test_get_daily_trends_groups_ranges_longer_than_24_hours_by_local_day(
) -> Result<(), AppError> {
    let db = Database::memory()?;
    let start = local_ts(2024, 3, 1, 12, 0, 0);
    let end = local_ts(2024, 3, 3, 12, 0, 0);

    {
        let conn = lock_conn!(db.conn);
        conn.execute(
            "INSERT INTO proxy_request_logs (
                request_id, provider_id, app_type, model,
                input_tokens, output_tokens, total_cost_usd,
                latency_ms, status_code, created_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            params![
                "day-1-detail",
                "p1",
                "claude",
                "claude-3",
                100,
                50,
                "0.01",
                100,
                200,
                local_ts(2024, 3, 1, 13, 0, 0)
            ],
        )?;
        conn.execute(
            "INSERT INTO proxy_request_logs (
                request_id, provider_id, app_type, model,
                input_tokens, output_tokens, total_cost_usd,
                latency_ms, status_code, created_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            params![
                "day-3-detail",
                "p1",
                "claude",
                "claude-3",
                200,
                75,
                "0.02",
                110,
                200,
                local_ts(2024, 3, 3, 10, 0, 0)
            ],
        )?;
        conn.execute(
            "INSERT INTO usage_daily_rollups (
                date, app_type, provider_id, model,
                request_count, success_count, input_tokens, output_tokens,
                cache_read_tokens, cache_creation_tokens, total_cost_usd, avg_latency_ms
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            params![
                "2024-03-02",
                "claude",
                "p1",
                "claude-3",
                4,
                4,
                400,
                200,
                0,
                0,
                "0.40",
                120
            ],
        )?;
    }

    let stats = db.get_daily_trends(Some(start), Some(end), Some("claude"))?;
    assert_eq!(stats.len(), 3);
    assert_eq!(stats[0].request_count, 1);
    assert_eq!(stats[0].total_tokens, 150);
    assert_eq!(stats[1].request_count, 4);
    assert_eq!(stats[1].total_tokens, 600);
    assert_eq!(stats[2].request_count, 1);
    assert_eq!(stats[2].total_tokens, 275);

    Ok(())
}

#[test]
fn test_get_model_stats_excludes_partial_rollup_boundary_days() -> Result<(), AppError> {
    let db = Database::memory()?;
    let start = local_ts(2024, 4, 1, 12, 0, 0);
    let end = local_ts(2024, 4, 3, 12, 0, 0);

    {
        let conn = lock_conn!(db.conn);
        conn.execute(
            "INSERT INTO usage_daily_rollups (
                date, app_type, provider_id, model,
                request_count, success_count, input_tokens, output_tokens,
                cache_read_tokens, cache_creation_tokens, total_cost_usd, avg_latency_ms
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            params![
                "2024-04-01",
                "claude",
                "p1",
                "claude-3-haiku",
                6,
                6,
                600,
                300,
                0,
                0,
                "0.60",
                100
            ],
        )?;
        conn.execute(
            "INSERT INTO usage_daily_rollups (
                date, app_type, provider_id, model,
                request_count, success_count, input_tokens, output_tokens,
                cache_read_tokens, cache_creation_tokens, total_cost_usd, avg_latency_ms
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            params![
                "2024-04-02",
                "claude",
                "p1",
                "claude-3-haiku",
                9,
                8,
                900,
                450,
                0,
                0,
                "0.90",
                110
            ],
        )?;
        conn.execute(
            "INSERT INTO usage_daily_rollups (
                date, app_type, provider_id, model,
                request_count, success_count, input_tokens, output_tokens,
                cache_read_tokens, cache_creation_tokens, total_cost_usd, avg_latency_ms
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            params![
                "2024-04-03",
                "claude",
                "p1",
                "claude-3-haiku",
                12,
                11,
                1200,
                600,
                0,
                0,
                "1.20",
                130
            ],
        )?;
    }

    let stats = db.get_model_stats(Some(start), Some(end), Some("claude"))?;
    assert_eq!(stats.len(), 1);
    assert_eq!(stats[0].model, "claude-3-haiku");
    assert_eq!(stats[0].request_count, 9);
    assert_eq!(stats[0].total_tokens, 1350);

    Ok(())
}

#[test]
fn test_strip_model_date_suffix_is_utf8_safe() {
    assert_eq!(
        strip_model_date_suffix("模型-2026-05-14").as_deref(),
        Some("模型")
    );
    assert_eq!(strip_model_date_suffix("abc🚀12345678"), None);
}

#[test]
fn test_prefix_pricing_does_not_match_short_base_model_to_variant() -> Result<(), AppError> {
    let db = Database::memory()?;
    let conn = lock_conn!(db.conn);

    conn.execute("DELETE FROM model_pricing WHERE model_id LIKE 'gpt-5%'", [])?;
    for (model_id, display_name) in [("gpt-5-mini", "GPT-5 Mini"), ("gpt-5-pro", "GPT-5 Pro")] {
        conn.execute(
            "INSERT INTO model_pricing (
                model_id, display_name, input_cost_per_million, output_cost_per_million,
                cache_read_cost_per_million, cache_creation_cost_per_million
            ) VALUES (?1, ?2, '1', '2', '0', '0')",
            params![model_id, display_name],
        )?;
    }

    let result = find_model_pricing_row(&conn, "gpt-5")?;
    assert!(
        result.is_none(),
        "缺少 gpt-5 基础定价时，不应前缀误匹配到 gpt-5-mini/gpt-5-pro"
    );

    Ok(())
}

#[test]
fn test_model_pricing_matching() -> Result<(), AppError> {
    let db = Database::memory()?;
    let conn = lock_conn!(db.conn);

    // 准备额外定价数据，覆盖前缀/后缀清洗场景
    conn.execute(
        "INSERT OR REPLACE INTO model_pricing (
            model_id, display_name, input_cost_per_million, output_cost_per_million,
            cache_read_cost_per_million, cache_creation_cost_per_million
        ) VALUES (?, ?, ?, ?, ?, ?)",
        params![
            "claude-haiku-4.5",
            "Claude Haiku 4.5",
            "1.0",
            "2.0",
            "0.0",
            "0.0"
        ],
    )?;

    // 测试精确匹配（seed_model_pricing 已预置 claude-sonnet-4-5-20250929）
    let result = find_model_pricing_row(&conn, "claude-sonnet-4-5-20250929")?;
    assert!(
        result.is_some(),
        "应该能精确匹配 claude-sonnet-4-5-20250929"
    );

    // 清洗：去除前缀和冒号后缀
    let result = find_model_pricing_row(&conn, "anthropic/claude-haiku-4.5")?;
    assert!(
        result.is_some(),
        "带前缀的模型 anthropic/claude-haiku-4.5 应能匹配到 claude-haiku-4.5"
    );
    let result = find_model_pricing_row(&conn, "moonshotai/kimi-k2-0905:exa")?;
    assert!(
        result.is_some(),
        "带前缀+冒号后缀的模型应清洗后匹配到 kimi-k2-0905"
    );

    // 清洗：@ 替换为 -（seed_model_pricing 已预置 gpt-5.2-codex-low）
    let result = find_model_pricing_row(&conn, "gpt-5.2-codex@low")?;
    assert!(
        result.is_some(),
        "带 @ 分隔符的模型 gpt-5.2-codex@low 应能匹配到 gpt-5.2-codex-low"
    );
    let result = find_model_pricing_row(&conn, "OpenAI/GPT-5.5@HIGH")?;
    assert!(
        result.is_some(),
        "大小写混合的 GPT-5.5 模型应能归一化匹配到 gpt-5.5-high"
    );
    let result = find_model_pricing_row(&conn, "OpenAI/GPT-5.5-2026-05-14")?;
    assert!(
        result.is_some(),
        "OpenAI 日期后缀模型应能回退到 gpt-5.5 基础定价"
    );
    let result = find_model_pricing_row(&conn, "google/gemini-3-pro-preview-20260514")?;
    assert!(
        result.is_some(),
        "Gemini 日期后缀模型应能回退到 gemini-3-pro-preview 基础定价"
    );

    // Claude Desktop route 短 ID：应通过前缀匹配到带日期的定价
    let result = find_model_pricing_row(&conn, "claude-haiku-4-5")?;
    assert!(
        result.is_some(),
        "Claude Desktop 短路由 claude-haiku-4-5 应能匹配到 claude-haiku-4-5-20251001"
    );

    // Claude Desktop 旧版/异常包装的非 Anthropic route：claude-gpt-5.5 → gpt-5.5
    let result = find_model_pricing_row(&conn, "claude-gpt-5.5")?;
    assert!(
        result.is_some(),
        "带 claude- 包装的非 Anthropic 模型应能剥离后匹配到真实模型定价"
    );

    // Bedrock/Vertex 常见形态：provider 前缀 + -vN 后缀 + :0 修饰
    let result =
        find_model_pricing_row(&conn, "global.anthropic.claude-haiku-4-5-20251001-v1:0")?;
    assert!(
        result.is_some(),
        "Bedrock/Vertex 风格 Claude 模型 ID 应能归一化到基础 Claude 模型定价"
    );

    // Reasoning effort 后缀：没有专门价格时回退到基础模型
    let result = find_model_pricing_row(&conn, "gpt-5.4@low")?;
    assert!(
        result.is_some(),
        "缺少专门 effort 价格时应回退到 gpt-5.4 基础模型定价"
    );

    // Kimi Code 是订阅/额度模型，不应伪装成公开按 token 计费模型
    let result = find_model_pricing_row(&conn, "kimi-for-coding")?;
    assert!(result.is_none(), "kimi-for-coding 没有固定 token 单价");

    // 测试不存在的模型
    let result = find_model_pricing_row(&conn, "unknown-model-123")?;
    assert!(result.is_none(), "不应该匹配不存在的模型");

    Ok(())
}
