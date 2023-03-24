use core::num::ParseIntError;
use sqlx::{mysql::MySqlConnectOptions, ConnectOptions};
use sqlx::{FromRow, MySqlConnection};
use std::error::Error;

#[derive(Default)]
pub struct Tables {
    pub tables: Vec<Table>,
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

        for table in qr_tables {
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
#[derive(Default, Clone)]
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
