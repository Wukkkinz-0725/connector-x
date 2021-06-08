use crate::errors::ConnectorAgentPythonError;
use connectorx::partition::{pg_get_partition_range, pg_single_col_partition_query};
use dict_derive::FromPyObject;
use fehler::throw;
use pyo3::prelude::*;
use pyo3::{
    exceptions::{PyNotImplementedError, PyValueError},
    PyResult,
};

#[derive(FromPyObject)]
pub struct PartitionQuery {
    query: String,
    column: String,
    min: Option<i64>,
    max: Option<i64>,
    num: usize,
}

impl PartitionQuery {
    pub fn new(query: &str, column: &str, min: Option<i64>, max: Option<i64>, num: usize) -> Self {
        Self {
            query: query.into(),
            column: column.into(),
            min,
            max,
            num,
        }
    }
}

pub fn read_sql<'a>(
    py: Python<'a>,
    conn: &str,
    return_type: &str,
    protocol: Option<&str>,
    queries: Option<Vec<String>>,
    partition_query: Option<PartitionQuery>,
) -> PyResult<&'a PyAny> {
    let queries = match (queries, partition_query) {
        (Some(queries), None) => queries,
        (
            None,
            Some(PartitionQuery {
                query,
                column: col,
                min,
                max,
                num,
            }),
        ) => {
            let mut queries = vec![];
            let num = num as i64;

            let (min, max) = match (min, max) {
                (None, None) => pg_get_partition_range(conn, &query, &col)
                    .map_err(ConnectorAgentPythonError::ConnectorAgentError)?,
                (Some(min), Some(max)) => (min, max),
                _ => throw!(PyValueError::new_err(
                    "partition_query range can not be partially specified",
                )),
            };

            let partition_size = (max - min + 1) / num;

            for i in 0..num {
                let lower = min + i * partition_size;
                let upper = match i == num - 1 {
                    true => max + 1,
                    false => min + (i + 1) * partition_size,
                };
                let partition_query = pg_single_col_partition_query(&query, &col, lower, upper)
                    .map_err(ConnectorAgentPythonError::ConnectorAgentError)?;
                queries.push(partition_query);
            }
            queries
        }
        (Some(_), Some(_)) => throw!(PyValueError::new_err(
            "partition_query and queries cannot be both specified",
        )),
        (None, None) => throw!(PyValueError::new_err(
            "partition_query and queries cannot be both None",
        )),
    };

    let queries: Vec<_> = queries.iter().map(|s| s.as_str()).collect();
    match return_type {
        "pandas" => Ok(crate::pandas::write_pandas(
            py,
            conn,
            &queries,
            protocol.unwrap_or("binary"),
        )?),
        "arrow" => Err(PyNotImplementedError::new_err(
            "arrow return type is not implemented",
        )),
        _ => Err(PyValueError::new_err(format!(
            "return type should be 'pandas' or 'arrow', got '{}'",
            return_type
        ))),
    }
}

pub fn read_mysql<'a>(
    py: Python<'a>,
    conn: &str,
    return_type: &str,
    protocol: Option<&str>,
    queries: Option<Vec<String>>,
    partition_query: Option<PartitionQuery>,
) -> PyResult<&'a PyAny> {

    // let queries = match (queries, partition_query) {
    //     (Some(queries), None) => queries,
    //     (
    //         None,
    //         Some(PartitionQuery {
    //             query,
    //             column: col,
    //             min,
    //             max,
    //             num,
    //         }),
    //     ) => {
    //         let mut queries = vec![];
    //         let num = num as i64;
    //
    //         let (min, max) = match (min, max) {
    //             (None, None) => pg_get_partition_range(conn, &query, &col)
    //                 .map_err(ConnectorAgentPythonError::ConnectorAgentError)?,
    //             (Some(min), Some(max)) => (min, max),
    //             _ => throw!(PyValueError::new_err(
    //                 "partition_query range can not be partially specified",
    //             )),
    //         };
    //
    //         let partition_size = (max - min + 1) / num;
    //
    //         for i in 0..num {
    //             let lower = min + i * partition_size;
    //             let upper = match i == num - 1 {
    //                 true => max + 1,
    //                 false => min + (i + 1) * partition_size,
    //             };
    //             let partition_query = pg_single_col_partition_query(&query, &col, lower, upper)
    //                 .map_err(ConnectorAgentPythonError::ConnectorAgentError)?;
    //             queries.push(partition_query);
    //         }
    //         queries
    //     }
    //     (Some(_), Some(_)) => throw!(PyValueError::new_err(
    //         "partition_query and queries cannot be both specified",
    //     )),
    //     (None, None) => throw!(PyValueError::new_err(
    //         "partition_query and queries cannot be both None",
    //     )),
    // };
    //
    // let queries: Vec<_> = queries.iter().map(|s| s.as_str()).collect();
    // match return_type {
    //     "pandas" => Ok(crate::pandas::write_pandas(
    //         py,
    //         conn,
    //         &queries,
    //         protocol.unwrap_or("binary"),
    //     )?),
    //     "arrow" => Err(PyNotImplementedError::new_err(
    //         "arrow return type is not implemented",
    //     )),
    //     _ => Err(PyValueError::new_err(format!(
    //         "return type should be 'pandas' or 'arrow', got '{}'",
    //         return_type
    //     ))),
    // }
}
