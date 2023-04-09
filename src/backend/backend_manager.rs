use super::csv_handler::{DataEntry, ImportedData};
use super::database_handler::{DBLoginData, QueryResult, Table, TableField, Tables};
use super::parser::parse;
use sqlx::mysql::MySqlQueryResult;
use sqlx::MySqlConnection;
use std::error::Error;
use std::sync::Arc;
use tokio::sync::mpsc::{Receiver, Sender};
use tokio::sync::{oneshot, Mutex, MutexGuard};
pub struct BackendManger {
    pub db_login_data: DBLoginData,
    pub imported_data: ImportedData,
    pub csv_data: Arc<Mutex<ImportedData>>,
    pub db_connection: Option<MySqlConnection>,
    pub receiver: Receiver<Communication>,
    pub db_table_data: Arc<Mutex<Tables>>,
}

impl BackendManger {
    pub async fn listen(&mut self) {
        if let Some(command) = self.receiver.recv().await {
            match command {
                Communication::ValidateCreditentials(db_login_data, sender) => {
                    self.db_login_data = db_login_data;

                    let result = self.db_login_data.validate_creditentials().await;
                    match result {
                        Ok(connection) => {
                            self.db_connection = Some(connection);
                            sender.send(Ok(())).unwrap_or_else(|_e| {
                                println!("failed to send ValidateCreditentials err")
                            });
                            let mut db_table_data = self.db_table_data.lock().await;
                            db_table_data
                                .query_for_tables(&mut self.db_connection.as_mut().unwrap())
                                .await
                                .unwrap();
                        }
                        Err(e) => {
                            sender.send(Err(e)).unwrap_or_else(|_e| {
                                println!("failed to send ValidateCreditentials err")
                            });
                        }
                    }
                }
                Communication::AreCreditentialsValidated(sender) => sender
                    .send(self.db_login_data.is_verified)
                    .unwrap_or_else(|e| {
                        println!("Server - Failed answering if credit. validated");
                    }),
                Communication::LoadImportFilePath(path, sender) => {
                    self.imported_data.path = path;
                    match self.imported_data.load_csv() {
                        Ok(_) => {
                            let mut data = self.csv_data.lock().await;

                            data.are_headers = self.imported_data.are_headers.clone();
                            data.data = self.imported_data.data.clone();
                            data.path = self.imported_data.path.clone();

                            drop(data);
                            sender.send(true).unwrap_or_else(|err| {
                                println!("Server - failed to send loaded file callback, {}", err)
                            })
                        }
                        Err(e) => {
                            println!("{}", e);
                            sender.send(false).unwrap_or_else(|e| {
                                println!("Server - Failed answering if imported{}", e)
                            })
                        }
                    }
                }
                Communication::GetTableDescription(table_index, sender) => {
                    let mut db_table_data = self.db_table_data.lock().await;
                    let mut csv_data = self.csv_data.lock().await;
                    db_table_data
                        .tables
                        .get_mut(table_index)
                        .unwrap()
                        .describe_table(&mut self.db_connection.as_mut().unwrap())
                        .await;
                    if csv_data.are_headers {
                        match try_match_headers_to_fields(
                            &mut db_table_data,
                            table_index,
                            &csv_data.data.iter_row(0),
                        ) {
                            Ok(_) => {
                                for (col_index, field) in db_table_data
                                    .tables
                                    .get_mut(table_index)
                                    .unwrap()
                                    .fields
                                    .as_mut()
                                    .unwrap()
                                    .iter_mut()
                                    .enumerate()
                                {
                                    println!(
                                        "      > automapping field \"{}\" to col \"{:?}\"",
                                        &field.description.field, field.mapped_to_col
                                    );
                                    if let Some(col_index) = field.mapped_to_col {
                                        for cell in csv_data.data.iter_col_mut(col_index) {
                                            cell.curr_field_description =
                                                Some(field.description.clone());
                                            parse(cell);
                                        }
                                    }
                                    if is_whole_col_parsed(&mut csv_data, col_index) {
                                        println!("col \"{}\" is parsed whole!", &col_index);
                                        csv_data.parsed_cols.push(col_index);
                                    }
                                    if is_whole_table_parsed(&csv_data) {
                                        csv_data.is_parsed = true;
                                    }
                                }
                                sender.send(true).unwrap_or_else(|e| {
                                    println!(
                                        "Server - failed to respond to getTableDescription, {}",
                                        e
                                    )
                                });
                            }
                            Err(_) => sender.send(false).unwrap_or_else(|e| {
                                println!("Server - failed to respond to getTableDescription, {}", e)
                            }),
                        }
                    } else {
                        sender.send(false).unwrap_or_else(|e| {
                            println!("Server - failed to respond to getTableDescription, {}", e)
                        });
                    }
                }
                Communication::TryParseCol(col_index) => {
                    let mut csv_data = self.csv_data.lock().await;
                    for cell in csv_data.data.iter_col_mut(col_index) {
                        if cell.curr_field_description.is_some() {
                            parse(cell);
                        }
                    }
                    if is_whole_col_parsed(&mut csv_data, col_index) {
                        println!("col \"{}\" is parsed whole!", &col_index);
                        csv_data.parsed_cols.push(col_index - 1);
                    }
                    if is_whole_table_parsed(&csv_data) {
                        csv_data.is_parsed = true;
                    }
                }
                Communication::StartInserting(table_index, sender, oneshot_sender) => {
                    let csv_data = self.csv_data.lock().await;
                    let db_table_data = self.db_table_data.lock().await;
                    let table = db_table_data.tables.get(table_index).unwrap();
                    let trans_start =
                        Table::start_transaction(self.db_connection.as_mut().unwrap()).await;

                    sender
                        .send(trans_start)
                        .await
                        .unwrap_or_else(|_| println!("failed to send Transaction Start"));

                    let start_i: usize = csv_data.are_headers.into();

                    for i in start_i..csv_data.data.rows() {
                        let row: Vec<&DataEntry> = csv_data.data[i].iter().collect();
                        let res = table
                            .insert_into_table(self.db_connection.as_mut().unwrap(), row)
                            .await;
                        println!(
                            "      | Query: {}\n       > Result: {:?}",
                            res.query, res.result
                        );
                        sender
                            .send(res)
                            .await
                            .unwrap_or_else(|_| println!("failed to send insert into table"));
                    }
                    oneshot_sender
                        .send(true)
                        .unwrap_or_else(|_| println!("Failed to send end of insertin transaction"));
                }
                Communication::TryCommit(sender) => {
                    sender
                        .send(Table::transaction_commit(self.db_connection.as_mut().unwrap()).await)
                        .unwrap_or_else(|_| println!("Failed to respond to TryCommit"));
                }
                Communication::TryRollBack(sender) => {
                    sender
                        .send(
                            Table::transaction_rollback(self.db_connection.as_mut().unwrap()).await,
                        )
                        .unwrap_or_else(|_| println!("Failed to respond to TryRollBack"));
                }
            }
        }
    }
}
pub fn is_whole_table_parsed(csv_data: &MutexGuard<ImportedData>) -> bool {
    if csv_data.data.cols() == csv_data.parsed_cols.len() {
        true
    } else {
        false
    }
}
pub fn is_whole_col_parsed(csv_data: &mut MutexGuard<ImportedData>, col_index: usize) -> bool {
    let mut csv_iter = csv_data.data.iter_col(col_index);
    if csv_data.are_headers {
        csv_iter.next();
    }
    csv_iter.all(|cel| {
        if let Some(parse) = &cel.is_parsed {
            match parse {
                Ok(_) => return true,
                Err(_) => {
                    println!(
                        "Cel \"{}\" in Col \"{}\" isnt parsed :(",
                        cel.data, col_index
                    );
                    return false;
                }
            }
        } else {
            println!(
                "Cel \"{}\" in Col \"{}\" isnt parsed :(",
                cel.data, col_index
            );
            false
        }
    })
}
pub fn try_match_headers_to_fields(
    tables: &mut MutexGuard<Tables>,
    table_index: usize,
    csv_headers: &std::slice::Iter<'_, DataEntry>,
) -> Result<(), ()> {
    let db_fields = tables
        .tables
        .get_mut(table_index)
        .unwrap()
        .fields
        .as_mut()
        .unwrap();
    let mut has_matched_some = false;
    for field in db_fields {
        for (i, header) in csv_headers.clone().enumerate() {
            if &field.description.field == &header.data {
                field.mapped_to_col = Some(i);
                has_matched_some = true;
            }
        }
    }
    match has_matched_some {
        true => return Ok(()),
        false => return Err(()),
    }
}

pub enum Communication {
    ValidateCreditentials(
        DBLoginData,
        oneshot::Sender<Result<(), Box<dyn std::error::Error + Send>>>,
    ),
    AreCreditentialsValidated(oneshot::Sender<bool>),
    LoadImportFilePath(String, oneshot::Sender<bool>),
    GetTableDescription(usize, oneshot::Sender<bool>),
    TryParseCol(usize),
    StartInserting(usize, Sender<QueryResult>, oneshot::Sender<bool>),
    TryCommit(oneshot::Sender<QueryResult>),
    TryRollBack(oneshot::Sender<QueryResult>),
}
