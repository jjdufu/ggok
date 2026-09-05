use rusqlite::{Connection, OpenFlags};
use std::path::Path;

#[must_use]
pub fn search_session_ids(grok_home: &Path, q: &str) -> Option<Vec<String>> {
    let q = q.trim();
    if q.is_empty() {
        return None;
    }
    let db = grok_home.join("sessions").join("session_search.sqlite");
    if !db.is_file() {
        return None;
    }
    let conn = Connection::open_with_flags(
        &db,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_URI,
    )
    .ok()?;
    let fts = fts_phrase(q);
    let mut stmt = conn
        .prepare("SELECT session_id FROM session_docs_fts WHERE session_docs_fts MATCH ?1")
        .ok()?;
    let rows = stmt.query_map([&fts], |row| row.get::<_, String>(0)).ok()?;
    let mut ids = Vec::new();
    for id in rows.flatten() {
        ids.push(id);
    }
    Some(ids)
}

fn fts_phrase(q: &str) -> String {
    let cleaned: String = q
        .chars()
        .map(|c| match c {
            '"' | '*' | '(' | ')' | ':' | '^' => ' ',
            _ => c,
        })
        .collect();
    format!("\"{}\"", cleaned.trim())
}
