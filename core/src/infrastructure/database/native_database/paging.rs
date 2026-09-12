//! Bounded, deterministic table reads used by finalization and its
//! post-reopen verification.
//!
//! duckdb-rs materializes a query result before handing rows back, so reading
//! a whole archive table in one statement would size peak memory by the user's
//! history rather than by a fixed budget. Both sides of the copy therefore read
//! through a key window: the candidate by its `__hv_source_ordinal`, the
//! finalized table by its primary key. A byte preflight bounds the payload page
//! the same way the candidate builder bounds its own copy, because a row's size
//! is data-dependent while its row count is not.

use duckdb::Connection;
use duckdb::types::Value;

use super::NativeDatabaseError;
use super::cell::{
  COPY_BATCH_ROWS, Cell, MAX_BATCH_BYTES, NativeColumnKind, quote_identifier,
};

/// One column read back from a DuckDB table.
#[derive(Clone, Debug)]
pub(super) struct ReadColumn {
  pub(super) name: String,
  pub(super) kind: NativeColumnKind,
}

/// Reads a table in bounded pages ordered by `key_columns`, which must be a
/// unique, NOT NULL key so the window can never skip or repeat a row.
pub(super) struct PagedReader<'a> {
  connection: &'a Connection,
  table: String,
  key_columns: Vec<String>,
  columns: Vec<ReadColumn>,
  key_indices: Vec<usize>,
  cursor: Option<Vec<Cell>>,
  finished: bool,
}

impl<'a> PagedReader<'a> {
  pub(super) fn new(
    connection: &'a Connection,
    table: &str,
    columns: Vec<ReadColumn>,
    key_columns: Vec<String>,
  ) -> Result<Self, NativeDatabaseError> {
    let key_indices = key_columns
      .iter()
      .map(|key| {
        columns
          .iter()
          .position(|column| &column.name == key)
          .ok_or_else(|| NativeDatabaseError::SchemaMismatch {
            table: table.to_owned(),
            detail: format!("paging key {key} is not part of the read projection"),
          })
      })
      .collect::<Result<Vec<_>, _>>()?;
    if key_indices.is_empty() {
      return Err(NativeDatabaseError::SchemaMismatch {
        table: table.to_owned(),
        detail: "table has no unique key to read it back in bounded pages".to_owned(),
      });
    }
    Ok(Self {
      connection,
      table: table.to_owned(),
      key_columns,
      columns,
      key_indices,
      cursor: None,
      finished: false,
    })
  }

  /// The next page of decoded rows, or `None` once the table is exhausted.
  pub(super) fn next_page(
    &mut self,
  ) -> Result<Option<Vec<Vec<Cell>>>, NativeDatabaseError> {
    if self.finished {
      return Ok(None);
    }
    let Some(page_end) = self.preflight()? else {
      self.finished = true;
      return Ok(None);
    };

    let mut sql = format!(
      "SELECT {} FROM {}",
      self
        .columns
        .iter()
        .map(projection)
        .collect::<Vec<_>>()
        .join(", "),
      quote_identifier(&self.table)
    );
    let mut parameters = Vec::new();
    let mut conditions = Vec::new();
    if let Some(cursor) = self.cursor.clone() {
      conditions.push(format!(
        "({})",
        self.after_predicate(&mut parameters, &cursor)
      ));
    }
    conditions.push(format!(
      "NOT ({})",
      self.after_predicate(&mut parameters, &page_end)
    ));
    sql.push_str(" WHERE ");
    sql.push_str(&conditions.join(" AND "));
    sql.push_str(&format!(" ORDER BY {}", self.order_by()));

    let mut statement = self.connection.prepare(&sql).map_err(|error| {
      NativeDatabaseError::duckdb("prepare bounded table read", error)
    })?;
    let mut rows = statement
      .query(duckdb::params_from_iter(parameters))
      .map_err(|error| NativeDatabaseError::duckdb("read bounded table page", error))?;
    let mut page = Vec::new();
    while let Some(row) = rows
      .next()
      .map_err(|error| NativeDatabaseError::duckdb("read bounded table page", error))?
    {
      page.push(decode_row(row, &self.columns)?);
    }
    if page.is_empty() {
      return Err(NativeDatabaseError::Verification {
        message: format!(
          "{} changed between the preflight and the payload of one bounded page",
          self.table
        ),
      });
    }
    self.cursor = Some(page_end);
    Ok(Some(page))
  }

  /// The last key of the next page: at most [`COPY_BATCH_ROWS`] rows, and no
  /// more than [`MAX_BATCH_BYTES`] of payload after the first row.
  fn preflight(&self) -> Result<Option<Vec<Cell>>, NativeDatabaseError> {
    let keys = self
      .key_indices
      .iter()
      .map(|index| projection(&self.columns[*index]))
      .collect::<Vec<_>>()
      .join(", ");
    let mut sql = format!(
      "SELECT {keys}, {} FROM {}",
      self.row_bytes_expression(),
      quote_identifier(&self.table)
    );
    let mut parameters = Vec::new();
    if let Some(cursor) = self.cursor.clone() {
      sql.push_str(" WHERE ");
      sql.push_str(&self.after_predicate(&mut parameters, &cursor));
    }
    sql.push_str(&format!(
      " ORDER BY {} LIMIT {COPY_BATCH_ROWS}",
      self.order_by()
    ));

    let key_columns = self
      .key_indices
      .iter()
      .map(|index| self.columns[*index].clone())
      .collect::<Vec<_>>();
    let mut statement = self.connection.prepare(&sql).map_err(|error| {
      NativeDatabaseError::duckdb("prepare bounded page preflight", error)
    })?;
    let mut rows = statement
      .query(duckdb::params_from_iter(parameters))
      .map_err(|error| {
        NativeDatabaseError::duckdb("read bounded page preflight", error)
      })?;
    let mut accepted: Option<Vec<Cell>> = None;
    let mut bytes = 0_u64;
    while let Some(row) = rows.next().map_err(|error| {
      NativeDatabaseError::duckdb("read bounded page preflight", error)
    })? {
      let key = decode_row(row, &key_columns)?;
      let row_bytes: i64 = row.get(key_columns.len()).map_err(|error| {
        NativeDatabaseError::duckdb("decode bounded page row size", error)
      })?;
      let row_bytes = u64::try_from(row_bytes).unwrap_or(u64::MAX);
      if accepted.is_some() && bytes.saturating_add(row_bytes) > MAX_BATCH_BYTES {
        break;
      }
      bytes = bytes.saturating_add(row_bytes);
      accepted = Some(key);
    }
    Ok(accepted)
  }

  fn order_by(&self) -> String {
    self
      .key_columns
      .iter()
      .map(|key| quote_identifier(key))
      .collect::<Vec<_>>()
      .join(", ")
  }

  /// `key > cursor` written out lexicographically, because DuckDB row
  /// comparison syntax is not portable across the identifier quoting used here.
  fn after_predicate(&self, parameters: &mut Vec<Value>, cursor: &[Cell]) -> String {
    fn build(
      keys: &[String],
      cursor: &[Cell],
      parameters: &mut Vec<Value>,
      index: usize,
    ) -> String {
      let column = quote_identifier(&keys[index]);
      let value = key_value(&cursor[index]);
      if index + 1 == keys.len() {
        parameters.push(value);
        return format!("{column} > ?");
      }
      parameters.push(value.clone());
      parameters.push(value);
      let rest = build(keys, cursor, parameters, index + 1);
      format!("{column} > ? OR ({column} = ? AND ({rest}))")
    }

    build(&self.key_columns, cursor, parameters, 0)
  }

  fn row_bytes_expression(&self) -> String {
    self
      .columns
      .iter()
      .map(|column| {
        let name = quote_identifier(&column.name);
        match column.kind {
          NativeColumnKind::BigInt
          | NativeColumnKind::UBigInt
          | NativeColumnKind::Double => {
            format!("CASE WHEN {name} IS NULL THEN 0 ELSE 8 END")
          }
          NativeColumnKind::TaggedNumeric => {
            format!("CASE WHEN {name} IS NULL THEN 0 ELSE 9 END")
          }
          NativeColumnKind::Varchar => {
            format!(
              "CASE WHEN {name} IS NULL THEN 0 ELSE octet_length(encode({name})) END"
            )
          }
          NativeColumnKind::Blob => {
            format!("CASE WHEN {name} IS NULL THEN 0 ELSE octet_length({name}) END")
          }
        }
      })
      .collect::<Vec<_>>()
      .join(" + ")
  }
}

fn key_value(cell: &Cell) -> Value {
  match cell {
    Cell::Integer(value) => Value::BigInt(*value),
    Cell::Real(bits) => Value::Double(f64::from_bits(*bits)),
    Cell::Text(value) => Value::Text(value.clone()),
    Cell::Blob(value) => Value::Blob(value.clone()),
    // Unreachable for a NOT NULL key; binding NULL makes the window empty
    // rather than silently wrapping around.
    Cell::Null => Value::Null,
  }
}

/// A column's projection, expanding a tagged union into its tag and members so
/// the integer/real distinction survives decoding.
pub(super) fn projection(column: &ReadColumn) -> String {
  let name = quote_identifier(&column.name);
  match column.kind {
    NativeColumnKind::TaggedNumeric => format!(
      "CAST(union_tag({name}) AS VARCHAR), union_extract({name}, 'i'), union_extract({name}, 'r')"
    ),
    _ => name,
  }
}

pub(super) fn decode_row(
  row: &duckdb::Row<'_>,
  columns: &[ReadColumn],
) -> Result<Vec<Cell>, NativeDatabaseError> {
  let mut cells = Vec::with_capacity(columns.len());
  let mut index = 0_usize;
  for column in columns {
    match column.kind {
      NativeColumnKind::TaggedNumeric => {
        let tag: Option<String> = row.get(index).map_err(decode_error)?;
        let integer: Option<i64> = row.get(index + 1).map_err(decode_error)?;
        let real: Option<f64> = row.get(index + 2).map_err(decode_error)?;
        cells.push(match (tag.as_deref(), integer, real) {
          (None, None, None) => Cell::Null,
          (Some("i"), Some(value), None) => Cell::Integer(value),
          (Some("r"), None, Some(value)) => Cell::Real(value.to_bits()),
          _ => {
            return Err(NativeDatabaseError::Verification {
              message: format!(
                "tagged numeric column {} has inconsistent tag and member payloads",
                column.name
              ),
            });
          }
        });
        index += 3;
      }
      NativeColumnKind::BigInt => {
        let value: Option<i64> = row.get(index).map_err(decode_error)?;
        cells.push(value.map_or(Cell::Null, Cell::Integer));
        index += 1;
      }
      NativeColumnKind::UBigInt => {
        let value: Option<u64> = row.get(index).map_err(decode_error)?;
        let value = value
          .map(|value| {
            i64::try_from(value).map_err(|_| NativeDatabaseError::Verification {
              message: format!(
                "unsigned column {} holds {value}, which does not fit a signed 64-bit value",
                column.name
              ),
            })
          })
          .transpose()?;
        cells.push(value.map_or(Cell::Null, Cell::Integer));
        index += 1;
      }
      NativeColumnKind::Double => {
        let value: Option<f64> = row.get(index).map_err(decode_error)?;
        cells.push(value.map_or(Cell::Null, |value| Cell::Real(value.to_bits())));
        index += 1;
      }
      NativeColumnKind::Varchar => {
        let value: Option<String> = row.get(index).map_err(decode_error)?;
        cells.push(value.map_or(Cell::Null, Cell::Text));
        index += 1;
      }
      NativeColumnKind::Blob => {
        let value: Option<Vec<u8>> = row.get(index).map_err(decode_error)?;
        cells.push(value.map_or(Cell::Null, Cell::Blob));
        index += 1;
      }
    }
  }
  Ok(cells)
}

fn decode_error(error: duckdb::Error) -> NativeDatabaseError {
  NativeDatabaseError::duckdb("decode a native table cell", error)
}
