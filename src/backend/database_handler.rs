use crate::backend::csv_handler::DataEntry;
use core::num::ParseIntError;
use sqlx::mysql::MySqlQueryResult;
use sqlx::{mysql::MySqlConnectOptions, ConnectOptions};
use sqlx::{Executor, FromRow, MySqlConnection};
use std::error::Error;
#[derive(Default)]
pub struct Tables {
    pub tables: Vec<Table>,
    pub current_working_table: Option<usize>,
    pub is_connection_verified: bool,
}

#[derive(Default, Clone)]
pub struct Table {
    pub name: String,
    pub fields: Option<Vec<TableField>>,
}
#[derive(Default, Clone)]
pub struct TableField {
    pub description: FieldDescription,
    pub mapped_to_col: Option<usize>,
}

#[derive(sqlx::FromRow, Default, Clone)]
#[sqlx(rename_all = "PascalCase")]
pub struct FieldDescription {
    pub field: String,
    #[sqlx(rename = "type")]
    pub field_type: String,
    pub null: String,
    pub key: String,
    pub default: Option<String>,
    pub extra: String,
}

impl Tables {
    pub async fn query_for_tables(
        &mut self,
        connection: &mut MySqlConnection,
    ) -> Result<(), Box<dyn Error + Send>> {
        let qr_tables: Vec<QRTables> = sqlx::query_as("SHOW TABLES")
            .fetch_all(connection)
            .await
            .unwrap();
        let mut table_i = 0;
        for table in qr_tables {
            println!("  >Found table:{}. {}", &table_i, &table.tables_in_quotes);
            table_i += 1;
            self.tables.push({
                Table {
                    name: table.tables_in_quotes,
                    fields: None,
                }
            })
        }

        Ok(())
    }
}

impl Table {
    pub async fn transaction_commit(connection: &mut MySqlConnection) -> QueryResult {
        match connection.execute("COMMIT").await {
            Ok(res) => {
                return QueryResult {
                    query: "COMMIT".to_owned(),
                    result: Ok(res),
                }
            }
            Err(e) => {
                return QueryResult {
                    query: "COMMIT".to_owned(),
                    result: Err(Box::new(e)),
                }
            }
        }
    }
    pub async fn transaction_rollback(connection: &mut MySqlConnection) -> QueryResult {
        match connection.execute("ROLLBACK").await {
            Ok(res) => {
                return QueryResult {
                    query: "ROLLBACK".to_owned(),
                    result: Ok(res),
                }
            }
            Err(e) => {
                return QueryResult {
                    query: "ROLLBACK".to_owned(),
                    result: Err(Box::new(e)),
                }
            }
        }
    }
    pub async fn start_transaction(connection: &mut MySqlConnection) -> QueryResult {
        match connection.execute("SET autocommit = OFF").await {
            Ok(_) => match connection.execute("BEGIN").await {
                Ok(res) => {
                    return QueryResult {
                        query: "BEGIN".to_owned(),
                        result: Ok(res),
                    }
                }
                Err(e) => {
                    return QueryResult {
                        query: "BEGIN".to_owned(),
                        result: Err(Box::new(e)),
                    }
                }
            },
            Err(e) => {
                return QueryResult {
                    query: "SET autocommit = OFF".to_owned(),
                    result: Err(Box::new(e)),
                }
            }
        }
    }
    pub async fn truncate_table(&self, connection: &mut MySqlConnection) -> QueryResult {
        match connection
            .execute(format!("truncate {}", self.name).as_str())
            .await
        {
            Ok(res) => {
                return QueryResult {
                    query: format!("truncate {}", self.name),
                    result: Ok(res),
                }
            }
            Err(e) => {
                return QueryResult {
                    query: format!("truncate {}", self.name),
                    result: Err(Box::new(e)),
                }
            }
        }
    }
    pub async fn query_for_table_entries(
        &mut self,
        connection: &mut MySqlConnection,
    ) -> QueryResult {
        let res = sqlx::query(format!("select * from {}", self.name).as_str())
            .fetch_all(connection)
            .await;
        match res {
            Ok(rows) => {
                for row in rows {
                    todo!()
                }
            }
            Err(e) => todo!(),
        }
        todo!();
    }
    pub async fn insert_into_table(
        &self,
        connection: &mut MySqlConnection,
        csv_row: Vec<&DataEntry>,
    ) -> QueryResult {
        /* Field_name, data_name */
        let mut fields = csv_row
            .iter()
            .fold(("".to_owned(), "".to_owned()), |row, next_row| {
                (
                    row.0
                        + next_row
                            .curr_field_description
                            .as_ref()
                            .unwrap()
                            .field
                            .as_str()
                        + ", ",
                    row.1 + "\'" + next_row.data.as_str() + "\', ",
                )
            });
        /* remove last ", " from both */
        fields.0.pop();
        fields.0.pop();
        fields.1.pop();
        fields.1.pop();
        let query = format!(
            "INSERT INTO {}({}) VALUES({})",
            self.name, fields.0, fields.1
        );
        match sqlx::query(&query).execute(connection).await {
            Ok(res) => {
                return QueryResult {
                    query,
                    result: Ok(res),
                }
            }
            Err(e) => {
                return QueryResult {
                    query,
                    result: Err(Box::new(e)),
                }
            }
        }
    }

    pub async fn describe_table(&mut self, connection: &mut MySqlConnection) {
        let qr_description: Vec<FieldDescription> =
            sqlx::query_as(format!("DESCRIBE {}", self.name).as_str())
                .fetch_all(connection)
                .await
                .unwrap();

        if self.fields.is_none() {
            self.fields = Some(Vec::new());
        }
        for field in qr_description {
            if let Some(fields) = &mut self.fields {
                println!(
                    "Discovering field '{}' to table '{}'",
                    field.field, self.name
                );
                fields.push(TableField {
                    description: field,
                    mapped_to_col: None,
                })
            }
        }
    }
}

#[derive(FromRow)]
struct QRTables {
    #[sqlx(rename = "Tables_in_quotes")]
    tables_in_quotes: String,
}
#[derive(Debug)]
pub struct QueryResult {
    pub query: String,
    pub result: Result<MySqlQueryResult, Box<dyn Error + Send>>,
}

#[derive(Default, Clone, Debug)]
pub struct DBLoginData {
    pub user_name: String,
    pub database: String,
    pub host: String,
    pub port: String,
    pub password: String,
    pub should_remember: bool,
    pub is_verified: bool,
}
impl DBLoginData {
    pub async fn validate_creditentials(
        &mut self,
    ) -> Result<MySqlConnection, Box<dyn Error + Send>> {
        let conn: MySqlConnection;

        let parsed_port: u16;
        let parse_res: Result<u16, ParseIntError> = self.port.parse();
        match parse_res {
            Ok(res) => parsed_port = res,
            Err(e) => return Err(Box::new(e)),
        }

        match MySqlConnectOptions::new()
            .host(self.host.as_str())
            .port(parsed_port)
            .username(self.user_name.as_str())
            .password(self.password.as_str())
            .database(self.database.as_str())
            .connect()
            .await
        {
            Ok(val) => {
                conn = val;
                self.is_verified = true;
            }
            Err(e) => {
                self.is_verified = false;
                return Err(Box::new(e));
            }
        }
        Ok(conn)
    }
}
