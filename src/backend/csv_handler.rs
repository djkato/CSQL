use std::{error::Error, sync::Arc};

use super::database_handler::FieldDescription;

#[derive(Clone)]
pub struct ImportedData {
    pub data: grid::Grid<DataEntry>,
    pub path: String,
    pub are_headers: bool,
}
#[derive(Clone, Default)]
pub struct DataEntry {
    pub data: String,
    pub curr_field_description: Option<FieldDescription>,
    pub is_parsed: Option<Result<(), Arc<dyn Error + Send + Sync>>>,
}
impl ImportedData {
    pub fn load_csv(&mut self) -> Result<(), Box<dyn Error + Send>> {
        match csv::Reader::from_path(self.path.as_str()) {
            Ok(mut rdr) => {
                /* If there are headers, add to first row, else first row is empty */

                if let Ok(headers) = rdr.headers() {
                    self.data = grid::Grid::new(0, 0);
                    let headers_vec: Vec<String> = headers.iter().map(|x| x.to_owned()).collect();
                    let mut data: Vec<DataEntry> = Vec::new();
                    for header in headers_vec.into_iter() {
                        data.push(DataEntry {
                            data: header,
                            ..Default::default()
                        });
                    }
                    self.data.push_row(data);
                    self.are_headers = true;
                } else {
                    self.data.push_row(Vec::new());
                    self.are_headers = false;
                }

                for rec_res in rdr.records() {
                    if let Ok(rec) = rec_res {
                        let records: Vec<String> = rec.iter().map(|x| x.to_owned()).collect();
                        let mut data: Vec<DataEntry> = Vec::new();
                        for entry in records.into_iter() {
                            data.push(DataEntry {
                                data: entry,
                                ..Default::default()
                            });
                        }
                        self.data.push_row(data);
                    }
                }
            }
            Err(e) => {
                return Err(Box::new(e));
            }
        }
        Ok(())
    }
}

impl Default for ImportedData {
    fn default() -> Self {
        ImportedData {
            data: grid::Grid::new(0, 0),
            path: String::new(),
            are_headers: false,
        }
    }
}
